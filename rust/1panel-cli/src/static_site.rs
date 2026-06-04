use crate::onepanel::OnePanelConfig;
use anyhow::{Result, anyhow};
use reqwest::{Client, multipart};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Website {
    pub id: Option<u64>,
    pub domain: String,
    pub alias: Option<String>,
    pub group_id: Option<u64>,
    pub site_path: Option<String>,
    pub status: Option<String>,
    pub domains: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DeployStaticResult {
    pub ok: bool,
    pub action: &'static str,
    pub domain: String,
    pub website_id: Option<u64>,
    pub created: bool,
    pub group_id: Option<u64>,
    pub source_path: String,
    pub uploaded: usize,
    pub failed: usize,
    pub total: usize,
    pub site_path: Option<String>,
    pub url: String,
}

fn clean_host(host: &str) -> String {
    host.trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string()
}

fn auth_headers(api_key: &str) -> (String, String) {
    let timestamp = chrono::Utc::now().timestamp();
    let token_raw = format!("1panel{}{}", api_key.trim(), timestamp);
    let token_digest = md5::compute(token_raw.as_bytes());
    (format!("{:x}", token_digest), timestamp.to_string())
}

fn v2_base(cfg: &OnePanelConfig) -> String {
    format!(
        "{}://{}:{}/api/v2",
        cfg.scheme,
        clean_host(&cfg.host),
        cfg.port
    )
}

fn client(cfg: &OnePanelConfig) -> Client {
    Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(cfg.insecure_skip_tls_verify)
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn unwrap_data(json: &Value) -> Value {
    json.get("data").cloned().unwrap_or_else(|| json.clone())
}

fn check_api(json: Value, action: &str) -> Result<Value> {
    if let Some(code) = json.get("code").and_then(|c| c.as_i64()) {
        if code != 200 {
            let msg = json
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("{} failed (API {}): {}", action, code, msg));
        }
    }

    Ok(json)
}

fn normalize_website(v: &Value) -> Website {
    let domains = v
        .get("domains")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if let Some(s) = item.as_str() {
                        Some(s.to_string())
                    } else {
                        item.get("domain")
                            .and_then(|x| x.as_str())
                            .map(ToString::to_string)
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let primary = v
        .get("primaryDomain")
        .and_then(|s| s.as_str())
        .or_else(|| v.get("domain").and_then(|s| s.as_str()))
        .or_else(|| v.get("alias").and_then(|s| s.as_str()))
        .or_else(|| domains.first().map(String::as_str))
        .unwrap_or_default()
        .to_string();

    Website {
        id: v.get("id").and_then(|x| x.as_u64()),
        domain: primary,
        alias: v
            .get("alias")
            .and_then(|x| x.as_str())
            .map(ToString::to_string),
        group_id: v
            .get("webSiteGroupId")
            .and_then(|x| x.as_u64())
            .or_else(|| v.get("webSiteGroupID").and_then(|x| x.as_u64())),
        site_path: v
            .get("sitePath")
            .and_then(|x| x.as_str())
            .map(ToString::to_string),
        status: v.get("status").map(|s| s.to_string()),
        domains,
    }
}

pub async fn list_websites(cfg: &OnePanelConfig) -> Result<Vec<Website>> {
    let api = v2_base(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let c = client(cfg);

    let list_url = format!("{}/websites/list", api);
    let res = c
        .get(&list_url)
        .header("1Panel-Token", &token)
        .header("1Panel-Timestamp", &ts)
        .header("Accept-Language", "en")
        .send()
        .await;

    let json = match res {
        Ok(resp) if resp.status().is_success() => {
            let parsed: Value = resp.json().await?;
            check_api(parsed, "list websites")?
        }
        _ => {
            let search_url = format!("{}/websites/search", api);
            let resp = c
                .post(search_url)
                .header("1Panel-Token", &token)
                .header("1Panel-Timestamp", &ts)
                .header("Accept-Language", "en")
                .json(&serde_json::json!({
                    "name": "",
                    "page": 1,
                    "pageSize": 999999,
                    "orderBy": "created_at",
                    "order": "null",
                    "websiteGroupId": 0,
                    "type": ""
                }))
                .send()
                .await?;

            let parsed: Value = resp.json().await?;
            check_api(parsed, "search websites")?
        }
    };

    let data = unwrap_data(&json);
    let entries = if let Some(items) = data.get("items").and_then(|x| x.as_array()) {
        items.clone()
    } else {
        data.as_array().cloned().unwrap_or_default()
    };

    Ok(entries.iter().map(normalize_website).collect())
}

async fn get_website_by_id(cfg: &OnePanelConfig, id: u64) -> Result<Website> {
    let api = v2_base(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let c = client(cfg);
    let resp = c
        .get(format!("{}/websites/{}", api, id))
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .header("Accept-Language", "en")
        .send()
        .await?;

    let parsed: Value = resp.json().await?;
    let json = check_api(parsed, "get website by id")?;
    Ok(normalize_website(&unwrap_data(&json)))
}

pub async fn get_website_detail(cfg: &OnePanelConfig, domain: &str) -> Result<Option<Website>> {
    let websites = list_websites(cfg).await?;
    let website = websites
        .into_iter()
        .find(|w| w.domain == domain || w.domains.iter().any(|d| d == domain));

    if let Some(w) = website {
        if let Some(id) = w.id {
            if w.site_path.is_none() {
                return Ok(Some(get_website_by_id(cfg, id).await?));
            }
        }
        return Ok(Some(w));
    }

    Ok(None)
}

async fn resolve_group_id(cfg: &OnePanelConfig, preferred: Option<u64>) -> Result<u64> {
    if let Some(id) = preferred {
        return Ok(id);
    }

    let api = v2_base(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let c = client(cfg);

    let resp = c
        .post(format!("{}/groups/search", api))
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .header("Accept-Language", "en")
        .json(&serde_json::json!({ "type": "website" }))
        .send()
        .await?;

    let parsed: Value = resp.json().await?;
    let data = unwrap_data(&check_api(parsed, "resolve website groups")?);
    let groups = data
        .as_array()
        .ok_or_else(|| anyhow!("resolve website groups: invalid response"))?;

    if let Some(id) = groups
        .iter()
        .find(|g| {
            g.get("isDefault")
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
        })
        .and_then(|g| g.get("id").and_then(|x| x.as_u64()))
    {
        return Ok(id);
    }

    if let Some(id) = groups
        .first()
        .and_then(|g| g.get("id").and_then(|x| x.as_u64()))
    {
        return Ok(id);
    }

    Err(anyhow!(
        "No website group found. Create a website group in 1Panel first, or pass --group-id."
    ))
}

pub async fn create_website(
    cfg: &OnePanelConfig,
    domain: &str,
    alias: Option<&str>,
    group_id: Option<u64>,
) -> Result<Website> {
    let website_group_id = resolve_group_id(cfg, group_id).await?;
    let api = v2_base(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let c = client(cfg);

    let req = serde_json::json!({
        "type": "static",
        "alias": alias.unwrap_or(domain),
        "remark": "",
        "proxy": "",
        "webSiteGroupID": website_group_id,
        "IPV6": false,
        "domains": [{
            "domain": domain,
            "port": 80,
            "ssl": false
        }],
        "ftpUser": "",
        "ftpPassword": "",
        "siteDir": ""
    });

    let resp = c
        .post(format!("{}/websites", api))
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .header("Accept-Language", "en")
        .json(&req)
        .send()
        .await?;

    let parsed: Value = resp.json().await?;
    check_api(parsed, "create website")?;

    get_website_detail(cfg, domain)
        .await?
        .ok_or_else(|| anyhow!("website creation succeeded but website detail was not found"))
}

async fn upload_single_file(
    cfg: &OnePanelConfig,
    file_path: &Path,
    target_dir: &str,
) -> Result<()> {
    let api = v2_base(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let c = client(cfg);

    let file_name = file_path
        .file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| anyhow!("invalid filename for {}", file_path.display()))?
        .to_string();

    let bytes = tokio::fs::read(file_path).await?;
    let part = multipart::Part::bytes(bytes).file_name(file_name);
    let form = multipart::Form::new()
        .part("file", part)
        .text("path", target_dir.to_string())
        .text("overwrite", "True".to_string());

    let resp = c
        .post(format!("{}/files/upload", api))
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .header("Accept-Language", "en")
        .multipart(form)
        .send()
        .await?;

    let parsed: Value = resp.json().await?;
    check_api(parsed, "upload file")?;
    Ok(())
}

fn should_ignore(rel_path: &str) -> bool {
    ["node_modules/", ".git/", ".vscode/", ".env", ".env.local"]
        .iter()
        .any(|pattern| rel_path.contains(pattern) || rel_path.ends_with(pattern))
}

pub async fn deploy_static(
    cfg: &OnePanelConfig,
    source_dir: &Path,
    mut domain: Option<String>,
    create_if_missing: bool,
    alias: Option<&str>,
    group_id: Option<u64>,
) -> Result<DeployStaticResult> {
    let source_abs = source_dir
        .canonicalize()
        .map_err(|e| anyhow!("source path invalid: {}", e))?;

    if !source_abs.is_dir() {
        return Err(anyhow!(
            "Build directory {} does not exist",
            source_abs.display()
        ));
    }

    if domain.is_none() {
        let websites = list_websites(cfg).await?;
        let first = websites
            .first()
            .ok_or_else(|| anyhow!("No websites found; pass --domain"))?;
        domain = Some(first.domain.clone());
    }

    let domain = domain.expect("domain must exist after fallback");

    let mut created = false;
    let mut website = get_website_detail(cfg, &domain).await?;

    if website.is_none() {
        if !create_if_missing {
            return Err(anyhow!(
                "Website not found: {}. Pass --create-if-missing to auto create.",
                domain
            ));
        }
        website = Some(create_website(cfg, &domain, alias, group_id).await?);
        created = true;
    }

    let website = website.ok_or_else(|| anyhow!("Website resolve failed"))?;
    let site_path = website
        .site_path
        .clone()
        .ok_or_else(|| anyhow!("sitePath is missing for website {}", domain))?;

    let mut total = 0usize;
    let mut uploaded = 0usize;
    let mut failed = 0usize;

    for entry in WalkDir::new(&source_abs) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let file = entry.path().to_path_buf();
        let rel = file
            .strip_prefix(&source_abs)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");

        if should_ignore(&rel) {
            continue;
        }

        total += 1;

        let parent_rel = PathBuf::from(&rel)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();

        let mut target_dir = site_path.clone();
        if !target_dir.ends_with('/') {
            target_dir.push('/');
        }

        if !parent_rel.is_empty() && parent_rel != "." {
            target_dir.push_str(&parent_rel);
        }

        match upload_single_file(cfg, &file, &target_dir).await {
            Ok(_) => uploaded += 1,
            Err(_) => failed += 1,
        }
    }

    if failed > 0 {
        return Err(anyhow!(
            "static deploy failed: uploaded {}/{} files, {} failed",
            uploaded,
            total,
            failed
        ));
    }

    Ok(DeployStaticResult {
        ok: true,
        action: "deploy",
        domain: domain.clone(),
        website_id: website.id,
        created,
        group_id: website.group_id.or(group_id),
        source_path: source_abs.display().to_string(),
        uploaded,
        failed,
        total,
        site_path: website.site_path,
        url: format!("https://{}", domain),
    })
}

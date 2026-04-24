use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::{multipart, Client};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct OnePanelConfig {
    pub host: String,
    pub port: u16,
    pub api_key: String,
    pub scheme: String,
    pub insecure_skip_tls_verify: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComposeInfo {
    pub name: String,
    pub path: String,
    pub status: Option<String>,
}

impl OnePanelConfig {
    pub fn new(
        host: String,
        port: u16,
        api_key: String,
        scheme: String,
        insecure_skip_tls_verify: bool,
    ) -> Self {
        Self {
            host,
            port,
            api_key,
            scheme,
            insecure_skip_tls_verify,
        }
    }
}

fn clean_host(host: &str) -> String {
    host.trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_string()
}

fn auth_headers(api_key: &str) -> (String, String) {
    let timestamp = Utc::now().timestamp();
    let token_raw = format!("1panel{}{}", api_key.trim(), timestamp);
    let token_digest = md5::compute(token_raw.as_bytes());
    (format!("{:x}", token_digest), timestamp.to_string())
}

fn get_client(cfg: &OnePanelConfig) -> Client {
    Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(cfg.insecure_skip_tls_verify)
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn v2_base(cfg: &OnePanelConfig) -> String {
    format!(
        "{}://{}:{}/api/v2",
        cfg.scheme,
        clean_host(&cfg.host),
        cfg.port
    )
}

async fn check_api_code(resp: reqwest::Response, action: &str) -> Result<Value> {
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!("{} failed: {} - {}", action, status, body));
    }

    let json: Value = serde_json::from_str(&body)
        .map_err(|e| anyhow!("{} response parse failed: {} | body: {}", action, e, body))?;

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

pub async fn list_composes(cfg: &OnePanelConfig, info: Option<&str>) -> Result<Vec<ComposeInfo>> {
    let client = get_client(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);

    let resp = client
        .post(format!("{}/containers/compose/search", v2_base(cfg)))
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .json(&serde_json::json!({
            "page": 1,
            "pageSize": 200,
            "info": info.unwrap_or("")
        }))
        .send()
        .await?;

    let json = check_api_code(resp, "list_composes").await?;
    let items = json
        .get("data")
        .and_then(|d| d.get("items"))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for item in items {
        let name = item.get("name").and_then(|x| x.as_str()).unwrap_or_default();
        let path = item.get("path").and_then(|x| x.as_str()).unwrap_or_default();
        if name.is_empty() || path.is_empty() {
            continue;
        }

        out.push(ComposeInfo {
            name: name.to_string(),
            path: path.to_string(),
            status: item.get("status").map(|s| s.to_string()),
        });
    }

    Ok(out)
}

pub async fn test_connection(cfg: &OnePanelConfig) -> Result<()> {
    let client = get_client(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("{}/websites/list", v2_base(cfg));

    let resp = client
        .get(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .send()
        .await?;

    check_api_code(resp, "test_connection").await?;
    Ok(())
}

pub async fn upload_file(cfg: &OnePanelConfig, file_path: &Path, remote_dir: &str) -> Result<String> {
    let client = get_client(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("{}/files/upload", v2_base(cfg));

    let file_name = file_path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| anyhow!("invalid filename"))?
        .to_string();

    let file_content = tokio::fs::read(file_path).await?;
    let part_file = multipart::Part::bytes(file_content).file_name(file_name.clone());

    let form = multipart::Form::new()
        .part("file", part_file)
        .part("path", multipart::Part::text(remote_dir.to_string()))
        .part("overwrite", multipart::Part::text("true"));

    let resp = client
        .post(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .multipart(form)
        .send()
        .await?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(anyhow!("upload_file failed: {} - {}", status, body));
    }

    let dir = remote_dir.trim_end_matches('/');
    let inferred_path = format!("{}/{}", dir, file_name);

    let json: Value = match serde_json::from_str(&body) {
        Ok(json) => json,
        Err(_) => return Ok(inferred_path),
    };

    if let Some(code) = json.get("code").and_then(|c| c.as_i64()) {
        if code != 200 {
            let msg = json
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            return Err(anyhow!("upload_file failed (API {}): {}", code, msg));
        }
    }

    if let Some(path) = json.get("data").and_then(|d| d.as_str()) {
        return Ok(path.to_string());
    }

    Ok(inferred_path)
}

pub async fn load_image(cfg: &OnePanelConfig, remote_path: &str) -> Result<()> {
    let client = get_client(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("{}/containers/image/load", v2_base(cfg));

    let resp = client
        .post(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .json(&serde_json::json!({ "paths": [remote_path] }))
        .send()
        .await?;

    check_api_code(resp, "load_image").await?;
    Ok(())
}

pub async fn read_file(cfg: &OnePanelConfig, path: &str) -> Result<String> {
    let client = get_client(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("{}/files/content", v2_base(cfg));

    let resp = client
        .post(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .json(&serde_json::json!({ "path": path }))
        .send()
        .await?;

    let json = check_api_code(resp, "read_file").await?;
    let content = json
        .get("data")
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow!("read_file: missing data.content"))?;

    Ok(content.to_string())
}

pub async fn update_compose(cfg: &OnePanelConfig, _name: &str, path: &str, content: &str) -> Result<()> {
    let client = get_client(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("{}/files/save", v2_base(cfg));

    let resp = client
        .post(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .json(&serde_json::json!({
            "path": path,
            "content": content
        }))
        .send()
        .await?;

    check_api_code(resp, "update_compose").await?;
    Ok(())
}

pub async fn operate_compose(cfg: &OnePanelConfig, name: &str, path: &str, operation: &str) -> Result<()> {
    let client = get_client(cfg);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("{}/containers/compose/operate", v2_base(cfg));

    let resp = client
        .post(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .json(&serde_json::json!({
            "name": name,
            "operation": operation,
            "path": path,
            "withFile": true
        }))
        .send()
        .await?;

    check_api_code(resp, "operate_compose").await?;
    Ok(())
}

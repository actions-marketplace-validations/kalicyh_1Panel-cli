use anyhow::{anyhow, Result};
use chrono::Utc;
use reqwest::{multipart, Client};
use serde_json::Value;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct OnePanelConfig {
    pub host: String,
    pub port: u16,
    pub api_key: String,
}

impl OnePanelConfig {
    pub fn new(host: String, port: u16, api_key: String) -> Self {
        Self {
            host,
            port,
            api_key,
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

fn get_client() -> Client {
    Client::builder().no_proxy().build().unwrap_or_else(|_| Client::new())
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

pub async fn test_connection(cfg: &OnePanelConfig) -> Result<()> {
    let client = get_client();
    let host = clean_host(&cfg.host);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("http://{}:{}/api/v1/system/info", host, cfg.port);

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
    let client = get_client();
    let host = clean_host(&cfg.host);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("http://{}:{}/api/v1/files/upload", host, cfg.port);

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

    let json = check_api_code(resp, "upload_file").await?;

    if let Some(path) = json.get("data").and_then(|d| d.as_str()) {
        return Ok(path.to_string());
    }

    let dir = remote_dir.trim_end_matches('/');
    Ok(format!("{}/{}", dir, file_name))
}

pub async fn load_image(cfg: &OnePanelConfig, remote_path: &str) -> Result<()> {
    let client = get_client();
    let host = clean_host(&cfg.host);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("http://{}:{}/api/v1/containers/image/load", host, cfg.port);

    let resp = client
        .post(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .json(&serde_json::json!({ "path": remote_path }))
        .send()
        .await?;

    check_api_code(resp, "load_image").await?;
    Ok(())
}

pub async fn read_file(cfg: &OnePanelConfig, path: &str) -> Result<String> {
    let client = get_client();
    let host = clean_host(&cfg.host);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("http://{}:{}/api/v1/files/content", host, cfg.port);

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

pub async fn update_compose(cfg: &OnePanelConfig, name: &str, path: &str, content: &str) -> Result<()> {
    let client = get_client();
    let host = clean_host(&cfg.host);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("http://{}:{}/api/v1/containers/compose/update", host, cfg.port);

    let resp = client
        .post(url)
        .header("1Panel-Token", token)
        .header("1Panel-Timestamp", ts)
        .json(&serde_json::json!({
            "name": name,
            "path": path,
            "content": content,
            "env": []
        }))
        .send()
        .await?;

    check_api_code(resp, "update_compose").await?;
    Ok(())
}

pub async fn operate_compose(cfg: &OnePanelConfig, name: &str, path: &str, operation: &str) -> Result<()> {
    let client = get_client();
    let host = clean_host(&cfg.host);
    let (token, ts) = auth_headers(&cfg.api_key);
    let url = format!("http://{}:{}/api/v1/containers/compose/operate", host, cfg.port);

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

use crate::onepanel::{self, OnePanelConfig};
use anyhow::{Result, anyhow};
use serde::Serialize;
use serde_yaml::Value;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub tar_path: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResult {
    pub remote_path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImageChange {
    pub service: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize)]
pub struct ComposeUpdateResult {
    pub changed: usize,
    pub changes: Vec<ImageChange>,
    pub dry_run: bool,
}

#[derive(Debug)]
pub struct ComposeUpdateOpts {
    pub compose_name: String,
    pub compose_path: String,
    pub service: Option<String>,
    pub from_image: Option<String>,
    pub to_image: String,
    pub dry_run: bool,
    pub apply: bool,
}

pub fn resolve_compose_name(compose_name: Option<String>, compose_path: &str) -> Result<String> {
    if let Some(name) = compose_name {
        let trimmed = name.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    infer_compose_name_from_path(compose_path)
}

fn infer_compose_name_from_path(compose_path: &str) -> Result<String> {
    let path = Path::new(compose_path);

    if let Some(parent_name) = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
    {
        let trimmed = parent_name.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Some(file_stem) = path.file_stem().and_then(|value| value.to_str()) {
        let trimmed = file_stem.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    Err(anyhow!(
        "unable to infer compose name from --compose-path: {}",
        compose_path
    ))
}

fn ensure_success(status: std::process::ExitStatus, step: &str) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("{} failed with status {:?}", step, status.code()))
    }
}

pub async fn export_image(image_tag: &str, output: &Path) -> Result<ExportResult> {
    let status = Command::new("docker")
        .arg("save")
        .arg("-o")
        .arg(output)
        .arg(image_tag)
        .status()
        .await?;

    ensure_success(status, "docker save")?;

    Ok(ExportResult {
        tar_path: output.display().to_string(),
    })
}

pub async fn upload_image_tar(
    cfg: &OnePanelConfig,
    input: &Path,
    remote_dir: &str,
) -> Result<UploadResult> {
    let remote_path = onepanel::upload_file(cfg, input, remote_dir).await?;
    Ok(UploadResult { remote_path })
}

pub async fn load_remote_image(cfg: &OnePanelConfig, remote_path: &str) -> Result<()> {
    onepanel::load_image(cfg, remote_path).await
}

pub fn update_compose_images(
    content: &str,
    opts: &ComposeUpdateOpts,
) -> Result<(String, Vec<ImageChange>)> {
    let mut root: Value = serde_yaml::from_str(content)?;
    let mut changes = Vec::new();

    let root_map = root
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("compose file is not a YAML object"))?;

    let services_key = Value::String("services".to_string());
    let services = root_map
        .get_mut(&services_key)
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| anyhow!("compose file missing services"))?;

    let keys: Vec<Value> = services.keys().cloned().collect();

    for service_key in keys {
        let service_name = match service_key.as_str() {
            Some(v) => v.to_string(),
            None => continue,
        };

        if let Some(target_service) = &opts.service {
            if &service_name != target_service {
                continue;
            }
        }

        let Some(service_obj) = services
            .get_mut(&service_key)
            .and_then(Value::as_mapping_mut)
        else {
            continue;
        };

        let image_key = Value::String("image".to_string());
        let Some(current_image) = service_obj.get(&image_key).and_then(Value::as_str) else {
            continue;
        };

        if let Some(from_image) = &opts.from_image {
            if current_image != from_image {
                continue;
            }
        }

        if current_image == opts.to_image {
            continue;
        }

        changes.push(ImageChange {
            service: service_name,
            from: current_image.to_string(),
            to: opts.to_image.clone(),
        });

        service_obj.insert(image_key, Value::String(opts.to_image.clone()));
    }

    if changes.is_empty() {
        return Err(anyhow!("no image entries matched update conditions"));
    }

    let updated = serde_yaml::to_string(&root)?;
    Ok((updated, changes))
}

pub async fn run_compose_update(
    cfg: &OnePanelConfig,
    opts: ComposeUpdateOpts,
) -> Result<ComposeUpdateResult> {
    let current = onepanel::read_file(cfg, &opts.compose_path).await?;
    let (updated, changes) = update_compose_images(&current, &opts)?;

    if !opts.dry_run {
        onepanel::update_compose(cfg, &opts.compose_name, &opts.compose_path, &updated).await?;

        if opts.apply {
            onepanel::operate_compose(cfg, &opts.compose_name, &opts.compose_path, "up").await?;
        }
    }

    Ok(ComposeUpdateResult {
        changed: changes.len(),
        changes,
        dry_run: opts.dry_run,
    })
}

pub async fn deploy_all(
    cfg: &OnePanelConfig,
    image_tag: &str,
    remote_dir: &str,
    keep_local_tar: bool,
) -> Result<(ExportResult, UploadResult)> {
    let tar_path = temp_tar_path(image_tag);
    let export = export_image(image_tag, &tar_path).await?;
    let upload = upload_image_tar(cfg, &tar_path, remote_dir).await?;
    load_remote_image(cfg, &upload.remote_path).await?;

    if !keep_local_tar {
        let _ = tokio::fs::remove_file(&tar_path).await;
    }

    Ok((export, upload))
}

pub async fn deploy_all_and_compose(
    cfg: &OnePanelConfig,
    image_tag: &str,
    remote_dir: &str,
    keep_local_tar: bool,
    mut compose: ComposeUpdateOpts,
) -> Result<(ExportResult, UploadResult, ComposeUpdateResult)> {
    let (export, upload) = deploy_all(cfg, image_tag, remote_dir, keep_local_tar).await?;
    if compose.to_image.trim().is_empty() {
        compose.to_image = image_tag.to_string();
    }
    let compose_result = run_compose_update(cfg, compose).await?;
    Ok((export, upload, compose_result))
}

fn temp_tar_path(image_tag: &str) -> PathBuf {
    let safe = image_tag.replace(['/', ':', '@'], "_");
    std::env::temp_dir().join(format!("{}_image.tar", safe))
}

#[cfg(test)]
mod tests {
    use super::{ComposeUpdateOpts, resolve_compose_name, update_compose_images};

    fn multi_service_compose() -> &'static str {
        r#"
services:
  app:
    image: example/app:v1
  worker:
    image: example/worker:v1
"#
    }

    fn single_service_compose() -> &'static str {
        r#"
services:
  docmost:
    image: gitea.nz.com/tigger/wiki:v1.0.1
"#
    }

    fn opts(service: Option<&str>, from_image: Option<&str>, to_image: &str) -> ComposeUpdateOpts {
        ComposeUpdateOpts {
            compose_name: "wiki".to_string(),
            compose_path: "/opt/1panel/docker/compose/wiki/docker-compose.yml".to_string(),
            service: service.map(ToString::to_string),
            from_image: from_image.map(ToString::to_string),
            to_image: to_image.to_string(),
            dry_run: false,
            apply: false,
        }
    }

    #[test]
    fn keeps_explicit_compose_name() {
        let name = resolve_compose_name(
            Some("wiki".to_string()),
            "/opt/1panel/docker/compose/wiki/docker-compose.yml",
        )
        .expect("expected compose name");
        assert_eq!(name, "wiki");
    }

    #[test]
    fn infers_compose_name_from_parent_directory() {
        let name = resolve_compose_name(None, "/opt/1panel/docker/compose/wiki/docker-compose.yml")
            .expect("expected compose name");
        assert_eq!(name, "wiki");
    }

    #[test]
    fn infers_compose_name_from_file_stem_when_parent_missing() {
        let name = resolve_compose_name(None, "docker-compose.yml").expect("expected compose name");
        assert_eq!(name, "docker-compose");
    }

    #[test]
    fn updates_all_image_services_when_no_filters_are_provided() {
        let (_, changes) =
            update_compose_images(multi_service_compose(), &opts(None, None, "example/new:v2"))
                .expect("expected compose update");
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].service, "app");
        assert_eq!(changes[1].service, "worker");
    }

    #[test]
    fn filters_by_from_image_when_present() {
        let (_, changes) = update_compose_images(
            multi_service_compose(),
            &opts(None, Some("example/worker:v1"), "example/new:v2"),
        )
        .expect("expected compose update");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].service, "worker");
    }

    #[test]
    fn filters_by_explicit_service_when_present() {
        let (_, changes) = update_compose_images(
            multi_service_compose(),
            &opts(Some("app"), None, "example/new:v2"),
        )
        .expect("expected compose update");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].service, "app");
    }

    #[test]
    fn updates_single_service_compose_without_filters() {
        let (_, changes) = update_compose_images(
            single_service_compose(),
            &opts(None, None, "gitea.nz.com/tigger/wiki:v1.0.2"),
        )
        .expect("expected compose update");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].service, "docmost");
    }
}

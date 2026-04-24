use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod deploy;
mod onepanel;
mod static_site;

use deploy::{ComposeUpdateOpts, ComposeUpdateResult};
use onepanel::OnePanelConfig;

#[derive(Parser, Debug)]
#[command(name = "1panel-cli", about = "Standalone 1Panel deployment CLI")]
struct Cli {
    #[arg(long, global = true, default_value_t = false)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Set persistent local defaults (base-url/api-key/host/port/insecure).
    Set {
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        api_key: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        insecure: Option<bool>,
    },
    /// View or manage local config values.
    Config {
        #[arg(long)]
        unset: Option<String>,
    },

    /// Deploy static files to a website.
    Deploy {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        group_id: Option<u64>,
        #[arg(long)]
        alias: Option<String>,
        #[arg(long, default_value_t = false)]
        create_if_missing: bool,
        #[arg(short = 'y', long, default_value_t = false)]
        yes: bool,
        #[arg(long, default_value_t = false)]
        non_interactive: bool,
    },

    /// List websites available in 1Panel.
    ListWebsites {
        #[command(flatten)]
        auth: ServerAuthArgs,
    },

    /// List compose files available in 1Panel.
    ListComposes {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        info: Option<String>,
    },

    /// Check 1Panel connectivity and auth.
    ServerTest(ServerAuthArgs),

    /// Export local docker image tarball.
    ImageExport {
        #[arg(long)]
        image_tag: String,
        #[arg(long)]
        output: PathBuf,
    },

    /// Upload local image tar to 1Panel.
    ImageUpload {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "/opt/1panel/tmp")]
        remote_dir: String,
    },

    /// Load remote image tar on 1Panel.
    DeployLoad {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        remote_path: String,
    },

    /// Update compose image references on 1Panel.
    DeployComposeUpdate {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        compose_name: Option<String>,
        #[arg(long)]
        compose_path: String,
        #[arg(long)]
        to_image: String,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        from_image: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        apply: bool,
    },

    /// Full pipeline: export -> upload -> load.
    DeployAll {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        image_tag: String,
        #[arg(long, default_value = "/opt/1panel/tmp")]
        remote_dir: String,
        #[arg(long, default_value_t = false)]
        keep_local_tar: bool,
    },

    /// Full pipeline: export -> upload -> load -> compose update -> compose up.
    DeployAllCompose {
        #[command(flatten)]
        auth: ServerAuthArgs,
        #[arg(long)]
        image_tag: String,
        #[arg(long, default_value = "/opt/1panel/tmp")]
        remote_dir: String,
        #[arg(long, default_value_t = false)]
        keep_local_tar: bool,
        #[arg(long)]
        compose_name: Option<String>,
        #[arg(long)]
        compose_path: String,
        #[arg(long)]
        to_image: Option<String>,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        from_image: Option<String>,
        #[arg(long, default_value_t = true)]
        apply: bool,
    },
}

#[derive(clap::Args, Debug, Clone)]
struct ServerAuthArgs {
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    api_key: Option<String>,
    #[arg(long)]
    base_url: Option<String>,
    #[arg(long, default_value_t = false)]
    insecure: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LocalConfig {
    base_url: Option<String>,
    api_key: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    insecure: Option<bool>,
}

#[derive(Serialize)]
struct OkMessage<'a> {
    ok: bool,
    message: &'a str,
}

fn config_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").or_else(|_| {
        if cfg!(windows) {
            std::env::var("USERPROFILE")
        } else {
            Err(std::env::VarError::NotPresent)
        }
    });
    let home = home.map_err(|_| anyhow!("HOME is not set (Windows also accepts USERPROFILE)"))?;
    Ok(PathBuf::from(home).join(".1panel-cli").join("config.json"))
}

fn read_local_config() -> Result<LocalConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(LocalConfig::default());
    }
    let content = std::fs::read_to_string(path)?;
    let cfg: LocalConfig = serde_json::from_str(&content)?;
    Ok(cfg)
}

fn write_local_config(cfg: &LocalConfig) -> Result<PathBuf> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&path, data)?;
    Ok(path)
}

fn parse_base_url(base_url: &str) -> Result<(String, u16, String)> {
    let normalized = if base_url.starts_with("http://") || base_url.starts_with("https://") {
        base_url.to_string()
    } else {
        format!("http://{}", base_url)
    };

    let url = Url::parse(&normalized).map_err(|e| anyhow!("invalid --base-url: {}", e))?;
    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("invalid --base-url: missing host"))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(9999);
    let scheme = url.scheme().to_string();

    Ok((host, port, scheme))
}

fn cfg_from(auth: &ServerAuthArgs) -> Result<OnePanelConfig> {
    let local = read_local_config()?;
    let env_host = std::env::var("ONEPANEL_HOST").ok();
    let env_port = std::env::var("ONEPANEL_PORT").ok().and_then(|v| v.parse::<u16>().ok());
    let env_api_key = std::env::var("ONEPANEL_API_KEY").ok();
    let env_base_url = std::env::var("ONEPANEL_BASE_URL").ok();
    let env_insecure = std::env::var("ONEPANEL_INSECURE")
        .ok()
        .map(|v| {
            let n = v.trim().to_ascii_lowercase();
            n == "1" || n == "true" || n == "yes" || n == "on"
        })
        .unwrap_or(false);

    let base_url = auth
        .base_url
        .clone()
        .or(env_base_url)
        .or(local.base_url.clone());
    let parsed_base = match base_url {
        Some(url) => Some(parse_base_url(&url)?),
        None => None,
    };

    let host = auth
        .host
        .clone()
        .or(env_host)
        .or(local.host)
        .or_else(|| parsed_base.as_ref().map(|(h, _, _)| h.clone()))
        .ok_or_else(|| anyhow!("host is required (use --host, --base-url, ONEPANEL_HOST, or ONEPANEL_BASE_URL)"))?;

    let port = auth
        .port
        .or(env_port)
        .or(local.port)
        .or_else(|| parsed_base.as_ref().map(|(_, p, _)| *p))
        .unwrap_or(9999);

    let scheme = parsed_base
        .as_ref()
        .map(|(_, _, s)| s.clone())
        .unwrap_or_else(|| "http".to_string());

    let api_key = auth
        .api_key
        .clone()
        .or(env_api_key)
        .or(local.api_key)
        .ok_or_else(|| anyhow!("api key is required (use --api-key or ONEPANEL_API_KEY)"))?;

    let insecure_skip_tls_verify = auth.insecure || env_insecure || local.insecure.unwrap_or(false);

    Ok(OnePanelConfig::new(
        host,
        port,
        api_key,
        scheme,
        insecure_skip_tls_verify,
    ))
}

fn print_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string_pretty(value).unwrap());
}

fn print_compose_result_human(result: &ComposeUpdateResult) {
    println!("changed={} dry_run={}", result.changed, result.dry_run);
    for c in &result.changes {
        println!("{}: {} -> {}", c.service, c.from, c.to);
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Set {
            base_url,
            api_key,
            host,
            port,
            insecure,
        } => {
            let mut cfg = read_local_config()?;

            if let Some(v) = base_url {
                cfg.base_url = Some(v);
            }
            if let Some(v) = api_key {
                cfg.api_key = Some(v);
            }
            if let Some(v) = host {
                cfg.host = Some(v);
            }
            if let Some(v) = port {
                cfg.port = Some(v);
            }
            if let Some(v) = insecure {
                cfg.insecure = Some(v);
            }

            let path = write_local_config(&cfg)?;

            if cli.json {
                print_json(&serde_json::json!({
                    "ok": true,
                    "action": "set",
                    "path": path,
                    "config": cfg
                }));
            } else {
                println!("saved config: {}", path.display());
            }
        }
        Commands::Config { unset } => {
            if let Some(key) = unset {
                let mut cfg = read_local_config()?;
                match key.as_str() {
                    "base-url" | "base_url" => cfg.base_url = None,
                    "api-key" | "api_key" => cfg.api_key = None,
                    "host" => cfg.host = None,
                    "port" => cfg.port = None,
                    "insecure" => cfg.insecure = None,
                    _ => {
                        return Err(anyhow!(
                            "unsupported key: {} (allowed: base-url, api-key, host, port, insecure)",
                            key
                        ));
                    }
                }

                let path = write_local_config(&cfg)?;
                if cli.json {
                    print_json(&serde_json::json!({
                        "ok": true,
                        "action": "config-unset",
                        "key": key,
                        "path": path,
                        "config": cfg
                    }));
                } else {
                    println!("unset {} in {}", key, path.display());
                }
            } else {
                let cfg = read_local_config()?;
                if cli.json {
                    print_json(&serde_json::json!({
                        "ok": true,
                        "action": "config",
                        "config": cfg
                    }));
                } else {
                    println!("base_url: {}", cfg.base_url.unwrap_or_else(|| "<unset>".to_string()));
                    println!("api_key: {}", if cfg.api_key.is_some() { "<set>" } else { "<unset>" });
                    println!("host: {}", cfg.host.unwrap_or_else(|| "<unset>".to_string()));
                    println!("port: {}", cfg.port.map(|v| v.to_string()).unwrap_or_else(|| "<unset>".to_string()));
                    println!(
                        "insecure: {}",
                        cfg.insecure
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "<unset>".to_string())
                    );
                }
            }
        }
        Commands::Deploy {
            auth,
            path,
            domain,
            group_id,
            alias,
            create_if_missing,
            yes,
            non_interactive: _,
        } => {
            let cfg = cfg_from(&auth)?;
            let result = static_site::deploy_static(&cfg, &path, domain, create_if_missing || yes, alias.as_deref(), group_id).await?;

            if cli.json {
                print_json(&result);
            } else {
                println!("Deployment completed successfully.");
                println!("Website: {}", result.url);
                println!("Files uploaded: {}/{}", result.uploaded, result.total);
            }
        }
        Commands::ListWebsites { auth } => {
            let cfg = cfg_from(&auth)?;
            let websites = static_site::list_websites(&cfg).await?;
            let payload = serde_json::json!({
                "ok": true,
                "action": "list-websites",
                "count": websites.len(),
                "websites": websites,
            });

            if cli.json {
                print_json(&payload);
            } else if websites.is_empty() {
                println!("No websites found.");
            } else {
                for w in websites {
                    println!("{}", w.domain);
                }
            }
        }
        Commands::ListComposes { auth, info } => {
            let cfg = cfg_from(&auth)?;
            let composes = onepanel::list_composes(&cfg, info.as_deref()).await?;
            let payload = serde_json::json!({
                "ok": true,
                "action": "list-composes",
                "count": composes.len(),
                "composes": composes,
            });

            if cli.json {
                print_json(&payload);
            } else if composes.is_empty() {
                println!("No compose files found.");
            } else {
                for c in composes {
                    println!("{}  {}", c.name, c.path);
                }
            }
        }
        Commands::ServerTest(auth) => {
            onepanel::test_connection(&cfg_from(&auth)?).await?;
            if cli.json {
                print_json(&OkMessage {
                    ok: true,
                    message: "connection successful",
                });
            } else {
                println!("connection successful");
            }
        }
        Commands::ImageExport { image_tag, output } => {
            let result = deploy::export_image(&image_tag, &output).await?;
            if cli.json {
                print_json(&result);
            } else {
                println!("exported: {}", result.tar_path);
            }
        }
        Commands::ImageUpload {
            auth,
            input,
            remote_dir,
        } => {
            let result = deploy::upload_image_tar(&cfg_from(&auth)?, &input, &remote_dir).await?;
            if cli.json {
                print_json(&result);
            } else {
                println!("uploaded: {}", result.remote_path);
            }
        }
        Commands::DeployLoad { auth, remote_path } => {
            deploy::load_remote_image(&cfg_from(&auth)?, &remote_path).await?;
            if cli.json {
                print_json(&OkMessage {
                    ok: true,
                    message: "image loaded",
                });
            } else {
                println!("image loaded");
            }
        }
        Commands::DeployComposeUpdate {
            auth,
            compose_name,
            compose_path,
            to_image,
            service,
            from_image,
            dry_run,
            apply,
        } => {
            let result = deploy::run_compose_update(
                &cfg_from(&auth)?,
                ComposeUpdateOpts {
                    compose_name: deploy::resolve_compose_name(compose_name, &compose_path)?,
                    compose_path,
                    service,
                    from_image,
                    to_image,
                    dry_run,
                    apply,
                },
            )
            .await?;

            if cli.json {
                print_json(&result);
            } else {
                print_compose_result_human(&result);
            }
        }
        Commands::DeployAll {
            auth,
            image_tag,
            remote_dir,
            keep_local_tar,
        } => {
            let (export, upload) =
                deploy::deploy_all(&cfg_from(&auth)?, &image_tag, &remote_dir, keep_local_tar).await?;

            if cli.json {
                print_json(&serde_json::json!({
                    "export": export,
                    "upload": upload,
                    "loaded": true
                }));
            } else {
                println!("exported: {}", export.tar_path);
                println!("uploaded: {}", upload.remote_path);
                println!("image loaded");
            }
        }
        Commands::DeployAllCompose {
            auth,
            image_tag,
            remote_dir,
            keep_local_tar,
            compose_name,
            compose_path,
            to_image,
            service,
            from_image,
            apply,
        } => {
            let cfg = cfg_from(&auth)?;
            let compose_result = deploy::deploy_all_and_compose(
                &cfg,
                &image_tag,
                &remote_dir,
                keep_local_tar,
                ComposeUpdateOpts {
                    compose_name: deploy::resolve_compose_name(compose_name, &compose_path)?,
                    compose_path,
                    service,
                    from_image,
                    to_image: to_image.unwrap_or_else(|| image_tag.clone()),
                    dry_run: false,
                    apply,
                },
            )
            .await?;

            if cli.json {
                print_json(&serde_json::json!({
                    "export": compose_result.0,
                    "upload": compose_result.1,
                    "compose": compose_result.2,
                    "loaded": true
                }));
            } else {
                println!("exported: {}", compose_result.0.tar_path);
                println!("uploaded: {}", compose_result.1.remote_path);
                println!("image loaded");
                print_compose_result_human(&compose_result.2);
            }
        }
    }

    Ok(())
}

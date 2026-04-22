use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

mod deploy;
mod onepanel;

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
        compose_name: String,
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
}

#[derive(clap::Args, Debug, Clone)]
struct ServerAuthArgs {
    #[arg(long)]
    host: String,
    #[arg(long, default_value_t = 9999)]
    port: u16,
    #[arg(long)]
    api_key: String,
}

#[derive(Serialize)]
struct OkMessage<'a> {
    ok: bool,
    message: &'a str,
}

fn cfg_from(auth: &ServerAuthArgs) -> OnePanelConfig {
    OnePanelConfig::new(auth.host.clone(), auth.port, auth.api_key.clone())
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
        Commands::ServerTest(auth) => {
            onepanel::test_connection(&cfg_from(&auth)).await?;
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
            let result = deploy::upload_image_tar(&cfg_from(&auth), &input, &remote_dir).await?;
            if cli.json {
                print_json(&result);
            } else {
                println!("uploaded: {}", result.remote_path);
            }
        }
        Commands::DeployLoad { auth, remote_path } => {
            deploy::load_remote_image(&cfg_from(&auth), &remote_path).await?;
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
            if service.is_none() && from_image.is_none() {
                return Err(anyhow!(
                    "compose update requires --service or --from-image to avoid accidental global replacement"
                ));
            }

            let result = deploy::run_compose_update(
                &cfg_from(&auth),
                ComposeUpdateOpts {
                    compose_name,
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
                deploy::deploy_all(&cfg_from(&auth), &image_tag, &remote_dir, keep_local_tar).await?;

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
    }

    Ok(())
}

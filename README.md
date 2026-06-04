# 1Panel CLI

A command-line toolset for 1Panel deployment workflows.

This project was originally adapted from [ruibaby/1Panel-rocket-cli](https://github.com/ruibaby/1Panel-rocket-cli).

[中文文档](README.zh.md)

## Components

- Node CLI (`src/index.mjs`): static website deploy workflow
- Rust CLI (`rust/1panel-cli`): image/compose/static-site workflow for CI and automation
- GitHub Action (`action.yml`): Rust-only composite wrapper around the Rust CLI

See [GitHub Action demos](GITHUB_ACTION_DEMOS.md) for static deploy, compose update, image upload, and full Docker image deployment examples.

## Rust CLI Highlights

Rust binary: `1panel-cli`

Key commands:

- `deploy`: deploy static files by domain
- `list-websites`: list websites
- `list-composes`: list compose files
- `deploy-compose-update`: update image refs in compose and optionally apply
- `deploy-all`: export image -> upload -> load
- `deploy-all-compose`: export image -> upload -> load -> compose update -> compose up
- `set`: persist defaults (`base-url`, `api-key`, `host`, `port`, `insecure`)
- `config`: view local config or `--unset` a key

## GitHub Action Quick Start

```yaml
- name: Deploy static site to 1Panel
  uses: kalicyh/1Panel-rocket-cli@v1
  with:
    command: deploy
    base-url: ${{ secrets.ONEPANEL_BASE_URL }}
    api-key: ${{ secrets.ONEPANEL_API_KEY }}
    path: ./dist
    domain: example.com
```

## Rust CLI Quick Start

```bash
# 1) set defaults once
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  set --base-url https://your.panel.domain --api-key <API_KEY> --insecure true

# 2) check saved config
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- config

# 3) list compose files
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- list-composes

# 4) update a compose image and deploy
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-compose-update \
  --compose-name wiki \
  --compose-path /opt/1panel/docker/compose/wiki/docker-compose.yml \
  --from-image gitea.nz.com/tigger/wiki:v1.0.1 \
  --to-image gitea.nz.com/tigger/wiki:v1.0.2 \
  --apply
```

## TLS / Self-signed Cert

Use `--insecure` to skip TLS certificate verification.

Supported in all Rust API commands.

You can also set:

```bash
export ONEPANEL_INSECURE=true
```

## Config Priority (Rust CLI)

1. CLI args
2. Environment variables
3. Local config file (`~/.1panel-cli/config.json`)

## Existing Node CLI (Static Site)

```bash
# Deploy static website
1panel-cli -p ./dist -d example.com

# List websites
1panel-cli list-websites
```

## Skills

- Existing skill: `skills/1panel-cli-ai/SKILL.md`
- New Rust skill: `skills/1panel-rust-cli/SKILL.md`

## Development

```bash
# Node
pnpm install
pnpm build

# Rust
cargo check --manifest-path ./rust/1panel-cli/Cargo.toml
```

## License

MIT

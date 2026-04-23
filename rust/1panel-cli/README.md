# 1panel-cli (Rust)

独立部署工具（无数据库），用于 CI 中将静态站点和 Docker 镜像发布到 1Panel。

## Build

```bash
cargo build --manifest-path ./rust/1panel-cli/Cargo.toml --release
```

## Core Commands

```bash
# 查看全部命令
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- --help

# 1) 设置本地默认配置（只需一次）
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  set --base-url https://panel.example.com --api-key <API_KEY> --insecure true

# 2) 查看本地配置
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- config

# 3) 清除某个配置项
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- config --unset api-key

# 4) 按域名更新静态网站
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy --path ./dist --domain example.com --create-if-missing

# 5) 列出编排文件（用于选择）
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- list-composes

# 6) 更新 compose 镜像并部署（推荐先 --dry-run）
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-compose-update \
  --compose-path /opt/1panel/docker/compose/example-app/docker-compose.yml \
  --service docmost \
  --from-image registry.example.com/example/app:v1.0.1 \
  --to-image registry.example.com/example/app:v1.0.2 \
  --apply

# 7) 一键流程：导出镜像 -> 上传 -> load -> 更新 compose -> up
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-all-compose \
  --image-tag registry.example.com/example/app:v1.0.2 \
  --compose-path /opt/1panel/docker/compose/example-app/docker-compose.yml \
  --from-image registry.example.com/example/app:v1.0.1 \
  --service docmost \
  --apply
```

## Three Scenarios

1. 根据域名更新静态网站：`deploy --path ... --domain ...`
2. 选择编排文件更新镜像部署：`list-composes` + `deploy-compose-update`
3. 导出上传后按编排部署：`deploy-all-compose`

## TLS 证书

- 使用 `--insecure` 忽略 TLS 证书校验
- 或设置环境变量：`ONEPANEL_INSECURE=true`

## Config

本地配置文件：`~/.1panel-cli/config.json`

优先级：

1. 命令行参数
2. 环境变量
3. 本地配置

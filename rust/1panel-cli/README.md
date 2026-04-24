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
  --from-image registry.example.com/example/app:v1.0.1 \
  --to-image registry.example.com/example/app:v1.0.2 \
  --apply

# 7) 一键流程：导出镜像 -> 上传 -> load -> 更新 compose -> up
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-all-compose \
  --image-tag registry.example.com/example/app:v1.0.2 \
  --compose-path /opt/1panel/docker/compose/example-app/docker-compose.yml \
  --apply
```

## Three Scenarios

1. 根据域名更新静态网站：`deploy --path ... --domain ...`
2. 选择编排文件更新镜像部署：`list-composes` + `deploy-compose-update`
3. 导出上传后按编排部署：`deploy-all-compose`

## Current Behavior

- `deploy-compose-update`:
  - `--compose-name` 可省略，会从 `--compose-path` 自动推导
  - `--service` 和 `--from-image` 都是可选过滤条件
  - 只传 `--compose-path + --to-image` 时，会把 compose 中所有 `image` 更新为目标镜像
- `deploy-all-compose`:
  - `--compose-name` 可省略，会从 `--compose-path` 自动推导
  - `--to-image` 可省略，默认等于 `--image-tag`
  - `--service` 和 `--from-image` 都是可选过滤条件
- API 兼容性:
  - 默认先尝试 1Panel `v2` API
  - 若 `v2` 不可用，会自动回退到 `v1` API
- 响应兼容性:
  - 若响应前面混入 HTML、后面才是正常 JSON，会自动跳过前缀并解析后面的 JSON
- Windows:
  - 若未设置 `HOME`，会自动回退到 `USERPROFILE`

## Minimal Examples

```bash
# 仅测试连接（推荐在新服务器上先跑一次）
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  server-test --base-url https://panel.example.com --api-key <API_KEY> --insecure

# 仅按 compose 路径把所有 image 更新为目标镜像
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-compose-update \
  --base-url https://panel.example.com \
  --api-key <API_KEY> \
  --insecure \
  --compose-path /opt/1panel/docker/compose/example-app/docker-compose.yml \
  --to-image registry.example.com/example/app:v1.0.11

# 一键导出/上传/load/更新 compose/up
cargo run --manifest-path ./rust/1panel-cli/Cargo.toml -- \
  deploy-all-compose \
  --base-url https://panel.example.com \
  --api-key <API_KEY> \
  --insecure \
  --image-tag registry.example.com/example/app:v1.0.11 \
  --compose-path /opt/1panel/docker/compose/example-app/docker-compose.yml
```

## TLS 证书

- 使用 `--insecure` 忽略 TLS 证书校验
- 或设置环境变量：`ONEPANEL_INSECURE=true`

## Config

本地配置文件：`~/.1panel-cli/config.json`

Windows 下若未设置 `HOME`，会使用 `USERPROFILE/.1panel-cli/config.json`

优先级：

1. 命令行参数
2. 环境变量
3. 本地配置

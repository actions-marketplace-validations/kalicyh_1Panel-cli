# 1panel-cli

独立部署工具（无数据库），用于 CI 中将本地 Docker 镜像发布到 1Panel。

## Build

```bash
cargo build -p onepanel-cli --release
```

## Commands

```bash
# 1) 连接测试
cargo run -p onepanel-cli -- server-test --host 1.2.3.4 --port 9999 --api-key xxx

# 2) 导出本地镜像
cargo run -p onepanel-cli -- image-export --image-tag myapp:sha-123 --output /tmp/myapp.tar

# 3) 上传镜像包到 1Panel
cargo run -p onepanel-cli -- image-upload \
  --host 1.2.3.4 --port 9999 --api-key xxx \
  --input /tmp/myapp.tar --remote-dir /opt/1panel/tmp

# 4) 在 1Panel 导入镜像
cargo run -p onepanel-cli -- deploy-load \
  --host 1.2.3.4 --port 9999 --api-key xxx \
  --remote-path /opt/1panel/tmp/myapp.tar

# 5) 一键流程（export -> upload -> load）
cargo run -p onepanel-cli -- deploy-all \
  --host 1.2.3.4 --port 9999 --api-key xxx \
  --image-tag myapp:sha-123

# 6) 更新 compose 镜像
cargo run -p onepanel-cli -- deploy-compose-update \
  --host 1.2.3.4 --port 9999 --api-key xxx \
  --compose-name my-stack \
  --compose-path /opt/1panel/docker/compose/my-stack/docker-compose.yml \
  --service web \
  --to-image myapp:sha-123 \
  --dry-run
```

`deploy-compose-update` 保护策略：必须传 `--service` 或 `--from-image`，避免全量误替换。

## GitHub Actions 示例

```yaml
- name: Build CLI
  run: cargo build -p onepanel-cli --release

- name: Deploy image to 1Panel
  env:
    ONEPANEL_HOST: ${{ secrets.ONEPANEL_HOST }}
    ONEPANEL_PORT: ${{ secrets.ONEPANEL_PORT }}
    ONEPANEL_API_KEY: ${{ secrets.ONEPANEL_API_KEY }}
  run: |
    ./target/release/1panel-cli deploy-all \
      --host "$ONEPANEL_HOST" \
      --port "$ONEPANEL_PORT" \
      --api-key "$ONEPANEL_API_KEY" \
      --image-tag "myapp:${GITHUB_SHA}"
```

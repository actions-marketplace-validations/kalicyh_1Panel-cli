# GitHub Action Demos

This repository provides a Rust-only GitHub Action wrapper around the `1panel-cli` Rust binary.

The Action is a composite action. It downloads the Rust MUSL release binary, adds it to `PATH`, and runs the selected `1panel-cli` command. The deployment logic stays in Rust; the Action shell layer only maps workflow inputs to CLI arguments.

## Requirements

- Use a Linux runner for automatic release downloads.
- Publish a GitHub release tag such as `v1` or `v0.1.2` with these assets:
  - `1panel-cli-x86_64-unknown-linux-musl`
  - `1panel-cli-aarch64-unknown-linux-musl`
  - `SHA256SUMS`
- Add these repository secrets in the consuming project:
  - `ONEPANEL_BASE_URL`
  - `ONEPANEL_API_KEY`

If you use `uses: ./` in this repository, build the Rust binary first and pass `binary-path`.

## Demo 1: Static Site Deploy

Deploy `./dist` to an existing static website.

```yaml
name: Deploy Static Site

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Build site
        run: |
          npm ci
          npm run build

      - name: Deploy to 1Panel
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: deploy
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}
          path: ./dist
          domain: example.com
```

## Demo 2: Static Site Deploy and Auto Create

Create the website when it does not exist.

```yaml
name: Deploy Static Site With Auto Create

on:
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Deploy to 1Panel
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: deploy
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}
          path: ./dist
          domain: example.com
          create-if-missing: true
          group-id: 1
          alias: example-site
```

## Demo 3: Connectivity and Inventory

Check auth first, then list websites and compose files.

```yaml
name: Check 1Panel

on:
  workflow_dispatch:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - name: Server test
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: server-test
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}

      - name: List websites
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: list-websites
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}

      - name: List compose files
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: list-composes
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}
```

## Demo 4: Compose Image Dry Run

Preview a compose image update without writing the remote compose file.

```yaml
name: Preview Compose Update

on:
  workflow_dispatch:

jobs:
  preview:
    runs-on: ubuntu-latest
    steps:
      - name: Preview image change
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: deploy-compose-update
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}
          compose-path: /opt/1panel/docker/compose/wiki/docker-compose.yml
          service: wiki
          from-image: registry.example.com/wiki:v1.0.1
          to-image: registry.example.com/wiki:v1.0.2
          dry-run: true
```

## Demo 5: Compose Image Update and Apply

Update the remote compose file and run compose up.

```yaml
name: Apply Compose Update

on:
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Update compose image
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: deploy-compose-update
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}
          compose-path: /opt/1panel/docker/compose/wiki/docker-compose.yml
          service: wiki
          to-image: registry.example.com/wiki:v1.0.2
          apply: true
```

## Demo 6: Export, Upload, Load, Update Compose

Build a Docker image in GitHub Actions, export it as a tarball, upload it to 1Panel, load it on the server, update compose, and apply.

```yaml
name: Deploy Docker Image To 1Panel Compose

on:
  push:
    tags:
      - "v*"

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Build image
        run: |
          docker build -t registry.example.com/wiki:${GITHUB_REF_NAME} .

      - name: Deploy image and compose
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: deploy-all-compose
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}
          image-tag: registry.example.com/wiki:${{ github.ref_name }}
          compose-path: /opt/1panel/docker/compose/wiki/docker-compose.yml
          service: wiki
          apply: true
```

## Demo 7: Upload an Existing Image Tar

Use separate steps when you need manual control over export, upload, and load.

```yaml
name: Upload Image Tar

on:
  workflow_dispatch:

jobs:
  upload:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Export image
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: image-export
          image-tag: registry.example.com/wiki:v1.0.2
          output: /tmp/wiki-image.tar

      - name: Upload image tar
        uses: kalicyh/1Panel-rocket-cli@v1
        with:
          command: image-upload
          base-url: ${{ secrets.ONEPANEL_BASE_URL }}
          api-key: ${{ secrets.ONEPANEL_API_KEY }}
          input: /tmp/wiki-image.tar
          remote-dir: /opt/1panel/tmp
```

## Demo 8: Local Action Smoke Test

This is useful inside this repository before publishing a release.

```yaml
name: Local Action Smoke Test

on:
  pull_request:

jobs:
  smoke:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Build Rust CLI
        env:
          CARGO_TARGET_DIR: target
        run: cargo build --manifest-path rust/1panel-cli/Cargo.toml --release --locked

      - name: Locate binary
        id: bin
        shell: bash
        run: |
          set -euo pipefail
          BIN_PATH="target/release/1panel-cli"
          echo "path=$BIN_PATH" >> "$GITHUB_OUTPUT"

      - name: Run local action help
        uses: ./
        with:
          command: help
          help-command: deploy-all-compose
          binary-path: ${{ steps.bin.outputs.path }}
```

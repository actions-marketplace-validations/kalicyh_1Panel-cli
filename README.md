# 1Panel CLI

A command-line tool for deploying static websites to 1Panel server.

This project was originally adapted from [ruibaby/1Panel-rocket-cli](https://github.com/ruibaby/1Panel-rocket-cli).

[中文文档](README.zh.md)

## Features

- Deploy static websites to 1Panel server
- Automatic website creation if it doesn't exist
- File uploading with retry mechanism
- Interactive mode for selecting existing websites
- Easy integration with CI/CD workflows
- Non-interactive mode for AI agents and CI jobs
- JSON output for machine-readable automation
- Website discovery via `list-websites`
- Current 1Panel API v2 compatibility

## Installation

```bash
# Install globally with npm
npm install -g 1panel-cli

# Use the primary runtime
nvm install 24
nvm use 24

# Optional compatibility check
nvm install 25
```

## Basic Usage

```bash
# Deploy a static website
1panel-cli -p ./dist -d example.com

# List websites
1panel-cli list-websites
```

## Command Line Options

| Option | Alias | Description | Environment Variable |
|--------|-------|-------------|---------------------|
| --baseUrl | -e | Base URL of the 1Panel API | ONEPANEL_BASE_URL |
| --apiKey | -a | API key for the 1Panel API | ONEPANEL_API_KEY |
| --path | -p | Path to the static website build directory | - |
| --domain | -d | Domain name of the website | - |
| --group-id | - | Website group ID for automatic website creation | ONEPANEL_WEBSITE_GROUP_ID |
| --alias | - | Website alias to use when creating a missing website | - |
| --yes | -y | Skip all prompts and use default values | - |
| --non-interactive | - | Fail instead of prompting for input | - |
| --json | - | Print machine-readable JSON output | - |
| --create-if-missing | - | Create website automatically if missing | - |

## Examples

```bash
# Using environment variables
export ONEPANEL_BASE_URL="http://your.1panel.com"
export ONEPANEL_API_KEY="your_api_key"
1panel-cli -p ./dist -d example.com

# Using command line arguments
1panel-cli -e "http://your.1panel.com" -a "your_api_key" -p ./dist -d example.com

# Interactive mode (without specifying domain)
1panel-cli -e "http://your.1panel.com" -a "your_api_key" -p ./dist
# You will be prompted to select a website from your 1Panel server

# AI/CI-safe deploy
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
1panel-cli -p ./dist -d example.com --non-interactive --json

# AI/CI-safe deploy with website auto-creation
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
1panel-cli -p ./dist -d example.com --create-if-missing --non-interactive --json

# Pin the website group explicitly for automation
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
ONEPANEL_WEBSITE_GROUP_ID="1" \
1panel-cli -p ./dist -d example.com --create-if-missing --non-interactive --json

# Discover available websites for automation
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
1panel-cli list-websites --json

# Run directly with npx, without global install
npx --yes 1panel-cli -p ./dist -d example.com --non-interactive --json
```

## AI Automation

For AI agents and CI jobs:

- Always prefer `--non-interactive --json`
- Pass `--domain` explicitly for deploy commands
- Use `list-websites --json` before deploy if the target domain is unknown
- Use `--create-if-missing` only when automatic site creation is intended
- For deterministic creation, pass `--group-id` or `ONEPANEL_WEBSITE_GROUP_ID`

The repository also includes an agent-oriented skill:

- `skills/1panel-cli-ai/SKILL.md`

## Node Support

- Primary supported runtime: Node 24
- Verified compatible runtime: Node 25
- Recommended local default: `.nvmrc` is pinned to `24`

## API Notes

- The CLI targets the current 1Panel API base path: `/api/v2`
- Website listing prefers `GET /websites/list`
- Automatic creation uses the current `request.WebsiteCreate` payload shape
- File upload uses `POST /files/upload` with both `file` and `path`

## Configuration Options

### Ignored Files

By default, the following files and directories are ignored and will not be uploaded:

- node_modules/
- .git/
- .vscode/
- .env
- .env.local

## GitHub Actions Integration

You can easily integrate 1Panel Rocket CLI with GitHub Actions to automate the deployment of your static website.

### Example Workflow

Create a `.github/workflows/deploy.yml` file in your repository:

```yaml
name: Build and Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v6
      
      - name: Setup pnpm
        uses: pnpm/action-setup@v6
        with:
          version: 10
      
      - name: Setup Node.js
        uses: actions/setup-node@v6
        with:
          node-version: '24'
          cache: 'pnpm'
      
      - name: Install dependencies
        run: pnpm install
      
      - name: Build
        run: pnpm build
      
      - name: Deploy to 1Panel
        env:
          ONEPANEL_BASE_URL: ${{ secrets.ONEPANEL_BASE_URL }}
          ONEPANEL_API_KEY: ${{ secrets.ONEPANEL_API_KEY }}
        run: |
          npx --yes 1panel-cli -p ./dist -d example.com --non-interactive --json
```

### Secrets Configuration

Make sure to add these secrets in your GitHub repository:

1. Go to your repository → Settings → Secrets and variables → Actions
2. Add the following secrets:
   - `ONEPANEL_BASE_URL`: The URL of your 1Panel server
   - `ONEPANEL_API_KEY`: Your 1Panel API key

## Development

### Setup

```bash
# Clone the repository
git clone https://github.com/ruibaby/1panel-rocket-cli.git
cd 1panel-rocket-cli

# Install dependencies
nvm use
pnpm install
```

### Local Development

```bash
# Link the package locally
npm link

# Run in development mode
1panel-cli -p ./dist -d example.com
```

## License

MIT

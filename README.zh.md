# 1Panel CLI

一个用于将静态网站部署到 1Panel 面板的命令行工具。

本项目最初基于 [ruibaby/1Panel-rocket-cli](https://github.com/ruibaby/1Panel-rocket-cli) 演进而来。

[English Documentation](README.md)

## 功能特点

- 将静态网站部署到 1Panel 服务器
- 如果网站不存在，自动创建
- 带有重试机制的文件上传
- 用于选择现有网站的交互模式
- 易于与 CI/CD 工作流集成
- 支持 AI 代理和 CI 的非交互模式
- 支持机器可读的 JSON 输出
- 支持通过 `list-websites` 发现网站
- 兼容当前 1Panel API v2

## 安装

```bash
# 使用 npm 全局安装
npm install -g 1panel-cli

# 使用主推荐运行时
nvm install 24
nvm use 24

# 可选兼容性验证
nvm install 25
```

## 基本用法

```bash
# 部署静态网站
1panel-cli -p ./dist -d example.com

# 列出已有网站
1panel-cli list-websites
```

## 命令行选项

| 选项 | 简写 | 描述 | 环境变量 |
|------|------|------|---------|
| --baseUrl | -e | 1Panel API 的基础 URL | ONEPANEL_BASE_URL |
| --apiKey | -a | 1Panel API 的 API 密钥 | ONEPANEL_API_KEY |
| --path | -p | 静态网站构建目录的路径 | - |
| --domain | -d | 网站的域名 | - |
| --group-id | - | 自动建站时指定网站分组 ID | ONEPANEL_WEBSITE_GROUP_ID |
| --alias | - | 自动建站时指定网站别名 | - |
| --yes | -y | 跳过所有提示并使用默认值 | - |
| --non-interactive | - | 遇到需要输入时直接失败而不是交互提示 | - |
| --json | - | 输出机器可读的 JSON | - |
| --create-if-missing | - | 网站不存在时自动创建 | - |

## 使用示例

```bash
# 使用环境变量
export ONEPANEL_BASE_URL="http://your.1panel.com"
export ONEPANEL_API_KEY="your_api_key"
1panel-cli -p ./dist -d example.com

# 使用命令行参数
1panel-cli -e "http://your.1panel.com" -a "your_api_key" -p ./dist -d example.com

# 交互式模式（不指定域名）
1panel-cli -e "http://your.1panel.com" -a "your_api_key" -p ./dist
# 系统将提示您从 1Panel 服务器中选择一个网站

# 适合 AI/CI 的部署方式
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
1panel-cli -p ./dist -d example.com --non-interactive --json

# 适合 AI/CI 的自动建站部署
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
1panel-cli -p ./dist -d example.com --create-if-missing --non-interactive --json

# 在自动化里显式指定网站分组
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
ONEPANEL_WEBSITE_GROUP_ID="1" \
1panel-cli -p ./dist -d example.com --create-if-missing --non-interactive --json

# 自动化前先列出网站
ONEPANEL_BASE_URL="http://your.1panel.com" \
ONEPANEL_API_KEY="your_api_key" \
1panel-cli list-websites --json

# 不全局安装，直接用 npx 运行
npx --yes 1panel-cli -p ./dist -d example.com --non-interactive --json
```

## AI 自动化使用说明

对于 AI 代理和 CI：

- 默认优先使用 `--non-interactive --json`
- 部署时显式传入 `--domain`
- 如果还不知道目标域名，先执行 `list-websites --json`
- 只有在允许自动创建站点时才使用 `--create-if-missing`
- 如果希望自动建站行为稳定可预测，传入 `--group-id` 或 `ONEPANEL_WEBSITE_GROUP_ID`

仓库内也附带了一个给代理使用的 skill：

- `skills/1panel-cli-ai/SKILL.md`

## Node 版本支持

- 主支持版本：Node 24
- 已验证兼容：Node 25
- 推荐本地默认版本：`.nvmrc` 固定为 `24`

## API 说明

- CLI 现在对接当前 1Panel API 基础路径 `/api/v2`
- 网站列表优先使用 `GET /websites/list`
- 自动建站使用当前 `request.WebsiteCreate` 请求体结构
- 文件上传使用 `POST /files/upload`，同时提交 `file` 和 `path`

## 配置选项

### 忽略文件

默认情况下，以下文件和目录将被忽略，不会上传：

- node_modules/
- .git/
- .vscode/
- .env
- .env.local

## GitHub Actions 集成

您可以轻松地将 1Panel CLI 与 GitHub Actions 集成，以自动部署您的静态网站。

### 工作流示例

在您的代码仓库中创建 `.github/workflows/deploy.yml` 文件：

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

### 配置密钥

确保在您的 GitHub 仓库中添加这些密钥：

1. 进入您的仓库 → Settings → Secrets and variables → Actions
2. 添加以下密钥：
   - `ONEPANEL_BASE_URL`：您的 1Panel 服务器的 URL
   - `ONEPANEL_API_KEY`：您的 1Panel API 密钥

## 开发

### 设置环境

```bash
# 克隆仓库
git clone https://github.com/ruibaby/1panel-rocket-cli.git
cd 1panel-rocket-cli

# 安装依赖
nvm use
pnpm install
```

### 本地开发

```bash
# 链接包到本地
npm link

# 在开发模式下运行
1panel-cli -p ./dist -d example.com
```

## 许可证

MIT

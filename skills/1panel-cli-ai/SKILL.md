---
name: 1panel-cli-ai
description: Use when an AI agent needs to deploy a static site to 1Panel, inspect available websites, or automate 1Panel static-site publishing with non-interactive CLI commands and JSON output.
---

# 1Panel CLI AI

Use this skill when operating `1panel-cli` as an automation-safe tool instead of a human-facing terminal app.

## Preferred commands

List available websites:

```bash
ONEPANEL_BASE_URL=... ONEPANEL_API_KEY=... 1panel-cli list-websites --json
```

Deploy a build directory to an existing website:

```bash
ONEPANEL_BASE_URL=... ONEPANEL_API_KEY=... 1panel-cli -p ./dist -d example.com --non-interactive --json
```

Deploy and create the website if missing:

```bash
ONEPANEL_BASE_URL=... ONEPANEL_API_KEY=... 1panel-cli -p ./dist -d example.com --create-if-missing --non-interactive --json
```

Deploy and create the website in a specific website group:

```bash
ONEPANEL_BASE_URL=... ONEPANEL_API_KEY=... ONEPANEL_WEBSITE_GROUP_ID=1 1panel-cli -p ./dist -d example.com --create-if-missing --non-interactive --json
```

## Rules

- Prefer `--json` for any agent-run invocation so results and failures are machine-readable.
- Prefer `--non-interactive` whenever no human is present to answer prompts.
- Provide `--domain` explicitly for deploys. Do not rely on interactive site selection.
- Use `list-websites --json` first when the target domain is unknown.
- Use `--create-if-missing` only when the workflow is allowed to create new 1Panel static sites.
- Prefer `--group-id` or `ONEPANEL_WEBSITE_GROUP_ID` when deterministic website creation matters.

## Expected outputs

- Successful deploy returns JSON with `ok: true`, `domain`, `created`, upload counts, `sitePath`, and final `url`.
- Failed commands return JSON with `ok: false` and an `error` string when `--json` is set.

## Environment

- `ONEPANEL_BASE_URL`: 1Panel base URL
- `ONEPANEL_API_KEY`: 1Panel API key
- `ONEPANEL_WEBSITE_GROUP_ID`: optional website group for automatic creation

## Node runtime

- Primary supported runtime: Node 24
- Compatibility target: Node 25 when smoke tests pass

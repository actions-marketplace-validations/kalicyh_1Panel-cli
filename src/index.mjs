#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Command } from "commander";
import prompts from "prompts";
import OnePanelAPI from "./api.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const packageJsonPath = path.join(__dirname, "..", "package.json");
const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));

function isInteractiveTerminal() {
  return Boolean(process.stdin.isTTY && process.stdout.isTTY);
}

function printOutput(payload, asJson) {
  if (asJson) {
    console.log(JSON.stringify(payload, null, 2));
    return;
  }

  if (payload.message) {
    console.log(payload.message);
  }
}

function formatWebsiteChoices(websites) {
  return websites.map((website) => ({
    title: website.primaryDomain,
    value: website.primaryDomain,
  }));
}

async function resolveDomain({ api, domain, nonInteractive }) {
  if (domain) {
    return domain;
  }

  const websites = await api.getWebsiteList();

  if (websites.length === 0) {
    throw new Error("No websites found");
  }

  if (nonInteractive) {
    throw new Error(
      `Domain is required in non-interactive mode. Available domains: ${websites.map((website) => website.primaryDomain).join(", ")}`,
    );
  }

  const answer = await prompts({
    type: "select",
    name: "value",
    message: "Select a website",
    choices: formatWebsiteChoices(websites),
  });

  if (!answer.value) {
    throw new Error("No website selected");
  }

  return answer.value;
}

async function resolveWebsite({ api, domain, createIfMissing, nonInteractive, alias, groupId }) {
  let website = await api.getWebsiteDetail(domain);

  if (website) {
    return { website, created: false };
  }

  if (createIfMissing) {
    website = await api.createWebsite({ domain, alias, groupId });
    return { website, created: true };
  }

  if (nonInteractive) {
    throw new Error(`Website not found: ${domain}`);
  }

  const answer = await prompts({
    type: "confirm",
    name: "value",
    message: "Website not found, create it? (y/n)",
  });

  if (!answer.value) {
    throw new Error("Website not found");
  }

  website = await api.createWebsite({ domain, alias, groupId });
  return { website, created: true };
}

const program = new Command();

program
  .name("1panel-cli")
  .version(packageJson.version)
  .description("Deploy static websites to 1Panel with AI-friendly automation support.")
  .option(
    "-e, --baseUrl <baseUrl>",
    "Base URL of the 1Panel API(You can also use environment variable: ONEPANEL_BASE_URL)",
  )
  .option("-a, --apiKey <apiKey>", "API key of the 1Panel API(You can also use environment variable: ONEPANEL_API_KEY)")
  .option("-p, --path <path>", "Path to the static website build directory")
  .option("-d, --domain <domain>", "Domain name of the website")
  .option("--group-id <groupId>", "Website group ID for automatic website creation")
  .option("--alias <alias>", "Website alias to use when creating a missing website")
  .option("-y, --yes", "Skip prompts and create the website automatically if it does not exist")
  .option("--non-interactive", "Fail instead of prompting for input")
  .option("--json", "Print machine-readable JSON output")
  .option("--create-if-missing", "Create the website automatically if it does not exist")
  .action(async (options) => {
    const {
      path: buildPath,
      baseUrl,
      apiKey,
      alias,
      groupId,
      yes,
      nonInteractive: nonInteractiveFlag,
      json,
      createIfMissing,
    } = options;
    const nonInteractive = Boolean(nonInteractiveFlag || yes || json || !isInteractiveTerminal());
    const shouldCreateIfMissing = Boolean(createIfMissing || yes);
    let { domain } = options;

    if (!buildPath) {
      throw new Error("Build directory path is required");
    }

    if (!fs.existsSync(buildPath)) {
      throw new Error(`Build directory ${buildPath} does not exist`);
    }

    const finalBaseUrl = baseUrl || process.env.ONEPANEL_BASE_URL;
    const finalApiKey = apiKey || process.env.ONEPANEL_API_KEY;

    if (!finalBaseUrl || !finalApiKey) {
      throw new Error("Base URL and API key are required");
    }

    const api = new OnePanelAPI({
      baseURL: finalBaseUrl,
      apiKey: finalApiKey,
    });

    domain = await resolveDomain({
      api,
      domain,
      nonInteractive,
    });

    const { website, created } = await resolveWebsite({
      api,
      domain,
      createIfMissing: shouldCreateIfMissing,
      nonInteractive,
      alias,
      groupId,
    });

    const uploadResult = await api.uploadStaticFiles(domain, buildPath);
    const payload = {
      ok: true,
      action: "deploy",
      domain,
      websiteId: website?.id ?? null,
      created,
      groupId: website?.webSiteGroupId ?? (Number(groupId || process.env.ONEPANEL_WEBSITE_GROUP_ID || 0) || null),
      sourcePath: path.resolve(buildPath),
      uploaded: uploadResult.successCount,
      failed: uploadResult.failCount,
      total: uploadResult.totalFiles,
      sitePath: website?.sitePath ?? null,
      url: `https://${domain}`,
      message: [
        "Deployment completed successfully.",
        `Website: https://${domain}`,
        `Files uploaded: ${uploadResult.successCount}/${uploadResult.totalFiles}`,
      ].join("\n"),
    };

    printOutput(payload, json);
  });

program
  .command("list-websites")
  .description("List websites available in 1Panel")
  .option(
    "-e, --baseUrl <baseUrl>",
    "Base URL of the 1Panel API(You can also use environment variable: ONEPANEL_BASE_URL)",
  )
  .option("-a, --apiKey <apiKey>", "API key of the 1Panel API(You can also use environment variable: ONEPANEL_API_KEY)")
  .option("--json", "Print machine-readable JSON output")
  .action(async (options) => {
    const finalBaseUrl = options.baseUrl || process.env.ONEPANEL_BASE_URL;
    const finalApiKey = options.apiKey || process.env.ONEPANEL_API_KEY;

    if (!finalBaseUrl || !finalApiKey) {
      throw new Error("Base URL and API key are required");
    }

    const api = new OnePanelAPI({
      baseURL: finalBaseUrl,
      apiKey: finalApiKey,
    });

    const websites = await api.getWebsiteList();
    const payload = {
      ok: true,
      action: "list-websites",
      count: websites.length,
      websites: websites.map((website) => ({
        id: website.id ?? null,
        domain: website.primaryDomain,
        alias: website.alias ?? null,
        groupId: website.webSiteGroupId ?? null,
        sitePath: website.sitePath ?? null,
        status: website.status ?? null,
      })),
      message:
        websites.length === 0 ? "No websites found." : websites.map((website) => website.primaryDomain).join("\n"),
    };

    printOutput(payload, options.json);
  });

program.parseAsync(process.argv).catch((error) => {
  const outputAsJson = process.argv.includes("--json");
  const payload = {
    ok: false,
    error: error.message,
  };

  if (outputAsJson) {
    console.error(JSON.stringify(payload, null, 2));
  } else {
    console.error(error.message);
  }

  process.exit(1);
});

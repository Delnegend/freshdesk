// Regenerates the product-list entries in `.env` from the live Freshdesk API.
//
// The repo ships no product names, so a maintainer who needs the real catalog
// for their own account runs this to (re)populate `FD_BASELINE_COMPONENTS` and
// `FD_DEFAULT_IN_CHARGE_PRODUCTS` in `.env`.
//
//   bun run scripts/sync-products.ts            # rewrite the two vars in .env
//   bun run scripts/sync-products.ts -- --print # also just print the catalog
//
// Requires FD_SERVER (and either a session in .freshdesk_session.json or
// FD_API_KEY) to be configured. The CLI itself is the source of truth: this
// shells out to `freshdesk components --json` rather than re-implementing auth.

import { existsSync, readFileSync, writeFileSync } from "fs";
import { resolve } from "path";

const ENV_PATH = resolve(process.cwd(), ".env");
const printOnly = process.argv.includes("--print");

if (!existsSync(ENV_PATH)) {
  console.error("No .env found at", ENV_PATH);
  process.exit(1);
}

// Reuse the same credentials the CLI reads, so this stays tenant-agnostic.
const envText = readFileSync(ENV_PATH, "utf8");
const hasServer = envText.split("\n").some((l) => /^FD_SERVER=\S/.test(l.trim()));
if (!hasServer) {
  console.error("FD_SERVER is not set in .env; cannot query the API.");
  process.exit(1);
}

// Ask the CLI for the live catalog. It resolves auth, domain, and the
// canonical _Components field for us.
const proc = Bun.spawnSync(["cargo", "run", "--release", "--quiet", "--", "components", "--json"], {
  stdout: "pipe",
  stderr: "pipe",
});

if (proc.exitCode !== 0) {
  console.error("`freshdesk components --json` failed:");
  console.error(proc.stderr.toString());
  process.exit(proc.exitCode ?? 1);
}

let products: string[];
try {
  products = JSON.parse(proc.stdout.toString());
} catch {
  console.error("Could not parse catalog output as JSON.");
  process.exit(1);
}

if (!Array.isArray(products) || products.length === 0) {
  console.error("Catalog came back empty; refusing to overwrite .env.");
  process.exit(1);
}

const list = products.join(",");
console.log(`Fetched ${products.length} products from Freshdesk.`);
if (printOnly) {
  console.log(list);
  process.exit(0);
}

// Quote the value: dotenvy (and most .env parsers) silently drop unquoted
// values that contain spaces, e.g. multi-word product names.
const quoted = `"${list}"`;

// Replace the two managed keys in place, or append them if absent.
function upsert(text: string, key: string, value: string): string {
  const line = `${key}=${value}`;
  const re = new RegExp(`^${key}=.*$`, "m");
  if (re.test(text)) {
    return text.replace(re, line);
  }
  return `${text.replace(/\n*$/, "\n")}\n${line}\n`;
}

let updated = upsert(envText, "FD_BASELINE_COMPONENTS", quoted);
updated = upsert(updated, "FD_DEFAULT_IN_CHARGE_PRODUCTS", quoted);

writeFileSync(ENV_PATH, updated);
console.log("Updated FD_BASELINE_COMPONENTS and FD_DEFAULT_IN_CHARGE_PRODUCTS in .env");
console.log("Note: FD_DEFAULT_IN_CHARGE_PRODUCTS now lists every product; trim it to the ones you manage.");

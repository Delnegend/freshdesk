import { chromium } from "playwright";
import { readFileSync, writeFileSync, existsSync } from "fs";
import { createHmac } from "crypto";
import { resolve } from "path";
import { $ } from "bun";

// Load .env
const envPath = resolve(process.cwd(), ".env");
if (!existsSync(envPath)) {
  console.error("No .env file found at", envPath);
  process.exit(1);
}

const envFile = readFileSync(envPath, "utf8");
const env: Record<string, string> = {};
for (const line of envFile.split("\n")) {
  const trimmed = line.trim();
  if (!trimmed || trimmed.startsWith("#")) continue;
  const eq = trimmed.indexOf("=");
  if (eq !== -1) {
    env[trimmed.slice(0, eq).trim()] = trimmed.slice(eq + 1).trim();
  }
}

const server = env.FD_SERVER || "care.your-account.freshdesk.com";
const user = env.FD_USER;
const password = env.FD_PASSWORD;
const totpSeed = env.FD_TOTP_SEED;

if (!user || !password) {
  console.error("FD_USER and FD_PASSWORD must be set in .env");
  process.exit(1);
}

function base32Decode(base32: string): Buffer {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
  let bits = 0;
  let value = 0;
  const output: number[] = [];

  for (let i = 0; i < base32.length; i++) {
    const char = base32[i].toUpperCase();
    if (char === "=") break;
    const val = alphabet.indexOf(char);
    if (val === -1) continue;
    value = (value << 5) | val;
    bits += 5;
    if (bits >= 8) {
      output.push((value >>> (bits - 8)) & 255);
      bits -= 8;
    }
  }
  return Buffer.from(output);
}

function generateTOTP(secretBase32: string): string {
  const key = base32Decode(secretBase32);
  const epoch = Math.floor(Date.now() / 1000);
  const counter = Math.floor(epoch / 30);
  const buf = Buffer.alloc(8);
  buf.writeBigInt64BE(BigInt(counter));
  const hmac = createHmac("sha1", key).update(buf).digest();
  const offset = hmac[hmac.length - 1] & 0xf;
  const binary =
    ((hmac[offset] & 0x7f) << 24) |
    ((hmac[offset + 1] & 0xff) << 16) |
    ((hmac[offset + 2] & 0xff) << 8) |
    (hmac[offset + 3] & 0xff);
  return (binary % 1000000).toString().padStart(6, "0");
}


console.log(`[login] Logging in as agent ${user} to ${server}...`);

const browser = await chromium.launch({
  headless: false,
  args: [
    "--disable-blink-features=AutomationControlled",
    "--no-sandbox",
    "--disable-setuid-sandbox",
  ],
});

const context = await browser.newContext({
  userAgent:
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
});

const page = await context.newPage();

const loginUrl = `https://${server}/agent/login`;
console.log(`[login] Navigating to ${loginUrl}...`);
await page.goto(loginUrl, { waitUntil: "networkidle" });

await page.waitForSelector("#username");
await page.fill("#username", user);
await page.fill("#password", password);

console.log("[login] Submitting credentials...");
await page.locator("[data-testid=\"login-button\"]").dispatchEvent("mousedown");
await page.locator("[data-testid=\"login-button\"]").dispatchEvent("click");

await page.waitForTimeout(3000);

// Check if reCAPTCHA challenge appears
const bframe = page.frames().find((f) => f.url().includes("bframe"));
if (bframe) {
  console.log("[login] Solving reCAPTCHA audio challenge...");
  const audioBtn = bframe.locator("#recaptcha-audio-button");
  if ((await audioBtn.count()) > 0) {
    await audioBtn.click();
    await page.waitForTimeout(2000);

    const audioEl = bframe.locator("#audio-source, audio source, audio");
    const src = await audioEl.first().getAttribute("src");

    if (src) {
      const audioResp = await fetch(src);
      const buf = await audioResp.arrayBuffer();
      const tmpMp3 = "/tmp/recaptcha_challenge.mp3";
      const tmpWav = "/tmp/recaptcha_challenge.wav";
      writeFileSync(tmpMp3, Buffer.from(buf));

      // Use Bun shell `$` to convert MP3 to WAV
      await $`ffmpeg -y -i ${tmpMp3} -ar 16000 -ac 1 ${tmpWav}`.quiet();

      // Use Bun shell `$` to transcribe with whisper-cli
      const whisperBin = existsSync("/usr/local/bin/whisper-cli")
        ? "/usr/local/bin/whisper-cli"
        : existsSync("/tmp/whisper.cpp/build/bin/whisper-cli")
          ? "/tmp/whisper.cpp/build/bin/whisper-cli"
          : "whisper-cli";
      const modelPath = existsSync("/usr/local/share/whisper-models/ggml-tiny.en.bin")
        ? "/usr/local/share/whisper-models/ggml-tiny.en.bin"
        : existsSync("/tmp/whisper.cpp/models/ggml-tiny.en.bin")
          ? "/tmp/whisper.cpp/models/ggml-tiny.en.bin"
          : "ggml-tiny.en.bin";
      const whisperResult = await $`${whisperBin} -m ${modelPath} -f ${tmpWav} --no-timestamps`.quiet().text();
      const transcribed = whisperResult.trim();
      console.log(`[login] Audio transcribed: "${transcribed}"`);

      const respInput = bframe.locator("#audio-response");
      await respInput.fill(transcribed);
      await bframe.locator("#recaptcha-verify-button").click();
      await page.waitForTimeout(3000);
    }
  }
}

// Check for TOTP / MFA
await page.waitForTimeout(3000);
const totpInputs = await page.locator("input[name*=\"mfa\"], input[name*=\"totp\"], input[name*=\"otp\"], input[type=\"text\"]:visible, input[type=\"tel\"]:visible").all();

if (page.url().includes("/login/mfa") || totpInputs.length > 0) {
  if (!totpSeed) {
    console.error("[login] TOTP required but FD_TOTP_SEED not provided in .env");
    await browser.close();
    process.exit(1);
  }
  const totpCode = generateTOTP(totpSeed);
  console.log(`[login] Entering 2FA TOTP code: ${totpCode}...`);

  if (totpInputs.length > 0) {
    await totpInputs[0].fill(totpCode);
    const submitBtn = page.locator("button[type=\"submit\"], button:has-text(\"Verify\"), button:has-text(\"Submit\"), button:has-text(\"Sign in\")");
    if ((await submitBtn.count()) > 0) {
      await submitBtn.first().click();
    } else {
      await totpInputs[0].press("Enter");
    }
    await page.waitForTimeout(6000);
  }
}

// Wait for final navigation
await page.waitForTimeout(3000);
console.log(`[login] Final URL reached: ${page.url()}`);

const cookies = await context.cookies();
await browser.close();

// Filter cookies for Freshdesk / your-account.freshdesk.com
const relevantCookies = cookies.filter(
  (c) => c.domain.includes("freshdesk.com") || c.domain.includes("your-account.freshdesk.com")
);

const cookieMap: Record<string, string> = {};
for (const c of relevantCookies) {
  cookieMap[c.name] = c.value;
}

const cookieHeader = Object.entries(cookieMap)
  .map(([k, v]) => `${k}=${v}`)
  .join("; ");

const sessionData = {
  server,
  user,
  created_at: new Date().toISOString(),
  cookie_header: cookieHeader,
  cookies: relevantCookies,
};

const sessionFile = resolve(process.cwd(), ".freshdesk_session.json");
writeFileSync(sessionFile, JSON.stringify(sessionData, null, 2));
console.log(`[login] Session cookies successfully saved to ${sessionFile}`);

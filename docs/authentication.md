# Authentication Guide

This document explains the authentication flows supported by the Freshdesk client and the automated login engine.

## Authentication Methods

```
                                  +---------------------+
                                  | FreshdeskClient     |
                                  +----------+----------+
                                             |
                     +-----------------------+-----------------------+
                     |                       |                       |
                     v                       v                       v
             [ AuthMethod::ApiKey ]  [ Session Cookie Env ]   [ Session Cache File ]
                     |                       |                       |
                     |                       |                       v
                     |                       |              .freshdesk_session.json
                     |                       |                       | (if missing / expired)
                     |                       |                       v
                     |                       |              scripts/login.ts (Bun)
                     |                       |                       |
                     +-----------------------+-----------------------+
                                             |
                                             v
                           Freshdesk REST API (v2 /api/v2/*)
```

---

## 1. Automated Agent Login (`scripts/login.ts`)

In instances where agents are not granted administrative permissions to create permanent API keys, access requires an authenticated session. The login helper automates this end-to-end:

1. **Agent Login Navigation**:
   - Opens `https://<server>/agent/login`.
   - Follows redirect to Freshworks Organization sign-in (`/org/login`).

2. **reCAPTCHA Invisible Bypass**:
   - The sign-in form triggers Google reCAPTCHA v2.
   - If an invisible challenge triggers, the script clicks `#recaptcha-audio-button` in the `bframe` iframe.
   - Downloads the challenge MP3 payload and uses `ffmpeg` to resample to 16 kHz WAV:
     ```typescript
     await $`ffmpeg -y -i ${tmpMp3} -ar 16000 -ac 1 ${tmpWav}`.quiet();
     ```
   - Uses `whisper-cli` with `ggml-tiny.en.bin` to transcribe the audio prompt locally in < 300ms:
     ```typescript
     const res = await $`${whisperBin} -m ${modelPath} -f ${tmpWav} --no-timestamps`.quiet().text();
     ```
   - Enters the transcription into `#audio-response` and clicks `#recaptcha-verify-button`.

3. **Two-Factor Authentication (TOTP)**:
   - On the MFA verification page (`/org/login/mfa/auth-code`), computes the 6-digit TOTP code from `FD_TOTP_SEED` (RFC 6238 HMAC-SHA1).
   - Enters the code and submits.

4. **Cookie Persistence**:
   - Upon completing the OAuth callback to `https://<server>/freshid/authorize_callback`, saves session cookies (`_helpkit_session`, `user_credentials`, `session_token`, etc.) to `.freshdesk_session.json`.

---

## 2. API Key Authentication

For accounts with API access:
```bash
export FD_API_KEY="your_api_key"
```
Or via CLI flag:
```bash
cargo run -- --api-key "your_api_key" list
```

---

## 3. Direct Session Cookie

To manually provide a cookie string:
```bash
export FD_SESSION_COOKIE="_helpkit_session=...; user_credentials=...; session_token=..."
```

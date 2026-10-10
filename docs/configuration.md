# Configuration

Copy the template and point it at your account. Unset values keep working;
there are no compiled-in tenant defaults.

```bash
cp .env.example .env
```

| Variable | Required | Default | Description |
|---|---|---|---|
| `FD_SERVER` | **yes** | — | Your Freshdesk account domain or URL; custom portal domains auto-resolve |
| `FD_USER` | for login | — | Agent email for the automated login |
| `FD_PASSWORD` | for login | — | Agent password |
| `FD_TOTP_SEED` | for login | — | Base32 TOTP seed for the 2FA step |
| `FD_API_KEY` | no | — | API key auth; overrides the session cookie when set |
| `FD_SESSION_COOKIE` | no | — | Existing session cookie; skips login entirely |
| `FD_BASELINE_COMPONENTS` | for catalog checks | — | Expected `_Components` catalog, comma-separated and quoted |
| `FD_DEFAULT_IN_CHARGE_PRODUCTS` | for reports | — | Report checkboxes defaulting to `TRUE`, comma-separated and quoted |

Every variable is also a CLI flag (`--server`, `--api-key`,
`--session-file`); the flag wins, then the file.
```bash
bun run scripts/sync-products.ts            # rewrite the two keys in .env
bun run scripts/sync-products.ts -- --print # just print the catalog
```

Tenant rule worth knowing: quote any value containing spaces. The parser
silently drops unquoted values with spaces in them, so `FD_BASELINE_COMPONENTS`
and `FD_DEFAULT_IN_CHARGE_PRODUCTS` must be wrapped in double quotes.

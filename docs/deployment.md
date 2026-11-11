# Deployment & Operations

## Installation

### Homebrew (Recommended)

Install the CLI via the official tap:

```bash
brew install Delnegend/tap/freshdesk
```

### Prebuilt Release Binaries

Download prebuilt Linux x86_64 binaries directly from GitHub Releases:

```bash
URL=$(curl -fsSL https://api.github.com/repos/Delnegend/freshdesk/releases/latest | grep -o 'https://[^"]*x86_64-unknown-linux-gnu\.tar\.xz')
curl -fsSL -o freshdesk.tar.xz "$URL" && tar -xJf freshdesk.tar.xz --strip-components=1
```

## Configuration

Freshdesk reads credentials from environment variables or a local `.env` file:

```bash
cp .env.example .env
```

Key variables:
- `FD_SERVER`: Freshdesk portal or custom CNAME URL.
- `FD_USER` / `FD_PASSWORD` / `FD_TOTP_SEED`: Automated agent credentials with 2FA.
- `FD_API_KEY`: API key authentication alternative.

See [docs/configuration.md](configuration.md) for full configuration reference.

## Cron & Daemon Usage

### Automated Reports Generation

Run scheduled quarterly reports using cron:

```cron
# Run on the 1st day of every quarter at 08:00 UTC
0 8 1 1,4,7,10 * /usr/local/bin/freshdesk report --output /var/reports/quarterly.xlsx
```

### Headless Agent Token Refresh

Run a scheduled job to refresh session cookies before expiration:

```bash
# Refresh session and verify open tickets
freshdesk login && freshdesk list --filter new-and-my-open --per-page 10
```

## Upgrades

Keep the installation up to date using Homebrew:

```bash
brew upgrade Delnegend/tap/freshdesk
```

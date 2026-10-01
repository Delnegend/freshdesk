# CI/CD & Autonomous Upgrades

This repository implements the `autonomous-upgrade` workflow architecture for dependency updates, continuous verification, and manual release triggering.

## Workflows Overview

```
+-----------------------------------------------------------+
| Dependabot (Daily 02:00 UTC)                              |
| - 14-day Supply-Chain Quarantine Cooldown                 |
| - Groups: patch-and-minor (auto-merge), major (review)    |
+-----------------------------+-----------------------------+
                              | (PR created)
                              v
+-----------------------------------------------------------+
| CI Gate: .github/workflows/ci.yml                         |
| - Check: 'Check (just check)'                             |
| - Executes `just check` (fmt, clippy, unit tests, types)  |
+-----------------------------+-----------------------------+
                              | (Green status)
                              v
+-----------------------------------------------------------+
| Dependabot Auto-Merge: .github/workflows/dependabot-...   |
| - `gh pr merge --auto --rebase`                           |
| - Merges patch/minor immediately                          |
+-----------------------------------------------------------+

                              [Accumulate on main]
                                      |
                                      | (User clicks "Run workflow")
                                      v
+-----------------------------------------------------------+
| Release: .github/workflows/release.yml                    |
| - Trigger: workflow_dispatch                              |
| - ietf-tools/semver-action derives version from commits   |
| - Compiles release binary: freshdesk-linux-x64.tar.gz     |
| - Publishes GitHub Release with generated release notes   |
+-----------------------------------------------------------+
```

---

## 1. Single CI Gate (`just check`)

- **Workflow**: `.github/workflows/ci.yml`
- **Check Name**: `Check (just check)`
- Runs the single command `just check`:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --test client_tests`
  - `bun x tsc --noEmit`

---

## 2. Dependabot Configuration (`.github/dependabot.yml`)

- Runs daily at `02:00 UTC`.
- Covers `cargo` and `github-actions`.
- Does **not** cover `bun`: Dependabot's `bun` updater image ships Bun 1.3.x, which cannot parse the `lockfileVersion: 2` lockfile that Bun 1.4+ writes, so every run fails with `Unsupported bun.lock 'lockfileVersion' 2`. This is blocked upstream on [dependabot-core#15897](https://github.com/dependabot/dependabot-core/issues/15897) (per-repository Bun versions in the updater image). Re-add the ecosystem in one line once that ships. See [dependabot-core#15848](https://github.com/dependabot/dependabot-core/issues/15848) for the original silent-lockfile-downgrade report.
- Enforces `cooldown: default-days: 14` to quarantine newly published dependency versions for two weeks against supply-chain attacks (account takeovers, poisoned point-releases).
- Isolates `patch-and-minor` updates from `major` breaking changes.

---

## 3. Auto-Merge (`.github/workflows/dependabot-auto-merge.yml`)

- Uses `gh pr merge --auto --rebase`.
- Merges patch and minor dependency PRs automatically upon passing `just check`.
- Leaves major breaking version upgrades open for human inspection.

---

## 4. Manual Release (`.github/workflows/release.yml`)

- **Trigger**: `workflow_dispatch` (manual button press).
- **Concurrency**: `group: release-main`, `cancel-in-progress: false`.
- **Version Calculation**: `ietf-tools/semver-action@v1` determines the next semantic version based on Conventional Commits since the last git tag (with fallback `v0.0.0`).
- **Binary Packaging**: Compiles `target/release/freshdesk` and packages it into `dist/freshdesk-linux-x64.tar.gz`.
- **Publishing**: `softprops/action-gh-release@v2` publishes the GitHub release with auto-generated release notes and attached binary asset.

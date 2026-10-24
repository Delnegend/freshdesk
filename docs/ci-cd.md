# CI/CD & Autonomous Upgrades

This repository implements the `ci-cd` workflow architecture for dependency updates, continuous verification, and release automation (manual dispatch or weekly schedule), running in maintenance mode.

## Workflows Overview

```
+-----------------------------------------------------------+
| Dependency PR sources                                     |
| - deps.yml: daily 02:00 UTC `cargo update` / `bun update` |
|    14-day cooldown via .cargo/config.toml & bunfig.toml   |
| - dependabot.yml: github-actions, weekly Monday 02:00     |
| Both open PRs labelled `dependencies`                     |
+-----------------------------------------------------------+
                              | (PR created)
                              v
+-----------------------------------------------------------+
| CI Gate: .github/workflows/ci.yml                         |
| - Check: 'Check (just check)'                             |
| - Executes `just check` (fmt, clippy, unit tests, types)  |
+-----------------------------------------------------------+
                              | (Green status)
                              v
+-----------------------------------------------------------+
| Auto-merge: .github/workflows/auto-merge.yml              |
| - `gh pr merge --auto --rebase`                           |
| - Merges any PR labelled `dependencies`                   |
+-----------------------------------------------------------+

                            [Accumulate on main]
                                      | (Weekly Sunday 00:00 UTC, or manual dispatch)
                                      v
+-----------------------------------------------------------+
| Release: .github/workflows/release.yml                    |
| - Triggers: workflow_dispatch (ref, dry-run) + cron       |
|   and weekly cron 0 0 * * 0 (Sunday 00:00 UTC)            |
| - ietf-tools/semver-action derives version from commits   |
| - Compiles release binary: freshdesk-<version>-x86_64-unknown-linux-gnu.tar.xz |
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

## 2. Dependency Updates

### 2.1 Automated upgrades (`.github/workflows/deps.yml`)

- **Trigger**: daily `0 2 * * *` (02:00 UTC), plus `workflow_dispatch`.
- Runs `cargo update` and `bun update`, so each package manager applies its own release cooldown rather than a workflow-level gate.
- **Cooldown**: 14 days, enforced by `.cargo/config.toml` (`global-min-publish-age = "14 days"`, requires Cargo ≥ 1.100) and `bunfig.toml` (`minimumReleaseAge = 1209600`). Versions published less than two weeks ago are skipped.
- Opens — or force-pushes — a single `deps/automatic` PR labelled `dependencies`, titled `chore(deps): upgrade dependencies`. A run with no changes opens nothing.

### 2.2 Dependabot (`.github/dependabot.yml`)

- Retained **only** for the `github-actions` ecosystem; `cargo` and `bun` are handled by `deps.yml` instead.
- Runs weekly, Monday at `02:00 UTC`.
- Labels its PRs `dependencies`, so they flow through the same CI gate and auto-merge path.
- Groups updates into `patch-and-minor` and `major`, caps open PRs at `2`, and prefixes commits with `chore(deps)`.

---

## 3. Auto-Merge (`.github/workflows/auto-merge.yml`)

- **Trigger**: `pull_request` targeting `main`.
- Runs only when the PR carries the `dependencies` label.
- Enables `gh pr merge --auto --rebase`, so the PR rebases and merges as soon as the CI gate passes.
- Keys on the label alone — it does **not** read Dependabot metadata — so any labelled PR is eligible regardless of author.

---

## 4. Release (`.github/workflows/release.yml`)

- **Triggers**: `workflow_dispatch` (`ref` selects ship / test-build / rebuild, `dry-run` previews) and weekly cron `0 0 * * 0` (Sunday 00:00 UTC). Dispatch modes are shared ([Delnegend/actions](https://github.com/Delnegend/actions/blob/main/README.md)); versions come from Conventional Commits via `ietf-tools/semver-action@v1`.
- **Concurrency**: `group: release-main`, `cancel-in-progress: false`.
- **Binary Packaging**: Compiles `target/release/freshdesk` and packages it into `dist/freshdesk-<version>-x86_64-unknown-linux-gnu.tar.xz`.
- **Publishing**: `softprops/action-gh-release@v3` publishes the GitHub release with auto-generated release notes and attached binary asset.

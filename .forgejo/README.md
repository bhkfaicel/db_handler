# Forgejo Actions (CI)

Security-sensitive checks, tag creation and publication logic live in this
repository. `_rust-crate-checks.yml` is the shared local gate, and
`actions/checkout` is pinned to an immutable commit SHA with
`persist-credentials: false`.

## Publish flow

| Branch | Tag shape | Registry | Forgejo release |
| --- | --- | --- | --- |
| feature branch | none | — | none |
| `dev` | `vX.Y.Z-rc` | `forgejorc` | prerelease |
| `staging` | — (merge gate only) | — | — |
| `main` | `vX.Y.Z` | `forgejo` | release |

`Cargo.toml` / `CHANGELOG.md` always hold the **stable** base version.
The stable/unstable split is done by **registry**, not by rewriting the crate version.

1. Feature branch push → `check-on-feature-branch.yml`
2. Merge to `dev` → `create-tag-on-dev.yml` creates `vX.Y.Z-rc`
3. Tag push → `publish-on-tag.yml` publishes to `forgejorc`
4. Merge `dev` → `staging` → `main`
5. Push to `main` → `create-tag-on-main.yml` creates `vX.Y.Z` + `release/X.Y`
6. Tag push → `publish-on-tag.yml` publishes to `forgejo`

`validate-branch-flow.yml` enforces PR direction: `* → dev`, `dev → staging`, `staging → main`.

## Shared gate (`_rust-crate-checks.yml`)

1. `cargo fmt --all -- --check`
2. `cargo clippy --features sensors,users,utoipaDoc`
3. `cargo machete`
4. `cargo deny check` (see `deny.toml`)
5. `bash scripts/check-version-sync.sh`
6. Reject duplicate stable tag (`enable_tag_check`, disabled on tag pushes)
7. `bash scripts/test.sh` (`cargo test`, in-memory SQLite)

Tests use `config.test.json` and `SQLX_OFFLINE=true`. Docker/InfluxDB tests
(`scripts/test-docker.sh`) are for local integration only.

## Protecting this directory

Protected file patterns on `main`:

```text
.gitea/**/*.yml;.forgejo/**/*.yml
```

Secret: `RUNNER_TOKEN` (read repo, push tags, publish packages, create releases).

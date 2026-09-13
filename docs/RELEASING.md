# Releases and deployments

Linger has one repository. The runtime has versioned releases; the website deploys continuously. The runtime runs on the user's machine; Vercel hosts only the static website.

| Track | Version source | Preview | Production |
| --- | --- | --- | --- |
| Runtime | Cargo.toml | Checks on PR/main; prerelease `runtime-vX.Y.Z-rc.N` tags build macOS artifacts in `runtime-preview` | `runtime-vX.Y.Z` publishes crates.io and GitHub archives in `runtime-production` |
| Website | Commit SHA | Same-repository PRs deploy to `website-preview`; forks build without secrets | Changes on main deploy to `website-production`; no website tags or GitHub releases |

Runtime tags must match Cargo.toml exactly. Do not reuse or move a published tag. Repository rules prevent release-tag updates/deletion and main force pushes/deletion. Release only reviewed, committed source; do not capture an active working tree with `--allow-dirty`.

## Runtime

1. Update Cargo.toml and Cargo.lock versions, CHANGELOG.md and docs/releases/runtime.md.
2. Run `cargo fmt --check`, `cargo test --locked`, and `cargo clippy --all-targets --locked -- -D warnings`. Exercise the terminal path with fictional examples. Run `cargo publish --dry-run --locked` from a clean commit.
3. Push the commit to main and wait for Runtime checks. Create an annotated `runtime-vX.Y.Z` tag at that exact commit and push it.
4. The release workflow rechecks Linux/macOS, builds and smoke-tests Apple Silicon and Intel macOS archives, publishes the crate, then creates the GitHub release with archives and checksums. Prerelease tags create GitHub prereleases without publishing crates.io.
5. Verify the registry version and download each architecture's archive. Verify checksums and run the native binary before announcing availability.

The first crates.io publication uses the `CARGO_REGISTRY_TOKEN` secret in `runtime-production`. After configuring crates.io Trusted Publishing for owner `rickhallett`, repository `linger`, workflow `runtime-release.yml`, environment `runtime-production`, set the repository variable `CARGO_TRUSTED_PUBLISHING=true` and remove the bootstrap secret. The workflow exchanges GitHub OIDC for a short-lived token.

If publication succeeds but a later step fails, rerun only the failed jobs; do not republish or move the tag. Existing crate versions cannot be replaced.

Mac binaries are not Apple Developer ID signed or notarized. Linux is checked in CI, but Linux binary distribution is not enabled. Apple signing/notarization and Linux distribution need a separate decision.

## Website

1. Change only the canonical site in `web/linger/`. Historical design experiments belong in ignored local `outputs/archive/`, never in deployment inputs.
2. Build with Node 24 and `pnpm@10.34.5`: `pnpm --dir web install --frozen-lockfile` then `pnpm --dir web run build`.
3. Open a PR for a preview. Review `/`, `/guide/`, mobile layout and demo controls.
4. Merge or push website changes to main. The workflow builds and deploys production automatically. No version bump, tag or GitHub release is needed.
5. Verify https://lingerer.xyz and the www redirect. A successful build alone does not prove domain routing.

Vercel project: `linger-website`, team `rick-halletts-projects`. The repository root supplies the active site's fictional example. `vercel.json` sets the build/output paths; `.vercelignore` excludes runtime, inherited browser app, private outputs and archives. Git auto-deploy is disabled to avoid duplicate deploys; GitHub Actions owns deployment.

Both website environments need `VERCEL_TOKEN`. Repository variables hold `VERCEL_ORG_ID` and `VERCEL_PROJECT_ID`; credentials never belong in Git. Preview deployment secrets are unavailable to forks.

123 Reg retains DNS hosting. Apex A records point to Vercel's project recommendations; www uses the project CNAME and Vercel redirects it to the apex with HTTP 308. Preserve unrelated NS, SOA, mail and verification records. Re-read Vercel's domain recommendations before future DNS changes.

For rollback, promote a previously verified Vercel production deployment using the Vercel dashboard/CLI. Record the selected deployment and verify the public domain. Runtime fixes require a new version/tag; never overwrite a released crate or asset.

## Initial registry handoff

The macOS release can be downloaded before crates.io onboarding finishes. Until then, install from the exact `runtime-v0.1.0` Git tag. After account verification and a valid Cargo token are available, publish from that tag in a clean worktree, configure Trusted Publishing, update the existing GitHub release notes, and switch website install copy back to crates.io on main. Do not move `runtime-v0.1.0` or include newer development source.

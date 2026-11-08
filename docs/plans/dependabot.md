# Dependabot Plan
Status: APPROVED

## Goal
Add `.github/dependabot.yml` so GitHub's Dependabot opens version-update pull requests for the ecosystems the repository uses: Cargo (the workspace root's `Cargo.toml` and `Cargo.lock`), GitHub Actions (the four workflows and the composite action `.github/actions/setup-build`), Docker (the two build images in `dev/`) and Docker Compose (the local stack's images in `docker-compose.yml`). Dependabot only proposes changes. Nothing in this change bumps a pinned version (AGENTS.md rule 3), and each Dependabot pull request still needs the human's approval to merge, which is the "asking" rule 3 requires. The configuration has to fit the repository's rules. `scripts/check-commits.sh` rejects any pull-request commit whose subject is not `feat`/`fix`/`docs`, but Dependabot's default subject is `Bump x from a to b` (or `build(deps): ...` with Conventional Commits detection), so it would fail the `checks` job on every Dependabot pull request.

## Facts verified while planning (2026-11-08, branch `feat/dependabot` at `9b0be46`, base `main`)
- No `.github/dependabot.yml` exists yet. `.github/` holds `actionlint.yaml`, `actions/setup-build/action.yml` and `workflows/{ci,perf,profile,release}.yml`.
- Cargo: there is one workspace at the root with a committed `Cargo.lock`. Exact pins (`=`) include iced `=0.14.0`, matrix-sdk/matrix-sdk-ui `=0.19.1`, livekit `=0.9.3` and the Phase 5 and 6 crates (PROGRESS.md "Pinned versions").
- GitHub Actions use two styles. `release.yml` pins every action by SHA with a `# vX.Y.Z` comment (checkout, setup-uv, upload-artifact, download-artifact, attest-build-provenance). `ci.yml`, `perf.yml` and `profile.yml` use major tags (`actions/checkout@v6`, `Swatinem/rust-cache@v2`, `actions/upload-artifact@v4`). `.github/actions/setup-build/action.yml` uses `actions/cache/restore@v4` and `actions/cache/save@v4`. Dependabot's `directory: "/"` scans `.github/workflows` and a root `action.yml` only, so the composite action's directory has to be listed explicitly. For SHA pins, Dependabot updates the SHA and the version comment together.
- Docker: `dev/ubuntu-24.04/Dockerfile` and `dev/ubuntu-24.04-aarch64-cross/Dockerfile` both start with `FROM ubuntu:24.04`, without a digest. Ubuntu 24.04 is deliberate: it sets the glibc 2.39 floor (Phase 7 plan Q14, `phase7-linux-glibc-floor.md`). A bump to `26.04` would break that floor, so it must be ignored.
- Docker Compose: `docker-compose.yml` pins `matrixdotorg/synapse:v1.151.0`, `livekit/livekit-server:v1.13.7`, `ghcr.io/element-hq/lk-jwt-service:0.7.0` and `vectorim/element-web:v1.12.30`.
- These references are not covered by any ecosystem: `release.yml`'s `FLATPAK_IMAGE` (an env string pinned by digest), the Rust 1.97.1 `rustup` lines in both Dockerfiles, the `scripts/checksums/` downloads and `astral-sh/setup-uv`'s `version: "0.12.19"` input.
- `scripts/check-commits.sh` (run by `ci.yml` → `checks` → "Conventional Commits (pull requests)" on every `pull_request`) accepts this subject pattern: `^[0-9a-f]+ (feat|fix|docs)(\([a-z0-9._/-]+\))?!?: [^ ].*$`. It does not check the `Step:` trailer or the body. With `commit-message: { prefix: "fix", include: "scope" }`, Dependabot writes `fix(deps): bump ...` (and `fix(deps-dev): ...` for Cargo dev-dependencies). Both match the pattern.
- `deny.toml` does not conflict. Dependabot's Cargo updates come from crates.io, which is the only allowed registry (`[sources]`). A bump that adds a duplicate version only warns (`multiple-versions = "warn"`). A bump that brings in a new advisory or licence fails `cargo deny`, which is the intended gate.
- `scripts/flatpak.sh check` (in `checks`) fails unless `packaging/flatpak/cargo-sources.json` matches `Cargo.lock`. Dependabot does not regenerate that file, so **every Cargo Dependabot pull request will fail `checks` until someone runs `scripts/flatpak.sh sources` on its branch**. This is expected and documented, not worked around (see D3).
- actionlint is not run in CI, and `.github/actionlint.yaml` only lists runner labels. actionlint does not read `dependabot.yml`, so the two do not conflict.
- `perf.yml` runs on `pull_request: labeled`, but its job requires the `perf` label, so Dependabot's `dependencies` label does not start a perf run. `ci.yml` uses no secrets, so Dependabot pull requests (which get no repository secrets) run CI normally.

## Decisions (proposed; the human confirms when approving the plan)
- **D1 Commit prefix `fix` with scope (`fix(deps): ...`).** On this branch model the allowed types are only `feat`/`fix`/`docs`. A dependency bump is closest to `fix` (`feat` would suggest new behaviour). The alternative, a `build`/`chore` prefix plus an exemption in `check-commits.sh` for `dependabot[bot]`, would weaken rule 5 and is rejected.
- **D2 Schedule and volume.** Every ecosystem runs weekly (Monday), with `open-pull-requests-limit: 5` and `cooldown: default-days: 7` so brand-new releases settle first. Cargo minor and patch updates are grouped into one pull request (`cargo-minor-patch`). Major updates, including bumps of the `=` pins, come as separate pull requests, so the human can approve or close each pinned crate on its own. GitHub Actions updates are grouped into one pull request. Docker and Docker Compose are not grouped.
- **D3 Flatpak sources are regenerated by hand.** Before merging a Cargo Dependabot pull request, the human (or an agent the human asks) runs `scripts/flatpak.sh sources` on the Dependabot branch and pushes that commit as `fix(deps): regenerate flatpak cargo sources`. That commit is allowed by D1, but it stops Dependabot from rebasing the pull request. A workflow that pushes the regenerated file automatically would need write tokens on Dependabot pull requests and is left out to keep this change small.
- **D4 Ubuntu stays on 24.04.** The `docker` entries ignore `version-update:semver-major` and `version-update:semver-minor` for `ubuntu`. Because the `FROM` lines have no digest, this means Dependabot proposes nothing for them today. The entries are still useful: they start working if the human later pins the images by digest (`ubuntu:24.04@sha256:...`), which would change a pin and is the human's decision (Q1).
- **D5 Pull-request branches and titles.** Dependabot uses its default `dependabot/<ecosystem>/...` branches. When a Dependabot pull request is squash-merged, its title becomes the `main` commit subject (`fix(deps): bump ... (#NN)`). It carries no `Step:` trailer, since these are not roadmap steps. No CI check requires the trailer.
- **D6 Out of scope.** `rust-toolchain` is left out because a toolchain bump must also change the Rust 1.97.1 lines in both Dockerfiles and PROGRESS.md, and a Dependabot pull request touching only `rust-toolchain.toml` would leave them out of sync (Q2). Security updates (the repository setting "Dependabot security updates") are a human setting, not part of this file.

## Questions for the human
- Q1: Pin `ubuntu:24.04` by digest so Dependabot refreshes the image, or leave it unpinned as it is today? The default is to leave it.
- Q2: Add the `rust-toolchain` ecosystem anyway? The default is no, per D6.
- Q3: Turn on "Dependabot alerts" and "Dependabot security updates" in Settings → Code security? This plan does not change repository settings.

### Task 1: Dependabot configuration for cargo, github-actions, docker and docker-compose
Language: rust
Files: .github/dependabot.yml (new), PROGRESS.md, docs/plans/dependabot.md
Depends on: none
Acceptance criteria:
- `.github/dependabot.yml` is `version: 2` and has exactly these `updates` entries:
  - `cargo` with `directory: "/"`;
  - `github-actions` with `directories: ["/", "/.github/actions/setup-build"]`;
  - `docker` with `directories: ["/dev/ubuntu-24.04", "/dev/ubuntu-24.04-aarch64-cross"]`;
  - `docker-compose` with `directory: "/"`.
- Every entry has `schedule.interval: weekly`, `open-pull-requests-limit: 5`, `cooldown.default-days: 7` and `commit-message: { prefix: "fix", include: "scope" }`.
- `cargo` has a group `cargo-minor-patch` with `update-types: [minor, patch]`, and `github-actions` has a group with `patterns: ["*"]`.
- Both `docker` entries ignore `ubuntu` for `version-update:semver-major` and `version-update:semver-minor`.
- A header comment in the file states that Dependabot only proposes changes (rule 3), why the prefix is `fix` (rule 5 / `check-commits.sh`), the `scripts/flatpak.sh sources` step for Cargo pull requests (D3), and why Ubuntu stays on 24.04 (D4).
- The file validates against the SchemaStore Dependabot v2 schema: `uvx check-jsonschema --builtin-schema vendor.dependabot .github/dependabot.yml` exits 0.
- In a scratch git repository, `scripts/check-commits.sh` accepts commits with Dependabot-style subjects, so it exits 0 on all of these:
  - `fix(deps): bump the cargo-minor-patch group across 1 directory with 3 updates`
  - `fix(deps): bump actions/checkout from 6.1.0 to 6.2.0`
  - `fix(deps-dev): bump tempfile from 3.20.0 to 3.21.0`
  - `fix(deps): bump matrixdotorg/synapse from v1.151.0 to v1.152.0`
- In the same scratch repository, a `build(deps): bump x from 1 to 2` subject still makes it exit 1. This shows the prefix is load-bearing.
- No `Cargo.toml`, `Cargo.lock`, workflow, action, Dockerfile, `docker-compose.yml` or `deny.toml` changes (`git diff --stat main` lists only the three files above).
  - Amended at review (2026-11-12, human-approved): `checks` was red on `main` from new cargo-deny advisories, so this commit also carries `Cargo.lock` (vodozemac 0.11.2, wasmtimer 0.4.5), the regenerated `packaging/flatpak/cargo-sources.json` and one `deny.toml` ignore (RUSTSEC-2026-0459). No `Cargo.toml` pin changes. The `cargo-minor-patch` group also gains `exclude-patterns` for the `=` pins (D2). See PROGRESS.md.
- PROGRESS.md gains a "Dependabot" section with:
  - the ecosystems covered;
  - D1 to D6 in one line each;
  - the checks run;
  - under "For the human": merging each Dependabot pull request is the rule 3 approval, the `scripts/flatpak.sh sources` step for Cargo, and Q1 to Q3.
- The gate passes: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (no Rust changes, so only that nothing regressed) and `scripts/check-commits.sh main` on the branch.
- There is one commit: `feat(ci): propose dependency updates with Dependabot`, with the trailer `Step: ci.dependabot`. The pull request is opened against `main`.
Evidence: **HUMAN (after merge):**
- GitHub → Insights → Dependency graph → Dependabot lists four update configurations without errors.
- The first Dependabot pull requests have titles starting `fix(deps):`, and their `checks` job passes the "Conventional Commits (pull requests)" step. For Cargo pull requests, `scripts/flatpak.sh check` fails until `scripts/flatpak.sh sources` is pushed, as D3 expects.
- Before merge, the pull request's own CI run is green. `.github/dependabot.yml` is not under `docs/` and is not a `.md` file, so `changes` reports `code=true` and the full build runs.

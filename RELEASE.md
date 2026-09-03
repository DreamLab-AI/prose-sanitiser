# Releasing prose-sanitiser

Two separate steps: publish crates to crates.io (manual, credentialled), then
tag the release so GitHub Actions builds and attaches prebuilt binaries.

---

## 1. crates.io publish order (prose-sanitiser)

Seven workspace crates, six published. Path dependencies mean each must land on
crates.io before the next can resolve its version. Publish in this order:

1. `prose-sanitiser-core`
2. `prose-sanitiser-unicode` (depends on core)
3. `prose-sanitiser-uk` (depends on core)
4. `prose-sanitiser-slop` (depends on core)
5. `prose-sanitiser-media` (depends on core, unicode)
6. `prose-sanitiser` (the cli crate; depends on core, unicode, uk, slop, media)

`prose-sanitiser-server` is **not published** to crates.io in the first release.
It is an internal HTTP service with no outside consumers.

### Pre-publish checks

```sh
# From the workspace root:

# Dry-run every crate in publish order:
cargo publish --dry-run -p prose-sanitiser-core
cargo publish --dry-run -p prose-sanitiser-unicode
cargo publish --dry-run -p prose-sanitiser-uk
cargo publish --dry-run -p prose-sanitiser-slop
cargo publish --dry-run -p prose-sanitiser-media
cargo publish --dry-run -p prose-sanitiser

# Full test suite:
cargo test --workspace --locked

# Docs (must build without warnings):
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Clippy:
cargo clippy --workspace --all-targets --locked -- -D warnings

# Format:
cargo fmt --all --check
```

### Publish

Credentials live at `$CARGO_HOME/credentials.toml`. Between each step, wait for
the crates.io index to update (typically 30 to 90 seconds) before publishing the
next crate.

```sh
cargo publish -p prose-sanitiser-core
# wait ~60s
cargo publish -p prose-sanitiser-unicode
cargo publish -p prose-sanitiser-uk
cargo publish -p prose-sanitiser-slop
cargo publish -p prose-sanitiser-media
cargo publish -p prose-sanitiser
```

---

## 2. Tagging

All workspace crates share one version. Tag the release commit:

```sh
# prose-sanitiser:
git tag -a v0.1.0 -m "prose-sanitiser v0.1.0"
git push origin v0.1.0

git push origin v0.1.0
```
---

## 3. Prebuilt binaries (GitHub Actions)

This repository carries `.github/workflows/release.yml`. Pushing a `v*` tag
triggers the workflow automatically. It can also be re-run manually via
`workflow_dispatch` with an existing tag.

### What the workflow does

1. **Build** on a four-target matrix, each on a native runner (no
   cross-compilation):

   | Target | Runner |
   |:-------|:-------|
   | `x86_64-unknown-linux-musl` | `ubuntu-24.04` |
   | `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm` |
   | `x86_64-apple-darwin` | `macos-15-intel` |
   | `aarch64-apple-darwin` | `macos-15` |

   Linux builds link against musl for fully static binaries.

2. **Package** each target's binaries (stripped), `LICENSE-MIT`,
   `LICENSE-APACHE` and `README.md` into a tar.gz archive, with a `.sha256`
   companion:
   - `prose-sanitiser-v0.1.0-x86_64-unknown-linux-musl.tar.gz`
   - `prose-sanitiser-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256`
   - (and three more targets)

3. **Smoke-test** every binary with `--help` before uploading.

4. **Create a GitHub Release** (or update an existing one) with all eight
   artefacts attached. Release notes are extracted from `CHANGELOG.md` when a
   matching `## [version]` section exists.

### prose-sanitiser artefact contents

Each archive contains 12 binaries:

`sanitise`, `inspect-text`, `clean-text`, `inspect-image`, `clean-image`,
`inspect-file`, `clean-file`, `audit-dir`, `audit-website`, `rewrite-text`,
`slop-scan`, `slop-detect`


### CI workflow

This repository also carries `.github/workflows/ci.yml`, which runs on every
push to `main` and every PR:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --locked` (warnings are errors)
- `cargo test --workspace --locked`
- `cargo doc --workspace --no-deps --locked` (warnings are errors)
- `cargo deny check licenses advisories bans sources` (supply-chain gate)

Only first-party GitHub actions are used (`actions/checkout`,
`actions/upload-artifact`, `actions/download-artifact`). No third-party release
actions; the `gh` CLI on the runner creates the release.

---

## 4. Nix pin bump in agentbox

After a release, bump `lib/prose-sanitiser.nix` in agentbox so the container
builds the published tag rather than an in-tree copy.

The pin follows the `lib/solid-pod-rs.nix` precedent: `fetchFromGitHub` on the
release tag's commit, and the crate's own checked-in `Cargo.lock` (no vendored
copy). The SRI hash is computed with `nix flake prefetch`; from a machine without
nix, the same command runs in the official container:

```sh
docker run --rm nixos/nix nix flake prefetch --json github:DreamLab-AI/prose-sanitiser/<rev>
# → {"hash":"sha256-…", …}
```

```nix
src = pkgs.fetchFromGitHub {
  owner = "DreamLab-AI";
  repo  = "prose-sanitiser";
  rev   = "<commit of the v0.1.0 tag>";
  hash  = "<sha256-… from the prefetch>";
};
cargoLock.lockFile = "${src}/Cargo.lock";
```

---

## Checklist summary

1. All tests pass, clippy clean, docs build without warnings, `cargo deny` clean
2. `cargo publish --dry-run` for each crate in order
3. `cargo publish` for each crate in order, waiting for the index between each
4. Tag the release commit and push the tag
5. GitHub Actions builds four platform archives and attaches them to a Release
6. Verify the GitHub Release page has all eight artefacts (four `.tar.gz` + four `.sha256`)
7. Bump the Nix pin in agentbox: `rev` and `hash`

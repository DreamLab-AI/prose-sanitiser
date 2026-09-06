---
title: ADR-2030 acceptance — prose-sanitiser side
date: 2026-09-05
type: closeout
status: partial
repo: prose-sanitiser
adr: agentbox ADR-2030
---

# ADR-2030 acceptance — the prose-sanitiser side

[agentbox ADR-2030](https://github.com/DreamLab-AI/agentbox/blob/main/docs/adr/ADR-2030-permissive-licensing-for-publishable-service-crates.md)
gained a closeout extension on 2026-09-04 after the estate review's
release-boundary pass (VisionFlow `docs/estate-review/configuration-projection.md`,
section "Service package and release metadata" — a sibling repository, so
deliberately named rather than linked relatively)
found its packaging commitments unimplemented. That review inventoried the
**eight package manifests still under agentbox `services/`**. prose-sanitiser is
not one of them: it was extracted on 2026-09-03 and removed from `services/` in
agentbox `38009c3`. This document covers only the extracted side. The
agentbox-side crates are the `agentbox-ingress-config` agent's work.

Evidence for everything below: [release receipt 2026-09-05](../../release-receipts/2026-09-05.md)
and its [JSON twin](../../release-receipts/2026-09-05.json).

## Closed from this repository

**Retained versus extracted ownership is reconciled — and proved.** Ownership is
fully extracted, with nothing retained. The `crates/` subtree here at `3b545e1`
and agentbox's `services/prose-sanitiser/crates` at `3c853e9` share the git tree
hash `7a3ef0d9b45dc85409ea295f33de185a0cf8e584`. Two repositories cannot share a
tree hash by accident, so this is a content-identity proof rather than a claim
read off commit subjects. The estate review's "older ten-crate account needs
reconciliation" is answered for this family: seven crates, six publishable, none
retained under `services/`.

**The declared texts and statements now exist, and ship.** Every crate directory
carries `LICENSE-MIT` and `LICENSE-APACHE` (symlinks to the workspace texts) and
a `README.md` with a licence statement. The review's complaint was that these
were *absent from the package directories*; here they are present **and inside
the archives** — cargo dereferences the symlinks, so each archive holds real
1,120-byte MIT and 11,393-byte Apache-2.0 files. Confirmed by extracting an
archive and reading the files.

**Packaged contents were actually inspected, not inferred.** `cargo package
--list` ran for all seven crates and `cargo package` built and verification-built
all six publishable ones. File counts, licence texts, README presence and
archive digests in the receipt come from the archives themselves.

**A broken licence trail was found and fixed.** All seven per-crate READMEs
linked ADR-2030 as `../../../../docs/adr/…`, a path that resolved only in the old
agentbox `services/prose-sanitiser/crates/<x>/` layout. Post-extraction it
escapes the repository root, and on crates.io a repo-relative link means nothing
at all. So the ADR-2030 statement ADR-2030 itself requires was, in every
published archive, a dead link. Each now uses an absolute URL to the agentbox
ADR, names both licence files, and states the position accurately: the crates
were permissive inside an AGPL aggregate, and since extraction there is no
copyleft aggregate above them.

**Manifest metadata is complete across all seven crates** — `description`,
`license`, `repository`, `readme`, `keywords`, `categories`, `rust-version` —
verified through `cargo metadata`. `prose-sanitiser-server` was the one gap
(`readme`, `keywords`, `categories` missing); it is `publish = false`, but is now
described identically to its siblings so a workspace-wide inventory sees no hole.

**The operator's published-crate standard is met and now enforced.**
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features` is
clean with `#![deny(missing_docs)]` active on **all seven** crates, up from two.
146 undocumented public items were documented; 11 new compiling examples bring
the doctest total to 43. Because the lint is a `deny` in each crate root, this is
a standard the build enforces from now on rather than a state that will erode.

**A CI blind spot was closed.** The workflow ran clippy, test, doc and the MSRV
build without `--all-features`, so the six `cfg` blocks behind the optional
`external-verify` feature in `prose-sanitiser-media` were never compiled by the
gate and could rot unnoticed. All four steps now pass `--all-features`. Safe on a
bare runner: the gated code degrades when a tool is absent rather than requiring
one, and the tests assert the shape of the tool report, not any tool's presence
— confirmed locally, where `exiftool` is installed and `c2patool` is not, so both
branches ran.

**A real clippy defect was fixed.** `cargo clippy --workspace --all-targets
--all-features -- -D warnings` failed on `chunks_exact_to_as_chunks` in the
zero-width-payload bit-packing loop (`crates/unicode/src/stego.rs`), now using
`as_chunks::<8>()`. 767 tests and `cargo deny check` pass.

## Remaining, and why

**The receipt binds a dirty working tree, not a committed revision.** This is the
main gap. Everything above is uncommitted: this pass was instructed not to
commit. So the archive digests belong to head `422fc82` *plus* diff
`40f3323a…`, and reproducing them needs both. The receipt says so plainly rather
than implying a revision binding it does not have. **Re-run the receipt once the
diff is committed** — that single step closes ADR-2030's "bind each release to
source revision" clause properly.

**The published `0.1.1` on crates.io is unverified against these digests.** No
registry was contacted. Whether the archives already published under `0.1.1`
match what this working tree produces is unknown from here, and given the README
and documentation changes above, they almost certainly do **not**. A `0.1.2`
carrying this work is the clean resolution; comparing against the published
archives is a separate pass.

**MSRV 1.89 was not proven locally.** No 1.89 toolchain exists in this
container. It is not an open gap in substance — CI's `msrv` job reads
`rust-version` from `Cargo.toml`, installs exactly that toolchain and compiles
every test binary against it — but this pass verified it on CI's authority
rather than by running it.

**The distribution-specific licensing review remains open.** ADR-2030's closeout
routes this through the designated maintainer process. `cargo deny check`
reporting `licenses ok` is a metadata check against an allow-list; it is not a
legal compatibility assessment and is not offered as one. The two advisory
ignores (`RUSTSEC-2023-0071`, `RUSTSEC-2024-0370`, both reached only through
`c2pa`) are documented with reasons in `deny.toml` and restated in the receipt.

**Nix consumption is not verified from here.** ADR-2030 asks for extracted
repository *and* consumed Nix revisions. The extracted repository is verified
above; the Nix side lives in agentbox `38009c3` and belongs to that agent's pass.

**ADR-2030's services-wide staleness gate is untouched.** Its stale
`verified_commit` covers a services-wide scope this repository cannot speak for.
Nothing here advances it, and no generated index was refreshed to make the gap
look smaller.

## Status

Partial, and honestly so. Every packaging, documentation and content-inspection
item ADR-2030 asked of the extracted crates is closed. The revision binding is
one commit away; the registry comparison, the MSRV proof and the maintainer
licensing review are genuinely outside this pass.

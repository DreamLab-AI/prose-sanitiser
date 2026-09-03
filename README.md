<div align="center">

# prose-sanitiser

### Deterministic AI-provenance sanitiser and prose linter, in Rust

[![Licence](https://img.shields.io/badge/Licence-MIT%20OR%20Apache--2.0-blue?style=flat-square)](LICENSE-MIT)
[![crates.io](https://img.shields.io/crates/v/prose-sanitiser?style=flat-square)](https://crates.io/crates/prose-sanitiser)
[![CI](https://img.shields.io/github/actions/workflow/status/DreamLab-AI/prose-sanitiser/ci.yml?style=flat-square)](https://github.com/DreamLab-AI/prose-sanitiser/actions)

</div>

---

## What is prose-sanitiser?

Two jobs. Make prose read as though a competent human with opinions decided
every word of it. Make the files that carry it clean of the provenance metadata
and invisible-Unicode contraband a machine can read.

What sets it apart is the combination: deterministic invisible-Unicode
surgery (including decoded smuggled payloads), lossless container-level
provenance stripping (EXIF, XMP, C2PA, OOXML revision IDs, PDF incremental
updates), sense-aware UK-English enforcement built on VarCon rather than a flat
word list, and versioned, confidence-tiered AI writing-tell tables whose
measured performance is published alongside the rules. Each of those exists
separately. Together they do not.

Everything runs in-process, with no model, no network and no external tools on
the default path. Every decision is either a codepoint classification or a
container-structure deletion, so a strip is verifiable by diffing the output.
The `sanitise` binary runs every layer over a file or tree in one pass,
reporting in SARIF 2.1.0, JSON Lines, JSON or plain text.

What it cannot do is stated in this README
rather than buried in a footnote. Statistical sampling watermarks (Claude's own, since 2 August 2026, and others) need the vendor key to detect or remove.
Pixel-domain image watermarks need a proprietary decoder. Slop rules are a
style linter with an evidence base, not a detector: on general-domain corpora
they separate human from machine text barely better than chance at a usable
operating point, and the numbers are below.

## Where prose-sanitiser sits in the ecosystem

prose-sanitiser is one component of
**[VisionFlow](https://github.com/DreamLab-AI/VisionFlow)**, a federated
human-AI mesh. It was extracted from agentbox (the sovereign agent runtime) into
its own repository for independent publication on crates.io.

| Sibling | Relationship |
|:--------|:-------------|
| [agentbox](https://github.com/DreamLab-AI/agentbox) | Consumes the binaries via a pinned Nix derivation and the `prose-sanitiser` skill. The SARIF output feeds agentbox's code-scanning surface |
| [loom](https://github.com/DreamLab-AI/loom) | Uses the evaluator's measurement-and-disclosure style as a model for its own benchmark presentation |
| [VisionFlow](https://github.com/DreamLab-AI/VisionFlow) | Ecosystem canon; the maturity vocabulary and ADR registry that govern this crate's publication |

## Architecture

### Crate table

| Crate | Role | Published |
|:------|:-----|:----------|
| `prose-sanitiser-core` | Shared types: `Finding`, `Span`, `Patch`, `Severity`, `ConfidenceTier`, `Fixability`, the `Check`/`Fix` traits. No I/O, no subprocesses | Candidate |
| `prose-sanitiser-unicode` | Layer A: invisible-Unicode classification, UTS #39 homoglyph detection, payload decoding, bidi policy | Candidate |
| `prose-sanitiser-uk` | VarCon-backed UK-English enforcement with CommonMark span exclusion and sense disambiguation | Candidate |
| `prose-sanitiser-slop` | Versioned, confidence-tiered AI writing-tell rule tables and scanners | Candidate |
| `prose-sanitiser-media` | Image and container provenance surgery (PNG, JPEG, WebP, PDF, OOXML, ODF, SVG) | Not first wave |
| `prose-sanitiser` (cli) | The `sanitise` umbrella binary, the eleven task binaries, audit sweeps and the rewrite layer | Workspace |
| `prose-sanitiser-server` | HTTP service (axum). Not published | No |

### Three axes

Conflating any two produces a specific bug, so they are kept separate and
`prose-sanitiser-core` enforces the separation in the type system.

| Axis | Answers | Values |
|:-----|:--------|:-------|
| `Severity` | How much does it matter? | `High`, `Medium`, `Low` |
| `ConfidenceTier` | Is the pattern right? | `CertainMechanical`, `HighConfidenceStylistic`, `LowConfidenceJudgement` |
| `Fixability` | Can it be repaired at all? | `Mechanical`, `OptIn`, `ReportOnly`, `NoFixExists` |

### Law

**Detection is unconditional; mutation is gated separately.** Every rule has two
switches: whether the finding exists, and whether it carries a repair.
Contraband is always reported. Whether the tool rewrites it is a policy question
with its own default, so "tell me but do not touch it" is always available.

### Capability matrix

As of 2026-09-03. Taken from the research brief, section B.

| Block | Contents |
|:------|:---------|
| **Detects and strips losslessly** (verifiable by diff) | Invisible Cf-class controls (zero-width family, tag block, variation selectors, Hangul fillers). Exotic whitespace (reported always; fold to U+0020 is opt-in). Variation-selector and tag-block smuggled payloads, including decoding the hidden bytes. Homoglyph and mixed-script substitution (reported always; fold to ASCII is opt-in). C2PA JUMBF manifests in JPEG, PNG, WebP, PDF, SVG. EXIF, XMP, IPTC, PNG text chunks. PDF `/Info` and `/Metadata` with full object-graph rewrite. OOXML `docProps`, tracked changes, `rsid`; ODF `meta.xml` |
| **Detects and reports only** | Statistical sampling watermarks (SynthID-Text, Kirchenbauer, Aaronson, Claude's own mark). Pixel-domain image watermarks. Durable Content Credentials (C2PA soft binding). AI stylistic tells: lexical, structural, narrative |
| **Degrades, never removes** | Paraphrase changes tokens, which degrades any sampling watermark as a side effect. Lossy, unverifiable, not removal |
| **Never touches** | Emoji ZWJ glue, Indic/Persian joiners, balanced bidi in RTL prose, BOM at offset 0, soft hyphen (reported; strip is opt-in). Code fences, inline code, URLs, file paths, front matter. US spelling in proper nouns and direct quotations. Sense-dependent pairs (program, meter, sulfur, dialog box). Pixel data (on the default path) |

## Quickstart

### Install

```sh
# From source (needs Rust 1.85+):
cargo install prose-sanitiser

# Or grab a prebuilt binary from the GitHub release:
# https://github.com/DreamLab-AI/prose-sanitiser/releases
```

### `sanitise` umbrella

```sh
# Review a Markdown draft for everything, human-readable output:
sanitise draft.md

# Same, as SARIF for GitHub code scanning:
sanitise --format sarif draft.md > report.sarif

# Apply the safe fixes (invisible Unicode, metadata):
sanitise --fix draft.md

# Also apply the high-confidence-stylistic fixes (UK spelling):
sanitise --write draft.md

# Preview what --write would change, without touching the file:
sanitise --diff draft.md
```

Exit codes: **0** clean, **1** findings reported, **2** tool error (bad
arguments, unreadable input, failed write).

### Worked examples

**De-slop a Markdown draft:**

```sh
sanitise --format text essay.md
# essay.md:3:15 tier1-vocab [low] "delve" is a Tier-1 AI lexical marker
# essay.md:7:1  us-spelling [high] "optimize" -> "optimise"
# exit 1

sanitise --write essay.md
# Applies us-spelling fixes; slop findings are report-only, never applied.
```

**Strip provenance from an image:**

```sh
inspect-image photo.jpg          # shows EXIF, XMP, C2PA manifest
clean-image photo.jpg            # writes photo.cleaned.jpg, pixels untouched
clean-image --in-place photo.jpg # overwrites (writes .bak first)
```

**Strip provenance from a .docx:**

```sh
inspect-file report.docx         # shows docProps, tracked changes, rsid
clean-file report.docx           # writes report.cleaned.docx
```

**Review without rewriting:**

```sh
sanitise draft.md                # exit 1 if anything found, nothing changed
sanitise --format jsonl draft.md # one JSON object per finding, for scripting
```

## Measured

A capability matrix is a claim until someone counts. Measured 2026-09-03
against ruleset **2026.09.03**, using the `ps-eval` harness over RAID (MIT),
MAGE (Apache-2.0), LLM-DetectAIve (CC BY-SA 4.0), and a 2,000-document British
prose set (Hansard, GOV.UK, Project Gutenberg; 1.2M words, all pre-2022).

### Fixtures

35 of 35 fixtures pass. 0 of 9 legitimate-Unicode false positives.

### Invisible-Unicode and homoglyph recall

On the RAID adversarial splits (1,000 homoglyph-injected, 1,000 zero-width-space-injected documents):

- **Homoglyph recall: 1.0000** (1,000/1,000). Precision 0.9804 (the 20 "false positives" on unattacked text are real no-break spaces and soft hyphens the corpus does not label).
- **Zero-width-space recall: 1.0000** (1,000/1,000). Same precision.

### UK-English false positives on British prose

2,000 documents of known-good British English. Every finding is a false positive
by construction.

- `us-spelling` flags **5.6% of documents** (0.10 per 1,000 words). Upper bound,
  not a count: spot-checked tokens are genuine Americanisms in quoted American
  sources and nineteenth-century Gutenberg text.
- With `--write`, **116 edits** were applied across 1.2M words, all from
  `us-spelling`, all genuine Americanisms. No other rule was write-eligible.
  Zero documents were corrupted.
- For comparison: seven commercial AI detectors showed a **61.3%** false-positive
  rate on TOEFL essays (Liang et al. 2023, *Patterns*). No published study
  measured linter false positives on British English before this.

### Slop rules: TPR at 1% FPR

Reported as true-positive rate at a fixed 1% false-positive rate, never AUROC.

| Corpus | Score | TPR at 1% FPR |
|:-------|:------|:--------------|
| RAID (unattacked) | raw slop_score | **2.8%** |
| RAID (unattacked) | per 1,000 words | **3.4%** |
| MAGE | raw slop_score | **0.9%** |
| MAGE | per 1,000 words | **1.7%** |
| LLM-DetectAIve (n=20 per class) | per 1,000 words | **50.0%** |

**Read that plainly.** On general-domain corpora the aggregate slop score
separates human from machine text barely better than chance at a usable
operating point. LLM-DetectAIve looks strong, but n is 20 per class and
the corpus is deliberately composed of heavily marked machine text, so it is an
illustration, not a result. This crate is a **style linter with an evidence
base**, not a detector.

## What it does not do

Three statements that appeared in the original Python tool were withdrawn rather
than softened, because publishing them would have been false claims:

1. **Statistical sampling watermarks are not something the tool strips.** It
   cannot, and no third party can without the vendor key. Paraphrase degrades a
   sampling watermark as a side effect of changing tokens; that is lossy and
   unverifiable.
2. **"Proving the mark was cleared in a closed loop"** was a claim about
   self-applied marks with known keys (the MarkLLM harness). It says nothing
   about a vendor's production watermark.
3. **Pixel-domain watermark removal is not a capability.** It depends on an
   external GPU harness, it is itself detectable at over 98% TPR at 1% FPR, and
   stripping a container manifest does not defeat a durable Content Credential.

prose-sanitiser does not detect authorship. A clean scan is not evidence of
human writing. A dirty one is not evidence of a model. A finding is a prompt
for an editor to look, never a verdict.

## Documentation

- Crate READMEs: [core](crates/core/README.md), [unicode](crates/unicode/README.md), [uk](crates/uk/README.md), [slop](crates/slop/README.md), [media](crates/media/README.md)
- [Research brief (2026-09-03)](docs/prose-sanitiser-research-2026-09-03.md)
- Review records: [adversarial pass 1](docs/prose-sanitiser-codex-review-2026-09-03.md), [pass 2](docs/prose-sanitiser-codex-review-2-2026-09-03.md), [external static review](docs/prose-sanitiser-external-review-2026-09-03.md)
- [Releasing](RELEASE.md)
- [CHANGELOG](CHANGELOG.md)
- [CLI quick reference](README-CLI.md)

## Status

As of 2026-09-03. Three review passes are complete ahead of the first
crates.io publication: two adversarial passes by an independent model
(`docs/prose-sanitiser-codex-review-*.md`) and one external static review
(`docs/prose-sanitiser-external-review-2026-09-03.md`), with every live
finding fixed and tested. The workspace carries 747 tests, `cargo clippy
-D warnings` is clean, and 35 of 35 evaluation fixtures pass. Anything not
covered by the fixture suite or the numbers above should still be treated as
provisional; deferred items are listed at the end of the review records.

Crates `core`, `unicode`, `uk` and `slop` are publication candidates:
pure Rust, no C dependencies, no subprocesses, no network, every public item
documented, `cargo doc --no-deps` clean. `media` pulls a heavier dependency tree
and is published with the CLI as a second wave. `server` is internal.

Rule tables are dated and versioned (`RULESET_VERSION 2026.09.03`). Every
rule carries `since`, `reviewed` and its sources, so a table nobody has
re-checked is visible as data rather than silently rotting. Structural
measures include two that do not work (paragraph-length CV, tricolon rate) and
one that inverts between corpora (Oxford-comma density). They are kept as
house-style budgets and labelled accordingly.

## Licence

**MIT OR Apache-2.0**, at your option.

Copyright (c) 2026 DreamLab AI Consulting Ltd and contributors.

Vendored VarCon data (in `prose-sanitiser-uk`) keeps its own permissive notice;
see `crates/uk/data/LICENSE-VarCon`.

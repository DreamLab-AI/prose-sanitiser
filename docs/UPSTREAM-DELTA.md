# Upstream delta: watermarks-remover v0.6.0

prose-sanitiser began as a clean-room Rust port of the deterministic layers of
[guillaumemeyer/watermarks-remover](https://github.com/guillaumemeyer/watermarks-remover),
which it credits in the changelog. Upstream shipped v0.3.0 to v0.6.0 between
mid-August and 26 August 2026. This page records the comparison made on
3 September 2026 so the next one starts from a known point. Upstream is under
its own licence; ideas and test vectors were ported, code was not copied, and
this workspace stays MIT OR Apache-2.0.

## Ported in 0.1.1

| Upstream change | Here |
|---|---|
| Layer A: reserved `Default_Ignorable` code points | `RESERVED_IGNORABLE_RANGES` in `prose-sanitiser-unicode`, kind `reserved_ignorable`, stripped everywhere |
| Layer A: noncharacters | All 66 (U+FDD0..FDEF, U+xFFFE/xFFFF per plane), kind `noncharacter` |
| Layer A: blank-rendering carriers outside `Cf` | U+180F, U+3164, U+FFA0; context-kept after their own script, stripped elsewhere |
| Decompressed PNG size cap (#308) | Per-chunk 1 MiB and per-file 8 MiB inflate budget for `zTXt`/`iTXt` in `image::png` |

Not from upstream but found by the same pass: Egyptian, Duployan and musical
layout controls were being stripped in context and corrupting rendered text;
they are now kept next to their own script.

## Already covered before 0.1.1

| Upstream change | Here |
|---|---|
| DTD and entity-expansion bombs (#146) | XML is read with `quick-xml`, which resolves only the five predefined entities and numeric references and never expands a DTD; WordprocessingML parsing also caps attributes per element (512) and depth (256) |
| Zip bombs in OOXML containers | `MAX_ZIP_DECOMPRESSED_BYTES` 128 MiB, `MAX_ZIP_ENTRY_BYTES` 64 MiB, `MAX_ZIP_ENTRIES` 4096 |
| Subprocess output growth | `MAX_OUTPUT_BYTES` 64 MiB plus address-space and file-size rlimits on every harness child |

## Not ported, tracked

| Upstream change | Status |
|---|---|
| ReDoS fix (#306) | Not audited against the four `Regex::new` sites in `prose-sanitiser-slop`. The `regex` crate guarantees linear-time matching, so the upstream class of bug (backtracking engine) cannot occur here; the sites still deserve a size-limit review. |
| SSRF hardening for the site crawler | `audit-website` does not yet refuse loopback, link-local or private-range hosts, nor pin redirects to the starting host. Open item. |
| Format coverage: AVIF, HEIC, BMP, GIF, TIFF, EPUB, OOXML beyond DOCX, audio and video | Not started. Each is a new container module under `prose-sanitiser-media`. |
| Layer B: detection-guided iterative rewriting, keyed-Gumbel EXP verifier, benchmarks | Not started; Layer B here is the single-pass `rewrite-text`. |
| Watermark-stealing module (#303) | Deliberately not ported. This is a sanitiser, not an attack tool. |
| Claude Code plugin distribution with a PostToolUse hook | Not applicable; the agentbox skill already drives the binaries. |

## Method

Upstream's changelog and pull-request numbers were read against the current
crate; each row above names the constant, module or binary that implements or
lacks the behaviour so the claim can be checked by grep. Nothing in this table
is inferred from documentation alone.

# Second-pass adversarial security and correctness review

Reviewed branch `rust/prose-sanitiser-hardening` at `0f80914b6`. No repository files were modified.

## A. First-pass finding verification

| # | Status | Evidence |
|---:|---|---|
| 1 | **CLOSED** | Exact tag-block payload now produces `output_length: 1`, `removed_count: 6`, leaving only U+1F3F4. The Arabic payload produces `output_length: 1`, `removed_count: 8`. Payload spans override preservation at `crates/unicode/src/lib.rs:393-399`. |
| 2 | **CLOSED** | Malformed PDF with XMP xpacket and broken xref exits **2**, reports that `lopdf` cannot build a complete object graph, and creates no output file. The raw-byte/copy success fallback is gone. |
| 3 | **CLOSED** | An 8 GiB sparse PDF exits **2** immediately: `refusing input larger than 268435456 bytes`; no destination is created. Image/container entry points use `read_capped`, e.g. `crates/media/src/container/mod.rs:109,190` and `crates/media/src/image/mod.rs:100,217`. |
| 4 | **CLOSED** | Workspace now resolves `quick-xml 0.41`; the two 0.37.5 advisories no longer appear in `cargo audit`. The unrelated transitive `rsa 0.9.10` advisory remains, as discussed below. |
| 5 | **CLOSED** | Lying-size ZIP regression test passed: `container::ooxml::tests::zip::the_budget_bounds_the_bytes_produced_not_the_bytes_declared`. Actual output is read through `take(allowance + 1)` and checked at `crates/media/src/container/ooxml/mod.rs:186-196`; entry count is capped at 4,096. |
| 6 | **STILL OPEN** | The exact four-line UK probe no longer reports either single-quoted sentence and no longer reports the relative-link destination. It still reports line 3, the four-space `color: red`. Without a preceding blank line, CommonMark treats this as paragraph continuation rather than an indented code block. |
| 7 | **CLOSED** | `a_non_c2pa_jumbf_box_is_neither_flagged_nor_deleted` passes. APP11 packets are reassembled and typed; non-C2PA JUMBF is reported as preserved and survives provenance-only cleaning at `crates/media/src/image/jpeg.rs:150-166`. |
| 8 | **CLOSED** | WordprocessingML now enables end-name checking and tracks depth at `crates/media/src/container/ooxml/wordml.rs:217-230`; mismatched/unclosed XML regression tests pass. Modified parts are also reparsed before archive assembly. |
| 9 | **CLOSED** | Accepted by ADR/copyright-holder decision. Workspace manifests, licence files and `services/LICENSING-NOTICE.md` consistently establish `MIT OR Apache-2.0`. |
| 10 | **CLOSED** | Documentation now scopes losslessness and pixel preservation to the relevant paths and describes pixel-removal backends as intentional lossy operations. Malformed PDF processing is now an error rather than a degraded successful clean. |

### Remaining reproduction for finding 6

```text
He said ‘The color is red.’
He said 'The color is red.'
    color: red
[color](relative/path/color)
```

```console
$ slop-scan probe.md --format json
# exit 1
[(3, "us-spelling", "color: red"),
 (4, "us-spelling", "[color](relative/path/color)")]
```

The line-4 finding is correctly against visible link text. The destination is no longer separately flagged.

The unresolved ambiguity is at `crates/uk/src/exclude.rs:183-205`: `pulldown-cmark` only emits `CodeBlock` for the indented line when the surrounding Markdown makes it one. If the documented promise is that every four-space line is protected, add a conservative lexical exclusion for lines beginning with four spaces or one tab. Otherwise document that CommonMark paragraph continuation is prose, not code, and narrow the claimed fix.

## B. New findings

| Severity | File:line | Description |
|---|---|---|
| **High** | `crates/slop/src/prose/mod.rs:238-244`; `crates/slop/src/design/mod.rs:213`; `crates/cli/src/dispatch.rs:84-90`; `crates/cli/src/sanitise.rs:286-294`; `crates/cli/src/settings.rs:51-60` | Slop, umbrella classification and configuration paths still perform uncapped whole-file reads; slop converts read/allocation failure into a clean result. |
| **Medium** | `crates/cli/src/bin/sanitise.rs:244-258`; `crates/slop/src/prose/mod.rs:138-175`; `crates/cli/src/audit/mod.rs:66-84` | Directory scans fail open on unreadable files and can exit 0 after scanning nothing. |
| **Medium** | `crates/core/src/suppress.rs:91-128,214-237` | A suppression-looking HTML comment inside fenced or inline code controls findings in later prose. |
| **Medium** | `crates/cli/src/bin/sanitise.rs:301-306`; `crates/core/src/report.rs:391-412` | `sanitise` SARIF emits contradictory driver/result fixability because the driver never receives the override table. |
| **Low** | `crates/server/src/lib.rs:67-72,280-291,574-585` | The router never installs its advertised body limit and therefore inherits Axum’s roughly 2 MiB `Bytes` extractor limit; NUL-containing names also become HTTP 500. |
| **Low** | `Cargo.toml:1`; workspace root | `cargo deny check` is not configured, so the licence gate rejects essentially every dependency and cannot provide meaningful licence assurance. |

## New finding details

### 1. High: uncapped reads and clean-on-read-failure in slop paths

Reproduction using a 300 MiB sparse text file under a 128 MiB virtual-memory limit:

```sh
truncate -s 300M huge.txt

(ulimit -v 131072; slop-scan huge.txt)
# exit 0, no diagnostic

(ulimit -v 131072; slop-detect huge.txt)
# exit 0, reports clean

(ulimit -v 131072; sanitise huge.txt)
# exit 2: cannot read ...: out of memory
```

`scan_file` catches every `std::fs::read` error and returns an empty finding list at `crates/slop/src/prose/mod.rs:238-244`. The design scanner behaves similarly. `sanitise` reports the error, but only after attempting uncapped reads during classification and text loading. Explicit hostile TOML is also read wholly through `read_to_string`.

Suggested fix:

- Introduce one capped regular-file reader in `core` or CLI infrastructure and use it for classification, slop, sanitise and configuration.
- Classify from an extension plus a capped prefix instead of reading the whole file.
- Give configuration a small independent cap, such as 1 MiB.
- Return a typed error from slop scanning; never encode read failure as `Vec::new()`.

### 2. Medium: recursive scans exit successfully after unreadable inputs

Reproduction with a directory containing one mode-000 `.txt` file:

| Binary | Exit | Observed result |
|---|---:|---|
| `audit-dir` | 0 | Counts one scanned error item, but exits clean |
| `sanitise` | 0 | Prints permission error, then `0 files checked, 0 findings` |
| `slop-detect` | 0 | Reports clean |
| `slop-scan` | 0 | Reports one file scanned and zero findings |

This violates the documented 0/1/2 contract and makes CI pass when the requested content was not examined.

Suggested fix:

- Track traversal, metadata and read errors separately from findings.
- Continue a recursive scan for coverage, but exit 2 if any requested file could not be examined.
- Include an explicit `errors` collection in JSON/SARIF output.
- Count `files_scanned` only after successful input decoding.

### 3. Medium: directives inside code fences suppress later prose

Reproduction:

````markdown
```html
<!-- prose-sanitiser-disable -->
```

We delve into a tapestry of insights.
````

```console
$ sanitise probe.md --no-language-filter
# exit 0, zero findings

$ sanitise probe.md --no-language-filter --no-suppressions
# exit 1, tier1-vocab finding for "delve"
```

`comments()` searches raw bytes for `<!--` and `-->` without consulting Markdown structure. Thus examples, generated code and quoted directive syntax alter policy outside their own spans.

Suggested fix:

- Parse Markdown/MDX/reST structure before recognising directives.
- Ignore comments inside fenced code, indented code, inline code and quoted literal blocks.
- At minimum, reuse the same protected-span machinery already used by the UK and structural scanners.
- Add tests for directives inside each excluded context and for a real directive immediately after one.

### 4. Medium: contradictory SARIF fixability from `sanitise`

Reproduction with a sufficiently long em-dash-heavy document:

```sh
sanitise structural.md --structural --format sarif
```

For `structural-emdash-density`:

```text
driver.properties.fixability = "opt-in"
result.properties.fixability = "report-only"
result.fixes                  = absent
result.properties.replacement = absent
```

`slop-scan` correctly calls `.with_fixability_table(fixability_table())`. The umbrella builder at `crates/cli/src/bin/sanitise.rs:301-306` does not.

The document remains schema-shaped, and `ruleId` values all resolved to emitted driver rules. `ruleIndex` is absent, which SARIF permits. The defect is semantic inconsistency between the rule descriptor and its results.

Suggested fix:

```rust
Report::new(...)
    .with_ruleset_version(RULESET_VERSION)
    .with_fixability_table(fixability_table())
    .with_entries(entries)
```

Add an invariant test over every emitted result asserting that its fixability equals the matching driver rule. Also assert that `fixes` exists only when a replacement exists and the resolved fixability permits it.

### 5. Low: server body-limit mismatch and bad filename status

The router at `crates/server/src/lib.rs:574-585` has no `DefaultBodyLimit::max(max_body_bytes())` layer. A 2.2 MB request was rejected by Axum before `read_json` ran:

```text
HTTP 413
Failed to buffer the request body: length limit exceeded
```

This is secure in the conservative direction but contradicts the intended approximately 384 MiB JSON-envelope limit. If the extractor limit is later raised, decoded text must also be capped directly: a 1.5× envelope permits decoded input somewhat larger than `max_input_bytes()`.

Separately:

```json
{"file":"","name":"bad\u0000.txt"}
```

returns HTTP 500 because filesystem rejection is classified as internal rather than a client filename error.

Suggested fix:

- Install an explicit request-body limit on the router.
- Reject decoded data exceeding the correct per-kind input cap before writing or processing it.
- Reject NULs and other unusable basename characters in `safe_name`, returning HTTP 400.
- Document the actual wire-size and decoded-size limits.

### 6. Low: `cargo deny` is not an operational gate

`cargo deny check` reports that it cannot find a configuration and then rejects ordinary MIT, Apache-2.0, BSD and Unicode dependencies. It consequently produces a very large, unusable licence failure.

No new copyleft dependency conflict was identified manually, but there is no working automated evidence for that conclusion.

Suggested fix:

- Commit a `deny.toml` with the project’s accepted SPDX expressions, source policy and advisory exceptions.
- Document the rationale and expiry/review date for unavoidable exceptions.
- Run `cargo deny check` in CI only once its expected result is clean.

## Dependency security results

`cargo audit` now reports:

- `quick-xml 0.41`: the two first-pass DoS advisories are gone.
- `rsa 0.9.10`: `RUSTSEC-2023-0071`, medium, still transitive through `c2pa`; no fixed release is available.
- `proc-macro-error 1.0.4`: unmaintained warning, transitive through `static-regular-grammar`.

The RSA issue is not new and is not currently exposed as private-key signing in the reviewed code. Keep it recorded and reassess whenever the C2PA dependency moves.

## What was checked and found sound

- `cargo test --workspace --all-targets --all-features`: passed, including 115 CLI-library, 84 core, 175 media, 106 slop, 83 UK, 104 Unicode and associated integration/fixture tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- All thirteen executables return 2 on missing required paths or invalid arguments; the server returns 2 on bind failure.
- Empty input is handled without panic.
- Invalid UTF-8 round-trips through the dedicated text cleaner; scanning paths deliberately ignore undecodable units without unsafe indexing.
- Capped text/media commands reject oversized regular files before reading them.
- The umbrella tier implementation itself is sound: `--fix` applies mechanical edits; `--write` enables stylistic replacements; judgement and `no-fix-exists` findings never become edits; `replacement: None` cannot yield a patch.
- SARIF uses version `2.1.0`, the OASIS schema URI, valid severity levels, matching emitted `ruleId` descriptors, locations and fingerprints.
- SARIF fixes were emitted only for findings carrying a replacement and a fixable resolved policy.
- `properties.fixability` is present on rules and results; `noFixExplanation` is emitted for `no-fix-exists` results.
- Hostile TOML syntax, unknown keys and invalid severity values fail with exit 2; `deny_unknown_fields` prevents typo-based silent fallback.
- The `whatlang` wrapper implements the intended fail-open language policy: short, unreliable or unclassified spans are scanned as English.
- Malformed JSON produces HTTP 400 without panic.
- `../../etc/passwd` is reduced to a basename and remains inside the temporary request directory.
- The server returns exit 2 on bind failure.
- No production `unwrap`, `expect` or indexing panic reachable from ordinary malformed request or document input was confirmed; remaining instances are constant regexes, Clap-restricted enums or tests.
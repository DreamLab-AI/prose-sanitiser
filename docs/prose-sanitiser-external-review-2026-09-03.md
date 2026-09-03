# prose-sanitiser: external static review (received 2026-09-03)

Provenance: forwarded by the operator from an external reviewer who had no Rust
toolchain and reviewed by static inspection. Several items cite code that had
already been replaced on `rust/prose-sanitiser-hardening` by the time the review
arrived (hand-rolled PNG parsing, the PDF regex fallback, APP11 treated as C2PA),
so each item must be checked against current HEAD before acting. Kept verbatim
below for triage.

---

Overall: good bones, clear modules, extensive tests, thoughtful malformed-input handling, and better security awareness than most Rust utilities. The main weakness is that several comments promise stronger safety than the implementation currently provides. I would address correctness and fail-closed behaviour before stylistic refactoring.

## Highest-priority fixes

The subprocess runner can deadlock.
In common/proc.rs:132–154, stdout and stderr are piped but not drained until the child exits. A child producing more than the pipe buffer can block forever; writing stdin before starting the timeout can deadlock even earlier. Capture streams concurrently, impose output-size limits, start the timeout before I/O, and kill the whole process group on timeout. Also propagate setrlimit failures rather than silently running unlimited.

The HTTP service is vulnerable to resource exhaustion.

- Axum's Bytes extractor has its own default body limit, so the advertised 256 MB input probably fails around 2 MB unless DefaultBodyLimit is configured.
- Conversely, read_json checks only after buffering the entire body.
- Base64-decoded length is never checked against max_input_bytes().
- Inspection, regex processing, file I/O, external tools and GPU harnesses all run synchronously on Tokio worker threads.
- There is no concurrency limit.
- Binding publicly without an API key merely warns.

Prefer streamed application/octet-stream uploads spooled into a capped temporary file. At minimum add the Axum limit, validate decoded length, use spawn_blocking, add separate semaphores for CPU and GPU work, and refuse non-loopback binding without authentication.

Some cleaners can damage valid documents.

- container/pdf.rs:192–202 deletes bytes from a PDF without rebuilding xrefs, explicitly noting that offsets may break, but still returns success.
- container/ooxml/mod.rs:217–247 drops every customXml/ part, including legitimate business data, without removing all relationships that reference those parts.
- ODT cleaning can drop arbitrary embedded parts merely because their bytes contain a marker, leaving broken manifest references.
- image/png.rs:123–164 returns a partial PNG when it encounters a truncated chunk instead of refusing the operation.
- HTML/SVG mutation through regex can break markup containing > in quoted attributes, CDATA, unusual namespaces, or malformed-but-browser-valid HTML.

The general rule should be: mutate into a temporary output, validate independently, re-inspect, and only then commit. If validity cannot be guaranteed, refuse to clean rather than returning a degraded success.

Detection semantics are currently too broad for destructive cleaning.

C2PA does not mean "AI-generated"; it can attest camera capture or ordinary editing. Keep separate fields such as: provenance_present, generative_ai_assertion, vendor_hint, heuristic_text_signal.

Related false positives include:

- Every JPEG APP11 segment being labelled and removed as C2PA.
- c2patool_reports_manifest() deliberately treating the tool's usage banner as a confirmed manifest.
- Dropping an HTML description merely because its content mentions OpenAI.
- Dropping author: Claude, which may be a person rather than provenance.

Default cleaning should remove only structurally confirmed fields. Put heuristic/vendor-string deletion behind an explicit aggressive policy.

File-size and symlink protections are inconsistent.

open_nofollow() exists but is unused. Several image, container and slop entry points call fs::read() without caps; directory scans use metadata-then-read, which is a TOCTOU size-check bypass. Walkers also follow symlinks to files outside the requested root.

Create one read_capped_nofollow() function and use it everywhere. Stream directories rather than collecting every path first, and report unreadable/skipped files as an incomplete audit rather than silently treating them as clean.

default_file_mode() is unsafe in a multithreaded process.
Temporarily changing the process-wide umask creates a race. Atomic replacement also resets an existing file's permissions, so cleaning a 0755 script in place can turn it into 0644. Preserve the existing target mode and ownership where appropriate; use a deliberately restrictive mode for new files.

## Definite smaller bugs

- container/svg.rs:100–109: generator attributes are removed only when no other action occurred. An SVG with both metadata and generator attributes retains the attributes.
- common/io.rs:101–115: writing to stdout adds a newline that was not in the cleaned content, breaking byte-preserving behaviour.
- slop-scan can return more than 255 findings, after which as u8 wraps the exit code; 256 findings becomes success.
- Boolean HTTP options with the wrong JSON type silently become false; use typed Serde request structures with #[serde(deny_unknown_fields)].
- inspect_image() can promote has_c2pa from c2patool without updating has_ai_metadata, producing internally inconsistent reports.
- Website-audit documentation says external tools are not invoked for downloads, but inspect_remote → scan_file → inspect_image/inspect_container does invoke them.
- Sitemap indexes have no total sitemap/request limit: max_pages limits final pages but not a large tree of nested indexes.
- Redirect policy permits HTTP → HTTPS → HTTP because the expected origin is never advanced.
- Duration::from_secs_f64() can panic on negative, infinite or NaN CLI values; candidate counts, image sizes, steps and strengths also need bounded parsers.

## Structural improvement

The extensive use of serde_json::Value, tuples such as (bool, bool, Vec<String>, Value), and confidence inferred from display strings is creating real inconsistencies. Introduce typed domain objects (Finding { kind, confidence, evidence, location }, InspectionReport { format, findings, completeness }) and derive JSON at the boundary. This removes string-matching logic such as classify_finding_confidence() and makes invalid combinations harder to represent.

Split or feature-gate the crate into core text handling, format handlers, external adapters, crawler, CLI and server. Currently even a small text utility inherits the build and supply-chain surface of Axum, Tokio, TLS, ZIP and the website crawler.

## Testing additions

The existing unit and Python-parity tests are valuable, but many verify parity rather than correctness. Add: cargo-fuzz targets for PNG, JPEG, WebP, ZIP, HTML and URL parsing; properties (no panic, clean is idempotent, clean files remain parseable, clean input is unchanged); independent validation using image decoders, qpdf --check, ZIP validation and headless LibreOffice; real-router tests for HTTP limits, concurrency and malformed option types; adversarial subprocess tests with large stdout/stderr, blocked stdin and grandchildren; tests proving incomplete scans return a distinct non-zero status.

One Unicode-specific issue deserves tests too: script joiners are currently preserved after any non-ASCII letter, including accented Latin, and only the preceding character is examined. That is substantially broader than "inside complex scripts" and permits invisible carriers to survive.

The reviewer could not execute cargo test or Clippy (no Rust toolchain), so these findings are from static inspection. The first four items are strong enough that the current build should be treated as an excellent prototype, not yet a production-safe sanitiser.

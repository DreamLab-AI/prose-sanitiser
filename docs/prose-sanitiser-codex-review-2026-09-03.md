# Adversarial security and correctness review

Scope: `crates/unicode`, `crates/uk`, `crates/media`, `crates/core`, their READMEs, and `skills/prose-sanitiser/SKILL.md`. `cli` and `slop` were treated as non-blocking works in progress. No files were modified.

No Critical issue was found. Several High issues invalidate current “clean” or “lossless” guarantees.

## Findings

| Severity | File:line | Description |
|---|---|---|
| High | `crates/unicode/src/lib.rs:386` | Detected tag and zero-width payloads can be preserved intact by the cleaner |
| High | `crates/media/src/container/pdf.rs:250` | Malformed PDFs are “cleaned” through an unsafe raw-byte fallback that may corrupt the PDF or leave provenance behind |
| High | `crates/media/src/image/mod.rs:99` | Public media entry points bypass the advertised input-size caps and read attacker-controlled files wholly into memory |
| High | `crates/media/Cargo.toml:30` | `quick-xml` 0.37.5 has two known high-severity denial-of-service vulnerabilities |
| Medium | `crates/media/src/container/ooxml/mod.rs:67` | ZIP-bomb defence trusts attacker-declared sizes and does not cap actual decompressed output or entry count |
| Medium | `crates/uk/src/exclude.rs:241` | Direct quotations, indented code and relative/bare paths can be rewritten despite documentation saying they are protected |
| Medium | `crates/media/src/image/jpeg.rs:181` | Every APP11 segment is deleted and classified as C2PA, including non-C2PA JPEG XT/JUMBF data |
| Medium | `crates/media/src/container/ooxml/wordml.rs:204` | Malformed WordprocessingML is deliberately accepted and can be emitted as a successful cleaned document |
| Medium | `crates/*/Cargo.toml:6` | Declared MIT/Apache licensing conflicts with the repository’s documented AGPL-only first-party licensing decision |
| Low | `skills/prose-sanitiser/SKILL.md:34` | Capability and “never touches pixels” wording overstates what all code paths guarantee |

## Detailed findings

### 1. High: detected payloads can survive cleaning intact

Locations:

- `crates/unicode/src/lib.rs:386-389`
- `crates/unicode/src/decide.rs:264-271`
- `crates/unicode/src/stego.rs:292-346`
- `crates/unicode/src/stego.rs:349-377`
- `skills/prose-sanitiser/SKILL.md:34-44`
- `skills/prose-sanitiser/SKILL.md:103-106`

Detection and cleaning are independent. The detector recognises a complete malicious payload, but the cleaner makes per-character preservation decisions. Preserved “glue” does not advance `previous_kept`, so every subsequent tag or joiner continues to be treated as attached to the same legitimate base.

Reproduction:

```sh
python3 -c \
'import sys; sys.stdout.write("🏴"+"".join(chr(0xE0000+ord(c)) for c in "ussta")+chr(0xE007f))' |
target/debug/clean-text --stats
```

Observed result:

- Payload reported as `tag_block`, hex `7573737461`.
- `input_length: 7`
- `output_length: 7`
- `removed_count: 0`
- The malicious non-RGI tag sequence remains byte-for-byte intact.

The same problem exists for zero-width binary after a joining-script letter:

```sh
python3 -c 'import sys; sys.stdout.write("ب"+"\u200c"*8)' |
target/debug/clean-text --stats
```

Observed result:

- Payload reported as `zero_width_binary`, hex `ff`.
- `input_length: 9`
- `output_length: 9`
- `removed_count: 0`

This directly contradicts the dedicated-cleaner assertion that everything it touches is mechanically stripped.

Suggested fix:

- Compute payload carrier spans once with `stego::scan`.
- During cleaning, remove every carrier within a detected payload span before applying ordinary glue preservation.
- Explicitly exempt only payloads that `scan` classified as legitimate RGI sequences.
- Add invariant tests asserting that every reported payload span is absent after `clean_text`, including:
  - black flag plus non-RGI tags;
  - Persian/Arabic base plus eight joiners;
  - emoji plus variation-selector payload;
  - mixed carrier chains.

### 2. High: PDF fallback reports success while leaving metadata or corrupting offsets

Locations:

- `crates/media/src/container/pdf.rs:220-225`
- `crates/media/src/container/pdf.rs:247-275`
- `crates/media/src/container/mod.rs:208-215`
- `crates/media/src/container/mod.rs:257-272`
- `crates/media/README.md:48-51`

If `lopdf` rejects a PDF, `clean_pdf` either:

1. deletes regex-matched XMP bytes without repairing object lengths or xref offsets; or
2. copies the entire file unchanged.

Both return `Ok`. The first explicitly admits that offsets may be broken. The second can retain `/Info`, `/Metadata`, embedded C2PA objects, incremental revisions and arbitrary hidden content.

Reproduction:

```text
%PDF-1.7
1 0 obj
<< /Length 123 /Type /Metadata >>
stream
<?xpacket begin="x"?><x:xmpmeta>c2pa</x:xmpmeta><?xpacket end="w"?>
endstream
endobj
%%EOF
```

Make the xref malformed or omit it so `lopdf` fails. Cleaning removes the XMP substring, but the `/Length` value and structural offsets remain unchanged. If no recognised xpacket exists, the input is copied as-is and the operation still succeeds in `"mode": "copy"`.

Suggested fix:

- Fail closed when `lopdf` cannot build a complete object graph.
- Do not write a destination file in that case.
- Remove the regex mutation fallback entirely, or expose it only through an explicitly unsafe recovery command whose exit status is failure/degraded.
- After rewriting, require successful reparse and fail if `still_has_c2pa` or known structural metadata remains.
- Do not describe a degraded/copy result as cleaned.

### 3. High: media input caps are not enforced

Locations:

- `crates/media/src/io.rs:12-21`
- `crates/media/src/io.rs:44-66`
- `crates/media/src/image/mod.rs:95-110`
- `crates/media/src/image/mod.rs:199-203`
- `crates/media/src/container/mod.rs:107-110`
- `crates/media/src/container/mod.rs:188-191`
- `crates/media/src/container/pdf.rs:229-231`

Although `io.rs` advertises 256 MiB and 64 MiB caps, the actual public image/container inspect and clean paths use `std::fs::read` directly. The cap only protects text-input helpers.

A multi-gigabyte sparse PDF, JPEG or ZIP is therefore allocated in full before parsing. Several parsers then clone or reserialise it, multiplying peak memory:

- `Bytes::copy_from_slice` duplicates image input.
- PDF structured scanning constructs replacement buffers.
- C2PA validation may parse the same input again.
- ZIP handling retains all decompressed entries simultaneously.

Reproduction:

```sh
truncate -s 8G hostile.pdf
clean-file hostile.pdf
```

On a machine where the sparse file can be read, the process attempts to allocate/read the full logical size rather than rejecting it at 256 MiB.

Suggested fix:

- Centralise all file reads through a cap-enforcing `open_nofollow` reader.
- Check opened-handle metadata, then read through `take(cap + 1)` to close the stat/read race.
- Give each format a lower, explicit compressed and expanded budget.
- Avoid repeated full-buffer copies where parsers accept shared or borrowed storage.

### 4. High: known `quick-xml` denial-of-service vulnerabilities

Locations:

- `Cargo.toml:48`
- `crates/media/Cargo.toml:29`
- `crates/media/src/container/ooxml/wordml.rs:156-165`

`cargo audit` found:

- `RUSTSEC-2026-0194`: quadratic duplicate-attribute checking.
- `RUSTSEC-2026-0195`: unbounded namespace-declaration allocation.
- Installed version: `quick-xml 0.37.5`.
- Fixed version: `>=0.41.0`.

The quadratic attribute issue is directly reachable through hostile OOXML/XML parts when `.attributes()` is called. Other XML cleaners also expose the parser to attacker-controlled input.

A representative input is an element containing thousands of duplicate or near-duplicate attributes:

```xml
<w:p a00000="x" a00001="x" ... a99999="x" w:rsidR="00A1"/>
```

Suggested fix:

- Upgrade the workspace dependency to `quick-xml >=0.41.0`.
- Retest every event-driven scrubber after the upgrade.
- Retain independent input, expanded-size, attribute-count and nesting-depth budgets; a dependency fix should not be the only DoS boundary.

`cargo audit` also reported `RUSTSEC-2023-0071` in transitive `rsa 0.9.10`. That timing attack is less directly relevant to public-signature verification, but should be tracked with the C2PA dependency.

### 5. Medium: ZIP budget trusts declared sizes

Locations:

- `crates/media/src/container/ooxml/mod.rs:27-29`
- `crates/media/src/container/ooxml/mod.rs:58-80`
- `crates/media/src/container/ooxml/mod.rs:62`
- `crates/media/src/container/ooxml/mod.rs:71-73`

The implementation sums `file.size()` from the central directory, then calls uncapped `read_to_end`. A malicious archive can lie about uncompressed size or otherwise cause decompression to exceed the declared budget before an error is returned. `Vec::with_capacity(archive.len())` also leaves entry count unbounded.

Path traversal is not presently exploitable because entries are never extracted to their named filesystem paths. Dangerous names are merely copied back into the output archive.

Suggested fix:

- Read each entry through `take(remaining_budget + 1)` and reject if actual output exceeds the remaining budget.
- Use checked addition for budget arithmetic.
- Cap entry count and per-entry uncompressed size.
- Reject duplicate critical part names and dangerous path forms even though no extraction currently occurs; this avoids producing archives that become dangerous downstream.

### 6. Medium: UK exclusions do not protect several documented spans

Locations:

- `crates/uk/src/exclude.rs:24-30`
- `crates/uk/src/exclude.rs:171-203`
- `crates/uk/src/exclude.rs:225-239`
- `crates/uk/src/exclude.rs:241-275`
- `skills/prose-sanitiser/SKILL.md:70-80`
- `crates/uk/README.md:19`
- `crates/uk/README.md:84-88`

The quotation parser handles curly double quotes, guillemets and straight double quotes, but deliberately ignores single quotation marks. The code-span parser ignores four-space indented Markdown code. The link parser recognises URI schemes, `www` and email addresses, but not relative Markdown targets or bare file paths.

Concrete probe:

```text
He said ‘The color is red.’
He said 'The color is red.'
    color: red
[color](relative/path/color)
```

All four `color` occurrences were reported as `us-spelling`. The relative target produced an additional finding, so applying `--write` through the normal fix pipeline could change a quotation, code, or link target.

Suggested fix:

- Parse Markdown spans with a real Markdown event parser, or implement complete CommonMark handling for code and links.
- Protect curly single-quote pairs.
- Treat ASCII single quotes conservatively where they form balanced quotation spans rather than apostrophes.
- Protect link destinations independently of their scheme.
- Either protect indented code blocks or narrow the documentation so it does not promise that code and file paths are never touched.
- Add fix-level tests, not just detection tests, asserting byte identity for these spans.

### 7. Medium: all JPEG APP11 data is treated as C2PA and deleted

Locations:

- `crates/media/src/image/jpeg.rs:27-36`
- `crates/media/src/image/jpeg.rs:117-124`
- `crates/media/src/image/jpeg.rs:170-189`
- `crates/media/README.md:43-50`

The presence of any APP11 segment sets `has_c2pa = true`, and `strip_jpeg` deletes every APP11 segment without verifying a JUMBF box type, C2PA UUID or manifest label.

Reproduction: construct an otherwise valid JPEG with an APP11 segment containing JPEG XT data or a non-C2PA JUMBF box:

```text
FF EB 00 0C 4A 50 00 01 00 00 00 01 6E 6F 70 65
```

Inspection labels it C2PA and cleaning drops it despite the absence of a C2PA manifest.

This can remove unrelated auxiliary image information and makes the C2PA result a false positive.

Suggested fix:

- Parse APP11 packet framing and the reassembled ISO box structure.
- Delete only boxes whose content type/UUID identifies C2PA.
- Preserve non-C2PA APP11 segments unless the user explicitly requests removal of all metadata.
- Do not set `has_c2pa` solely because APP11 exists.

### 8. Medium: malformed WordprocessingML is accepted as successfully cleaned

Locations:

- `crates/media/src/container/ooxml/wordml.rs:200-205`
- `crates/media/src/container/ooxml/wordml.rs:211-229`
- `crates/media/src/container/ooxml/wordml.rs:291-295`
- `crates/media/src/container/ooxml/wordml.rs:399-403`

`check_end_names` is disabled, and EOF terminates processing successfully even while tags or a skipped deletion element remain open. The test suite explicitly approves `<w:p><w:r>` as valid input.

If an edit occurred before the malformed tail, the generated output is returned as a successful rewrite despite not being well-formed XML. If EOF occurs while `skipping` is active, all remaining content has been discarded.

Suggested fix:

- Enable end-name validation for complete OOXML parts.
- Track element depth and reject EOF unless depth and `skipping` are both clear.
- Only tolerate fragments in a separate internal API that cannot be used for DOCX parts.
- Reparse every modified part before assembling the output archive.

### 9. Medium: licensing is explicitly unresolved

Locations:

- `Cargo.toml:35`
- `crates/unicode/Cargo.toml:6`
- `crates/uk/Cargo.toml:6`
- `crates/media/Cargo.toml:6`
- `crates/unicode/README.md:125-132`
- `crates/uk/README.md:200-207`

The manifests declare `MIT OR Apache-2.0`, but the crate READMEs themselves state that ADR-016 made first-party code AGPL-3.0-only and that merely adding MIT/Apache licence files did not resolve the conflict.

This is a release-blocking provenance problem, not a dependency-licence problem. The examined direct dependencies are generally permissively licensed, and VarCon carries its own permissive notice.

Suggested fix:

- Obtain an explicit relicensing grant from all relevant copyright holders, or publish under the repository’s governing AGPL licence.
- Preserve the VarCon notice in packaged artefacts.
- Add automated licence policy checking only after the first-party licence position is resolved.

### 10. Low: documentation overstates guarantees across optional and degraded paths

Locations:

- `skills/prose-sanitiser/SKILL.md:34-44`
- `skills/prose-sanitiser/SKILL.md:70-80`
- `skills/prose-sanitiser/SKILL.md:103-106`
- `crates/media/src/image/mod.rs:153-184`
- `crates/media/src/image/mod.rs:214-253`
- `crates/media/src/container/pdf.rs:247-275`

The skill says metadata stripping is lossless and that pixel data is never touched. However:

- PDF degraded mode performs unsafe raw-byte surgery or copies metadata intact.
- `clean_image` can invoke `CtrlRegen` or diffusion purification on the destination, which intentionally changes pixels.
- Dedicated cleaners do not currently strip every Unicode payload they detect.

Suggested fix:

- Scope “lossless” and “never touches pixels” explicitly to successful container-only paths with pixel-removal options disabled.
- Treat degraded PDF processing as failure, not sanitisation.
- State that post-inspection is evidence about known embedded carriers, not proof of anonymity or complete provenance removal.
- Retain the existing, sound caveat that statistical watermarks and durable soft bindings cannot be proven removed.

## Checks that were sound

- Rust rejects overlong UTF-8 and surrogate encodings at `&str` boundaries; arbitrary invalid input bytes are preserved through the `Unit::Raw` surrogate-escape representation.
- No unchecked indexing or production `unwrap` panic was found in the reviewed parsers on ordinary malformed input.
- Valid PNG/JPEG/WebP test fixtures preserve compressed pixel data and decoded pixels across supported metadata surgery.
- Clean PNG/JPEG/WebP inputs return byte-identically when no removal is performed.
- WebP cleaning rejects unexplained trailing data rather than silently dropping it.
- PNG CRC and malformed segment handling are delegated to `img-parts` and malformed images are refused for rewriting.
- PDF’s normal `lopdf` path performs a full rewrite, preventing superseded incremental revisions from surviving as raw prior bytes.
- ZIP paths are not extracted, so archive path traversal does not currently reach the host filesystem.
- `safe_write_bytes` refuses a final symlink and uses a same-directory temporary file plus atomic rename; it does not follow the final symlink during replacement.
- Sense-dependent UK pairs never carry an applyable replacement.
- NFKC is opt-in and disabled by default.
- Bidi handling distinguishes source code from prose and preserves balanced controls in genuine RTL text.
- C2PA soft-binding documentation correctly says that stripping an embedded manifest does not prove unlinkability.
- Focused test suite passed: 425 unit/integration/fixture/doc tests across the four reviewed crates. The passing suite does not cover the adversarial cases above.

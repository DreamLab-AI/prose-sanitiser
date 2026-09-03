# prose-sanitiser CLI quick reference

Every binary exits **0** (clean / success), **1** (findings reported) or **2**
(tool error: bad arguments, unreadable input, failed write).

## `sanitise` umbrella

Runs every sanitiser layer over a file or tree on one confidence scale.

```
sanitise [OPTIONS] <PATH>
```

| Flag | Effect |
|:-----|:-------|
| `--format <FORMAT>` | Output format: `text` (default), `json`, `jsonl`, `sarif` |
| `--severity <SEVERITY>` | Minimum severity to report: `high`, `medium`, `low` (default) |
| `--fix` | Apply certain-mechanical findings (invisible Unicode, metadata) |
| `--write` | Apply certain-mechanical and high-confidence-stylistic findings. Implies `--fix`. Low-confidence-judgement findings are never applied |
| `--diff` | Show what would change, without writing |
| `--structural` | Also report whole-document structural measures (em-dash density, Oxford-comma rate, sentence-length CV, etc.) |
| `--aggressive` | Report every character with an ASCII confusable prototype, not only mixed-script flags. Flags honest Greek and Cyrillic prose |
| `--normalize-spaces` | Offer to rewrite exotic spaces (U+00A0, U+202F) to U+0020. Off by default; exotic whitespace is always *reported* |
| `--no-language-filter` | Scan every span regardless of detected language |
| `--no-suppressions` | Ignore HTML-comment suppression directives |
| `--oxford` | Use Oxford -ize spelling |
| `--disable <RULE>` | Rule to skip; repeatable |
| `--config <PATH>` | Configuration file path (default: nearest `.prose-sanitiser.toml`) |

---

## Layer A: invisible Unicode

### `inspect-text`

Report invisible Unicode, space homoglyphs and smuggled payloads in text.

```
inspect-text [OPTIONS] [PATH]
```

| Flag | Effect |
|:-----|:-------|
| `--json` | JSON report |
| `--aggressive` | Also flag Latin confusable and fullwidth lookalikes |
| `--strip-emoji-glue` | Paranoid mode: flag all load-bearing invisibles too (emoji glue, script joiners, flag tags) |
| `--force-text` | Scan even when the input looks like a binary container |

`PATH` defaults to `-` (stdin).

### `clean-text`

Strip invisible Unicode and normalise space homoglyphs.

```
clean-text [OPTIONS] [PATH]
```

| Flag | Effect |
|:-----|:-------|
| `-o, --output <PATH>` | Output path (default: stdout) |

`PATH` defaults to `-` (stdin).

---

## Layer B: image and container provenance

### `inspect-image`

Show C2PA manifests, EXIF, XMP and AI-related metadata in PNG, JPEG or WebP.

```
inspect-image [OPTIONS] <PATH>
```

| Flag | Effect |
|:-----|:-------|
| `--json` | JSON report |
| `--synthid-dir <DIR>` | Reverse-SynthID checkout root for optional pixel scoring |

### `clean-image`

Strip C2PA manifests and AI-related metadata. Pixels are never re-encoded.

```
clean-image [OPTIONS] <PATH>
```

| Flag | Effect |
|:-----|:-------|
| `-o, --output <PATH>` | Output path (default: `*.cleaned.*`) |
| `--in-place` | Overwrite the input (writes a `.bak` backup first) |

### `inspect-file`

Unified inspect: auto-detects text, image or container format.

```
inspect-file [OPTIONS] <PATH>
```

| Flag | Effect |
|:-----|:-------|
| `--json` | JSON report |
| `--aggressive` | Text: flag confusables |
| `--as <TYPE>` | Force type: `text`, `image`, `container`, `auto` (default) |
| `--force-text` | Scan as text even when the bytes look like a binary container |

### `clean-file`

Unified clean: auto-detects text, image or container format.

```
clean-file [OPTIONS] <PATH>
```

| Flag | Effect |
|:-----|:-------|
| `-o, --output <PATH>` | Output path |
| `--in-place` | Overwrite (writes `.bak` first) |
| `--json` | JSON report |
| `--nfkc` | Text: NFKC normalise |

---

## Audit sweeps

### `audit-dir`

Aggregate AI-provenance audit over a directory tree.

```
audit-dir [OPTIONS] <PATH>
```

| Flag | Effect |
|:-----|:-------|
| `--json` | JSON report |
| `--skip <DIRS>` | Comma-separated extra directory names to skip |

### `audit-website`

Aggregate AI-provenance audit over the URLs listed in a sitemap.

```
audit-website [OPTIONS]
```

| Flag | Effect |
|:-----|:-------|
| `--sitemap <URL>` | Sitemap URL to audit |
| `--base <URL>` | Base URL; discover the sitemap automatically |
| `--max-pages <N>` | Maximum pages to fetch (default: 200) |
| `--timeout <SECS>` | Per-page timeout (default: 15) |
| `--max-bytes <N>` | Per-page byte limit (default: 4 MiB) |
| `--json` | JSON report |

---

## Slop scanning

### `slop-scan`

Scan prose or Markdown for AI writing tells. Report-only; never writes.

```
slop-scan [OPTIONS] <PATH>
```

| Flag | Effect |
|:-----|:-------|
| `--severity <SEVERITY>` | Minimum severity to report |
| `--format <FORMAT>` | Output format: `text` (default), `json`, `jsonl`, `sarif` |
| `--structural` | Include whole-document structural measures |
| `--no-language-filter` | Scan every span regardless of detected language |
| `--no-suppressions` | Ignore suppression directives |
| `--disable <RULE>` | Rule to skip; repeatable |
| `--explain-rules` | Print the full rule table with `since`, `reviewed`, sources and changelog |

### `slop-detect`

Deterministic design anti-pattern detector (source code and configuration).

```
slop-detect [OPTIONS] <PATHS>...
```

| Flag | Effect |
|:-----|:-------|
| `--json` | Machine-readable output |
| `--rule <RULE>` | Run only this rule |
| `--ignore <RULE>` | Rule id to skip; repeatable |
| `--min-severity <LEVEL>` | `info` (default), `warn`, `error` |

---

## Rewrite hook

### `rewrite-text`

Layer B optional rewrite hook for statistical watermarks. Paraphrases text
through an external model backend. Lossy by design.

```
rewrite-text [OPTIONS] [PATH]
```

| Flag | Effect |
|:-----|:-------|
| `-o, --output <PATH>` | Output path (default: stdout or `*.rewritten.*`) |
| `--backend <BACKEND>` | `print-prompt`, `ollama`, `openai-compatible` |

`PATH` defaults to `-` (stdin).

---

## Configuration file

The CLI looks for `.prose-sanitiser.toml` or `prose-sanitiser.toml` in the
nearest parent directory. Every field is optional; unset keys keep their
defaults.

```toml
# .prose-sanitiser.toml
write = false                              # permit high-confidence-stylistic fixes
min_severity = "medium"                    # drop low-severity findings
oxford = false                             # -ise rather than Oxford -ize
language_filter = true                     # skip non-English paragraphs
suppressions = true                        # honour HTML-comment directives
disabled_rules = ["us-spelling", "hedge-words"]
```

Precedence: **built-in defaults < configuration file < command-line flags**.

Unknown keys are an error rather than a silent no-op, so a typo in a style
file is reported rather than ignored.

---

## Suppression comments

Inert in every Markdown renderer.

```markdown
<!-- prose-sanitiser-disable tier1-vocab -->
We delve into it deliberately, here.
<!-- prose-sanitiser-enable tier1-vocab -->

<!-- prose-sanitiser-disable-line hedge-words -->
This is essentially fine on this one line.

<!-- prose-sanitiser off -->
...whole region suppressed...
<!-- prose-sanitiser on -->

<!-- prose-sanitiser:ignore us-spelling -->
```

The single-line `slop-ignore` marker from the Python tool is also accepted.

---

## Output formats

| Format | Flag | Notes |
|:-------|:-----|:------|
| `text` | `--format text` | Human-readable, one line per finding. Default |
| `json` | `--format json` | The tool's own JSON report, with the rule table in `tool.driver.rules[]` |
| `jsonl` | `--format jsonl` | One JSON object per line, for streaming and scripting |
| `sarif` | `--format sarif` | SARIF 2.1.0, the format GitHub code scanning accepts. Carries confidence tier, `since`, `reviewed`, sources, and `partialFingerprints` per result |

All four formats include `RULESET_VERSION` so a finding can be traced to the
rule table that produced it.

# ADR-001: Skip arms gate on the owning module's alphabet, not a shared kind string

Date: 2026-11-05 · Status: accepted (operator approval 2026-10-25, dream case 4af4039e)

## Context

`check_text` in `crates/unicode/src/check.rs` runs an invisible-carrier pass and leaves some characters to better-informed rules: bidi controls, tag characters and steganographic variation selectors. It used to decide which characters to skip by their `kind` label alone. Two modules share the label `variation_selector` for different codepoint sets: the stego alphabets (U+FE00..=U+FE0F, U+E0100..=U+E01EF) and the Mongolian free variation selectors (U+180B..=U+180F). The invisible pass skipped both, but only the first has a stego rule. So a stray Mongolian FVS was stripped by the cleaner yet reported by nothing.

## Decision

A skip arm in `check.rs` defers a character only when the module that owns that kind would claim it, tested with that module's own alphabet predicate (for variation selectors, `stego::is_variation_selector`). Matching on the shared kind string alone never decides a skip.

## Consequences

- A character no other rule claims falls through to an invisible-carrier finding instead of vanishing between two surfaces. The fix is `check.rs` around line 308; the regression tests are in `fvs_gap_regression` in the same file.
- Every other skip arm (`bidi`, `tag_chars`) must be audited against its owning module's alphabets in the same way.

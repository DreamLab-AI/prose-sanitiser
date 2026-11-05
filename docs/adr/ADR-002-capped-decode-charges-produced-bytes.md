# ADR-002: A capped decode charges its budget for the bytes it produced, even when it fails

Date: 2026-11-05 · Status: accepted (dream case de4e83e7)

## Context

Container and image parsing in `crates/media` inflates compressed streams under two caps: a per-chunk allowance (`MAX_TEXT_INFLATED_BYTES`, 1 MiB) and a per-file budget (`MAX_TEXT_INFLATED_TOTAL_BYTES`, 8 MiB). A corrupt stream can inflate nearly its whole allowance before it fails. When a refusal charged nothing, a file of many corrupt streams could inflate far past the per-file cap, one chunk at a time.

## Decision

Every capped decode or expand path charges its budget for the bytes it actually produced, whether or not the stream then succeeds. A refusal is not free. The PNG text-chunk path implements this in `InflateBudget::inflate` (`crates/media/src/image/png.rs`), with a regression test that sixteen corrupt streams exhaust the budget.

## Consequences

The per-file cap is a true ceiling on decompression work, which bounds memory and time on hostile input. New decoders (other image chunks, OOXML parts, any future container) must charge the same budget the same way, with a test that a failing stream still pays.

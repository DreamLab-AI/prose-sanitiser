# ADR-003: Slop rule-table changes ship with their version bump

- Status: accepted
- Date: 2026-11-21
- Area: `crates/slop/src/rules/`

## Context

The slop rule tables are versioned (`RULESET_VERSION`), review-dated
(`RULESET_REVIEWED`) and carry a `CHANGELOG`. On 2026-11-11 the
claudish-structure deduplication merged as PR #15 with none of the three: the
table had changed while every stamp still read 2026.11.01, so two reports
produced either side of the change were indistinguishable from the versioned
record's point of view. The record was backfilled on 2026-11-16 as ruleset
2026.11.11 (PR #20, merged as 98d6df4), reconstructed from the PR description
rather than from a note written when the table moved.

## Decision

Any change to the slop rule tables - an edit to `RULES`, `TIER2`,
`TRANSITIONS`, the structural thresholds, or the severity, tier, sources or
dates of any rule - ships in the same change as:

1. a `RULESET_VERSION` bump,
2. a `RULESET_REVIEWED` refresh, and
3. a `CHANGELOG` entry of the same date saying what moved and on what evidence.

The doc comment on `RULESET_VERSION` states the rule; this file is its durable
record. A change that does not touch the tables needs no bump.

## Consequences

- A report's ruleset version alone tells a reader whether the table that
  produced it predates a given change, and the changelog says what moved.
- The bump, the review-date refresh and the changelog entry sit in the same
  diff as the table edit, so an omission is caught at review instead of after
  merge.
- `the_whole_table_was_reviewed_at_the_ruleset_date` and
  `the_changelog_leads_with_the_current_version` in
  `crates/slop/src/rules/tests.rs` fail the checks when the review date or the
  changelog drifts from the version, which enforces parts 2 and 3
  mechanically.

Approved by the operator on 2026-11-16 and reconfirmed on 2026-11-19, which
asked for this file to be written by the next night working in this area.

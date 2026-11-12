# Dream-cycle ledger

One row per night the dream engine runs on this repository (nominated 2026-10-24).

| date | slot | verdict | finding |
|---|---|---|---|
| 2026-10-25 | unicode-surgery | check_text was silent on stray Mongolian FVS: skip outran stego's alphabets | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/1 | yes | ACCEPT |  | 8b60b3495096 |  |  |  |
| 2026-10-26 | slop-rules | changelog over-claimed 3 added markers (navigate/tackle/myriad); note corrected | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/2 | yes | ACCEPT |  | 21bd8800c798 |  |  |  |
| 2026-10-28 | media-provenance | no crates/media source shown; media-provenance deep had no code to audit | NONE | NONE | yes | INCONCLUSIVE |  | d31b94e66abe |  |  |  |
| 2026-10-29 | cli-and-server | lexical_divergence read wholly different one-word texts as identical; now 1.0 | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/3 | yes | ACCEPT |  | e4399f4d5325 |  |  |  |
| 2026-10-30 | unicode-surgery | VETOED: tag_chars skip arm now gates on stego::is_tag_char; U+E0002..E001F… | NONE | VETOED | yes | REJECT |  | b39364e576bc |  |  |  |
| 2026-10-31 | slop-rules | VETOED: honest* out of tier1; in-other-words/put-simply fire one rule… | NONE | NONE | yes | BLOCKED-ENV |  | b5d9fdb337c4 |  |  |  |
| 2026-11-01 | slop-rules | 2026.11.01: honest* out of tier1; in-other-words/put-simply fire one rule | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/6 | yes | ACCEPT |  | 90d911623d06 |  |  |  |
| 2026-11-02 | uk-english | Gazetteer::spans panicked on caller names with a multi-byte first char; fixed | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/7 | yes | ACCEPT |  | 5c2d014eec19 |  |  |  |
| 2026-11-03 | media-provenance | InflateBudget never charged corrupt-stream inflate work; error path now pays | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/8 | yes | ACCEPT |  | b95c03e2cc5d |  |  |  |
| 2026-11-04 | cli-and-server | VETOED: audit: unreadable files get kind "error", no longer counted as scanned… | NONE | VETOED | yes | REJECT |  | e063e1cb5e79 |  |  |  |
| 2026-11-05 | unicode-surgery | is_joining_letter kept joiners after NFD Latin (U+02B0..036F etc); now excluded | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/9 | yes | ACCEPT |  | 69ded3fb83a4 |  |  |  |
| 2026-11-06 | slop-rules | negative-parallelism/tricolon densities fired below the 250-word minimum; gated | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/11 | yes | ACCEPT |  | e75290ea7740 |  |  |  |
| 2026-11-07 | uk-english | bare sense pair after a copula (is/was/be) was scored as a verb; now a noun | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/12 | yes | ACCEPT |  | 16fd64aaede7 |  |  |  |
| 2026-11-08 | media-provenance | inspect_webp missed C2PA markers outside C2PA/XMP/EXIF chunks; byte-scan added | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/13 | yes | ACCEPT |  | 64a360f89d9c |  |  |  |
| 2026-11-09 | cli-and-server | Environment fault, hypothesis untested: connected node annexe unhealthy: SSH com | NONE | NONE | blocked | BLOCKED-ENV |  | BLOCKED |  |  |  |
| 2026-11-10 | unicode-surgery | decomposed-Latin joiners (e+U+0301 then ZWNJ/ZWJ) pinned as must-strip in tests | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/14 | yes | ACCEPT |  | 6cab7c184466 |  |  |  |
| 2026-11-11 | slop-rules | 'to put it simply/another way' fired two rules; claudish-structure arm narrowed | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/15 | yes | ACCEPT |  | 8d1c16b4110a |  |  |  |
| 2026-11-12 | uk-english | have/has/had before a bare sense pair now reads as noun (copula branch) | NONE | https://github.com/DreamLab-AI/prose-sanitiser/pull/16 | yes | ACCEPT |  | a838e311103c |  |  |  |

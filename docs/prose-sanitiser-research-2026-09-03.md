# prose-sanitiser: state-of-the-art research and design brief

Date: 2026-09-03. Scope: research input for hardening and publishing the Rust crate
`prose-sanitiser`, ported from the Python skill at
`agentbox/skills/prose-sanitiser`. UK English throughout. No em-dashes.

Audience: the engineers who will redesign the crate before a crates.io publication.
Everything below is sourced. Where evidence is weak or contested, it says so.

---

## 0. Executive summary: what changed since the Python tool was written

Four findings should reset the design.

1. **Anthropic now watermarks Claude's text output.** Models launched on or after
   2 August 2026 embed a statistical sampling watermark, applied globally, per
   [Anthropic's own announcement](https://www.anthropic.com/news/claude-text-watermark).
   Anthropic states explicitly that "nothing is added to the text and there are no
   hidden characters". Google has watermarked Gemini text since October 2024
   ([Dathathri et al., *Nature* 634, 2024](https://www.nature.com/articles/s41586-024-08025-4)).
   The dominant provenance mark on text produced today is therefore **not** an
   invisible Unicode carrier. It is a sampling watermark that no third party can
   detect or remove without the vendor key. The crate's headline claim must move.

2. **There is no verified vendor Unicode watermark in any major LLM.** The
   U+202F narrow no-break space seen in GPT-4o-class output is best read as a
   tokenisation or training-data artefact, not a deliberate mark. OpenAI's own
   [provenance post](https://openai.com/index/understanding-the-source-of-what-we-see-and-hear-online/)
   confirms its text watermarking method was developed but **shelved**. Invisible
   Unicode in text is overwhelmingly a *third-party injection* problem (prompt
   smuggling, supply-chain attacks, detector evasion), not a vendor-watermark
   problem. That is still worth solving, but it is a different product story.

3. **The de-facto strongest Unicode carrier is now the variation-selector chain.**
   Paul Butler's [Smuggling arbitrary data through an emoji](https://paulbutler.org/2025/smuggling-arbitrary-data-through-an-emoji/)
   (February 2025) maps the 256 variation selectors onto byte values, so any base
   character followed by a selector chain carries an arbitrary byte string that
   survives copy and paste. It was used in the real *os-info-checker-es6* npm
   supply-chain attack. The current crate strips these characters but does not
   **decode and report the payload**, which is the more useful capability.

4. **The crate's UK-English rule is a single regex and is unsafe.** `src/slop/rules.rs`
   implements US-to-UK spelling as one flat alternation including `license`,
   `meter`, `catalog` and `fulfill`, with no sense disambiguation, no proper-noun
   protection and no code-span exclusion. It will flag "a driving licence issued to
   license a doctor", "gas meter", "dialog box" and "World Health Organization".
   This is the single largest correctness defect in the crate.

---

## A. Findings by area

### A1. AI text detection and "slop" tells

**Lexical markers are real, well quantified, and still live in 2026.** The
methodological anchor is [Kobak et al., "Delving into LLM-assisted writing in
biomedical publications through excess vocabulary"](https://arxiv.org/abs/2406.07016)
(published in [*Science Advances* 11, eadt3813, July 2025](https://doi.org/10.1126/sciadv.adt3813)).
The method is counterfactual frequency: extrapolate a 2024 baseline from the
2021-2022 trend across 15 million PubMed abstracts, then measure the gap. Results:
300-plus words showed unprecedented frequency jumps in 2024 against roughly 190 at
the 2021 COVID peak; the excess shifted from noun-dominated (79.2 per cent pre-2024)
to verb and adjective dominated (66 per cent verbs, 14 per cent adjectives); at
least **13.5 per cent of 2024 PubMed abstracts show LLM involvement**. The word list
and yearly counts are open at
[berenslab/llm-excess-vocab](https://github.com/berenslab/llm-excess-vocab).

[Juzek and Ward, "Why Does ChatGPT 'Delve' So Much?"](https://arxiv.org/html/2412.11385v1)
(COLING 2025) isolates 21 focal words and reports per-million increases: "delves"
from 0.21 to 14.38 opm (+6,697 per cent), "delved" +2,240 per cent, "showcasing"
+1,396 per cent, "boasts" +918 per cent, "underscores" +904 per cent. The paper
tests causes and finds the evidence consistent with RLHF preference amplification
rather than architecture or pretraining data, though the authors hedge causation.

[Liang et al., "Monitoring AI-Modified Content at Scale"](https://arxiv.org/html/2403.07183)
(ICML 2024) uses a distributional mixture estimator rather than word counting, and
finds **6.5 to 16.9 per cent of ICLR/NeurIPS peer-review text** substantially
LLM-modified, with no comparable signal in Nature Portfolio reviews.

**Has "delve" decayed?** Contested, and the crate should not bet on a single word.
Pangram (a commercial vendor, quoted via
[SFGate](https://www.sfgate.com/tech/article/chatgpt-change-writing-22404681.php))
says "delve" is on its way out. Against that, the
[Pew Research Center Data Labs analysis (Bestvater, 20 August 2026)](https://www.pewresearch.org/data-labs/2026/08/20/how-much-of-the-internet-is-written-with-ai/)
tracked a fixed 27-word list over roughly 490,000 Common Crawl pages and found the
category **more than doubled** between January 2023 and January 2026. Net position:
individual flagship words may be suppressed by frontier labs, but the vocabulary
class as a population signal was still growing through mid-2026. **Design
implication: the word table must be data-versioned and refreshable, not frozen.**

**Structural tells now have real measurement, from the same Pew study.** Per 10,000
words, January 2023 to January 2026: em-dashes 5.79 to 11.19 (roughly doubled);
Oxford commas 34.04 to 55.51 (+63 per cent); AI-typical vocabulary 11.94 to 26.02;
negative parallelism ("it's not just X, it's Y") 0.87 to 2.36 (nearly tripled, but
rare in absolute terms). This is the strongest quantitative support for the
"em-dash means AI" claim, but **only as a population-level signal**. No single
marker identifies a document. Pew used one detector (Pangram), so the result is
somewhat detector-dependent.

Weaker evidence: burstiness and sentence-length variance are well grounded as
*detector inputs* but lack isolated peer-reviewed effect sizes. Tricolons, uniform
paragraph length and list-of-three formatting are widely observed practitioner
heuristics (see the [Wikipedia "Signs of AI writing"](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing)
catalogue, CC BY-SA, maintained by
[WikiProject AI Cleanup](https://en.wikipedia.org/wiki/Wikipedia:WikiProject_AI_Cleanup))
but are **not** backed by a measurement study comparable to Pew's. Mark them as
low-confidence in the rule table.

**Detector mechanisms.**

| Detector | Statistic computed | Reported performance |
|---|---|---|
| [DetectGPT](https://arxiv.org/abs/2301.11305) (ICML 2023) | Log-probability curvature under mask-fill perturbation | Baseline; expensive (many forward passes) |
| [Fast-DetectGPT](https://arxiv.org/abs/2310.05130) (ICLR 2024) | Conditional probability curvature via sampling, not perturbation | 340x speedup; white-box AUROC 0.9887 vs 0.9554; black-box on ChatGPT/GPT-4 0.9338 vs 0.7225 |
| [Binoculars](https://arxiv.org/abs/2401.12070) (ICML 2024) | log-PPL(observer) / log-cross-PPL(observer scoring performer) | Paper: >90 per cent detection at 0.01 per cent FPR. Independent re-test: AUROC 0.87 on GPT-4o, **TPR@1%FPR only 0.14-0.47** |
| [Ghostbuster](https://arxiv.org/abs/2305.15047) (NAACL 2024) | Structured search over features from weaker LMs, then linear classifier | 99.0 F1 in-domain; +7.5 F1 cross-domain over prior best |
| RADAR (NeurIPS 2023) | Adversarially co-trained detector vs PPO paraphraser | AUROC 0.857 under unseen paraphrase; **-86.89 per cent TPR@1%FPR under targeted adversarial paraphrase** |
| GPTZero | Perplexity plus burstiness (proprietary) | No peer-reviewed evaluation |
| OpenAI AI Text Classifier | Withdrawn 20 July 2023 | **26 per cent TPR at 9 per cent FPR**, per OpenAI |

The critical methodological point, from
[A Practical Examination of AI-Generated Text Detectors (NAACL 2025 Findings)](https://aclanthology.org/2025.findings-naacl.271.pdf):
**AUROC is misleading for this task.** High AUROC coexists with near-zero true
positive rate at the low false-positive thresholds any real deployment requires.
Report TPR@1%FPR, not AUROC.

**False positives and fairness.** [Liang et al. 2023](https://arxiv.org/abs/2304.02819)
(*Patterns*) tested seven detectors on 91 TOEFL essays and 88 US eighth-grade
essays: near-perfect on native writers, but a **61.3 per cent average false-positive
rate on TOEFL essays**; 97.8 per cent flagged by at least one detector; 19.8 per cent
flagged unanimously by all seven. All were human-written. Vocabulary-enhancing the
essays dropped the FPR to 11.6 per cent, implicating perplexity as the driver.
[Weber-Wulff et al. 2023](https://link.springer.com/article/10.1007/s40979-023-00146-z)
found none of 14 commercial tools exceeded 80 per cent accuracy across a full matrix.

**No study found measures false positives on British English specifically.** That
is a genuine evidence gap and an opportunity: it is the natural evaluation the crate
should run and publish (see section D).

**Evasion.** [SilverSpeak (ACL 2025 GenAI-Detect workshop)](https://aclanthology.org/2025.genaidetect-1.1.pdf)
shows that replacing 5 to 20 per cent of Latin characters with homoglyphs collapses
seven detectors from mean MCC 0.64 to **-0.01**, i.e. to chance. RAID independently
includes homoglyph and zero-width-space as two of its adversarial categories. The
mitigation SilverSpeak proposes is input-side: **Unicode normalisation and
character-set restriction before scoring**. That is precisely what this crate does,
which reframes it usefully: the crate is a **detector-hardening preprocessor**, not
only an evasion tool.

**Ethics.** [COPE's position](https://publicationethics.org/guidance/cope-position/authorship-and-ai-tools)
is that AI tools cannot be authors and use must be disclosed in methods; routine
grammar and spelling assistance does not require disclosure. The
[EU AI Act Article 50](https://artificialintelligenceact.eu/article/50/) transparency
duties apply generally from **2 August 2026**, with a grace period to 2 December 2026
for machine-readable marking. Article 50(4) exempts AI-generated text that underwent
**genuine human editorial review with a named person holding editorial
responsibility**. That exemption is the crate's ethical home: it supports a human
editor taking responsibility for a text, which is a lawful and disclosed workflow.

The defensible line: **legitimate editing improves the text and enforces a house
style regardless of who or what drafted it; evasion targets a specific detector's
signature.** The crate should refuse to market itself on detector-defeat metrics.

### A2. Invisible Unicode and steganography

**Carrier classes and what each is legitimately for.**

| Class | Codepoints | Legitimate purpose |
|---|---|---|
| Zero-width | U+200B, U+200C, U+200D, U+FEFF, U+2060 | Joining control in shaping scripts; U+2060 is Unicode's recommended mid-text no-break marker |
| Tag block | U+E0000-U+E007F | Deprecated in 5.1; U+E0020-U+E007F un-deprecated in 8.0/9.0 for [emoji tag sequences](https://www.unicode.org/charts/PDF/UE0000.pdf) (England, Scotland, Wales flags) |
| Variation selectors | U+FE00-U+FE0F, U+E0100-U+E01EF | Glyph-variant and emoji-presentation selection |
| Bidi controls | U+202A-U+202E, U+2066-U+2069, U+200E/F | [UAX #9](https://unicode.org/reports/tr9/) bidirectional algorithm |
| Exotic whitespace | U+00A0, U+202F, U+2000-U+200A, U+205F, U+3000, U+1680 | Genuine typographic spacing with distinct widths |
| Soft hyphen | U+00AD | Reclassified Pd to Cf in Unicode 4.0; invisible unless a break occurs |
| Hangul fillers | U+3164, U+FFA0 | Default-ignorable but **non-zero width**; used to bypass empty-string validation |

Note a common error the current tables avoid: **U+180E was reclassified from
whitespace to a plain letter-class character in Unicode 6.3** and is no longer
invisible whitespace.

**The Trojan Source attack** ([Boucher and Anderson, arXiv 2111.00169](https://arxiv.org/abs/2111.00169),
CVE-2021-42574) uses bidi controls to make source code display in one logical order
and compile in another, across C, C++, C#, JavaScript, Java, Rust, Go, Python, SQL,
Bash and Solidity. It triggered out-of-band security releases in GCC, Clang and
rustc 1.56.1. The standards response is
[UTS #55, Unicode Source Code Handling](https://www.unicode.org/reports/tr55/)
(Revision 5, Version 2, 2024-01-29). **Design implication: bidi controls should be
rejected outright in source-code contexts but preserved in prose contexts.** The
crate currently applies one policy to both.

**Vendor evidence, stated honestly.**

- **VERIFIED, Anthropic**: statistical watermark on models launched from 2 August
  2026, applied globally for [EU AI Act](https://artificialintelligenceact.eu/article/50/)
  compliance, described as "a version of the SynthID-Text approach". Detection API
  is private preview, gated to regulators, researchers and fact-checkers.
  ([Anthropic](https://www.anthropic.com/news/claude-text-watermark),
  [TechCrunch](https://techcrunch.com/2026/08/11/anthropic-says-it-will-watermark-text-generated-by-its-ai-models/))
- **VERIFIED, Google**: SynthID-Text in Gemini production since October 2024,
  validated across roughly 20 million responses with no significant quality impact
  ([*Nature* 634](https://www.nature.com/articles/s41586-024-08025-4)); open-sourced
  as `SynthIDTextWatermarkLogitsProcessor` in
  [Hugging Face Transformers 4.46.0+](https://ai.google.dev/responsible/docs/safeguards/synthid).
- **NOT DEPLOYED, OpenAI**: method developed, shelved. OpenAI's stated reasons are
  fragility to "globalized tampering" and disproportionate impact on non-native
  English speakers ([OpenAI](https://openai.com/index/understanding-the-source-of-what-we-see-and-hear-online/)).
- **INCIDENTAL, not a watermark**: U+202F in GPT-4o-class output. No primary source
  confirms deliberate marking. Best read as a tokenisation or typographic artefact.

**Unicode security data.** [UTS #39](https://www.unicode.org/reports/tr39/) stands at
Revision 32, Version 17.0.0 (2025-09-04). It defines `confusables.txt`,
`IdentifierStatus.txt`/`IdentifierType.txt`, restriction levels from ASCII-Only to
Unrestricted, mixed-script detection, and the **skeleton algorithm**, which is
**NFD-based, not NFKC-based**: NFD, remove Default_Ignorable characters, substitute
confusable prototypes, re-apply NFD. A `bidiSkeleton` variant accounts for
reordering.

**Normalisation.** Per [UAX #15](https://www.unicode.org/standard/reports/tr15/tr15-21.html),
NFC and NFD are canonical and lossless. **NFKC and NFKD are lossy by design**: they
expand ligatures, strip superscripts, fold full-width and half-width forms, collapse
Roman numerals and circled digits, and convert no-break space to plain space. None
of the forms are closed under concatenation. The standard recommendation
([W3C charmod-norm](https://www.w3.org/International/wiki/CharmodNormProposal2013),
[PRECIS RFC 8264](https://datatracker.ietf.org/doc/html/rfc8264)) is **NFC for
storage and display, NFKC only for security canonicalisation and search**. RFC 8264
deliberately separates a width-mapping rule from the normalisation rule precisely
because Stringprep's mandatory NFKC proved too lossy. **The crate must never apply
NFKC to user-facing prose.**

**Must-preserve rules** (the precise legitimate-versus-suspicious test):

| Carrier | Preserve when | Suspicious when |
|---|---|---|
| U+200D | Both sides are well-formed `emoji_zwj_element` matching an RGI sequence in [emoji-zwj-sequences.txt](https://www.unicode.org/Public/17.0.0/emoji/emoji-zwj-sequences.txt) ([UTS #51](https://www.unicode.org/reports/tr51/) ED-16) | Between non-emoji characters, or in long unstructured runs |
| Mn/Mc combining marks | Always. Never blanket-strip | Never; only Cf-class controls are candidates |
| ZWNJ/ZWJ, Indic | Directly after a dead consonant or virama in an Indic run ([Unicode 17.0 ch.12](https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-12/)) | In Latin-only text |
| ZWNJ, Persian | Between morphemes of one Persian word | Elsewhere |
| Bidi controls | Balanced (matching PDF/PDI) and consistent with surrounding script direction, in **prose** | Unbalanced or nested overrides, and **any** occurrence in source code (Trojan Source) |
| U+FEFF | Byte offset 0 only (BOM) | Anywhere else; strip as stray ZWNBSP |
| Regional indicators | Well-formed pairs | Singleton |
| U+FE0F | Directly after a character with the `Emoji` property | Chained after an arbitrary base (the Butler smuggling signature) |

### A3. Statistical sampling watermarks

**Schemes.** [Kirchenbauer et al. (ICML 2023, arXiv 2301.10226)](https://arxiv.org/abs/2301.10226)
partition the vocabulary into a green list of fraction gamma seeded by hashing the
preceding h tokens plus a secret key, add logit bonus delta to green tokens, and
detect by a statistical test on green-token fraction.
[Aaronson's Gumbel/exponential scheme](https://scottaaronson.blog/?p=10032) (OpenAI,
2022) uses exponential-minimum sampling and is distortion-free in expectation.
[Christ, Gunn and Zamir](https://eprint.iacr.org/2023/1661.pdf) formalise
cryptographically undetectable watermarks. [SynthID-Text](https://www.nature.com/articles/s41586-024-08025-4)
replaces green-list biasing with **tournament sampling**, configurable to be
non-distortionary, and is the direct ancestor of Anthropic's Claude watermark.

**Detectability without the key: no.** Every deployed scheme requires the secret key
or PRF seed for its standard detection procedure. This is the design point, not an
oversight. Publicly-detectable watermarking is a separate research thread that
neither Anthropic nor Google deploys. The one demonstrated key-free-ish route is
**watermark stealing** ([Jovanović, Staab and Vechev, arXiv 2402.19361](https://arxiv.org/html/2402.19361v2)):
around 200,000 tokens of API queries approximate the green-list rules well enough to
spoof or scrub, for under 50 US dollars. That is a research-grade red-team exercise
requiring vendor API access and almost certainly breaching terms of service. **It is
not a feature this crate can ship.**

**Removal: only by changing tokens.** [DIPPER (Krishna et al., NeurIPS 2023)](https://arxiv.org/abs/2303.13408)
drops DetectGPT accuracy from 70.3 to 4.6 per cent at 1 per cent FPR; a
green-list-aware DIPPER variant pushes UNIGRAM-WATERMARK TPR
[below 10 per cent](https://aclanthology.org/2024.emnlp-main.1005.pdf). OpenAI itself
names an "emoji attack" (insert a special character between every word, then delete
it) as a circumvention route. The theoretical result is
[Zhang et al., "Watermarks in the Sand" (arXiv 2311.04378, ICML 2024)](https://arxiv.org/abs/2311.04378):
given a quality oracle and a perturbation oracle, a generic attack strips **any**
watermark meeting a basic low-FPR property, including private-key schemes, with only
minor quality loss. A rebuttal notes the attack needs many iterations and
near-perfect oracles, so practical robustness today exceeds the asymptotic case.

**Definitively: no lossless, token-preserving removal exists or is claimed anywhere
in the literature.** These watermarks are defined entirely by which tokens were
selected. Every demonstrated removal changes tokens. The open question is only how
small that change can be.

**Honest claim.** A tool without the vendor key **cannot detect** a
Kirchenbauer/SynthID/Aaronson-family watermark, and **cannot guarantee removal**. It
can honestly say only that paraphrase-style rewriting degrades such a watermark as a
side effect of changing tokens. Unkeyed stylometry (token-frequency skew, repeated
n-gram bias, entropy outliers) does not read the cryptographic watermark and is
demonstrably unreliable.

**Tooling.** [MarkLLM (arXiv 2405.10051)](https://arxiv.org/abs/2405.10051),
[THU-BPM/MarkLLM](https://github.com/THU-BPM/MarkLLM), unifies nine to twelve
algorithms with a 12-tool evaluation suite. It is Python and PyTorch throughout.
**There are no meaningful Rust implementations, and this is structural, not a
maturity gap**: these watermarks are injected inside the vendor's inference stack at
the logits stage, so a model-external tool cannot reimplement or invert them.

### A4. Image and document provenance

**C2PA is at specification 2.4 (April 2026)**
([spec.c2pa.org](https://spec.c2pa.org/specifications/specifications/2.4/specs/C2PA_Specification.html)),
adding a JSON-LD reporting serialisation and three assertions including AI
Disclosure. A manifest store is a JUMBF superbox
([ISO/IEC 19566-5:2023](https://cdn.standards.iteh.ai/samples/84635/398cb1ade93f4a11a0b55ed243811ee0/ISO-IEC-19566-5-2023.pdf))
labelled `c2pa`, type UUID `63327061-0011-0010-8000-00AA00389B71`, holding
assertions, a claim, and a COSE/CMS claim signature.

Embedding locations: **JPEG** APP11 (multi-segment JUMBF per JPEG XT); **PNG** the
`caBX` ancillary chunk, recommended before `IDAT`; **WebP/RIFF** a `C2PA` chunk;
**AVIF/HEIF** an item in the `meta` box; **MP4/BMFF** a `uuid` box before the first
`mdat` and `moov`, after `ftyp`; **PDF** an embedded file specification with
`AFRelationship = C2PA_Manifest`; **SVG** base64 in a `c2pa:manifest` element inside
`<metadata>`; **TIFF** via an IFD tag; **MP3/FLAC** an ID3v2 GEOB frame; **GIF** has
no native slot and uses a sidecar.

**The load-bearing honesty fact: stripping the manifest does not guarantee
unlinkability.** C2PA formally defines a **soft binding** as "a content identifier
that is either not statistically unique, such as a fingerprint, or embedded as an
invisible watermark", and a **Durable Content Credential** as one for which soft
bindings enable discovery in a manifest repository. The
[Soft Binding Resolution API](https://spec.c2pa.org/specifications/specifications/2.4/softbinding/Decoupled.html)
lets a validator that finds a stripped asset detect the surviving watermark, query a
key-value store for the right repository, and retrieve the original signed manifest.
Adobe runs a live implementation, the
[CAI Soft Binding Resolution API](https://developer.adobe.com/cai-soft-binding-api/),
which for TrustMark-watermarked assets returns the full manifest store from
`cai-manifests.adobe.com`. **The crate must say this explicitly.**

**Who emits C2PA in 2026**: OpenAI (DALL-E 3 onwards since February 2024; joined the
C2PA steering committee 19 May 2026 and now also adds SynthID as a second pixel-domain
layer, per [OpenAI](https://openai.com/index/advancing-content-provenance/));
Google (SynthID across Imagen, Veo, Lyria and Gemini text; C2PA on Gemini 3 Pro image
models from November 2025); Adobe Firefly (since launch, paired with TrustMark);
Microsoft Designer and Bing; cameras from Leica, Sony, Nikon, Canon and Samsung;
TikTok, YouTube and LinkedIn read and display incoming credentials.

**Can `c2pa-rs` strip a manifest? Effectively no.** The crate
([docs.rs/c2pa](https://docs.rs/c2pa/latest/c2pa/),
[contentauth/c2pa-rs](https://github.com/contentauth/c2pa-rs)) is pre-1.0, MSRV
1.88.0, **MIT OR Apache-2.0**. Reading and validating is fully supported via the
`Reader` API. Removal exists only as trait methods `CAIWriter::remove_cai_store_from_stream`
and `AssetIO::remove_cai_store` in `sdk/src/asset_io.rs`, reachable only through the
largely internal `jumbf_io` machinery. There is **no** top-level
`c2pa::remove_manifest(path)`, and `c2patool` has **no** `--remove` or `--strip`
flag. Note also that C2PA
[redaction](https://github.com/contentauth/c2pa-rs/blob/main/docs/redaction.md) is a
different thing: it removes one assertion from a prior ingredient's manifest inside
the signed chain, and is itself logged. **Conclusion: use `c2pa-rs` for read and
validate; implement removal at the container level ourselves.**

**Rust crate stack for byte-level metadata surgery.**

| Crate | Version | Licence | Role | Maturity |
|---|---|---|---|---|
| [img-parts](https://lib.rs/crates/img-parts) | 0.4.0 | MIT/Apache-2.0 | **Core.** Lossless JPEG/PNG/WebP/RIFF segment and chunk insert/delete | Stable, active (Aug 2025) |
| [little_exif](https://lib.rs/crates/little_exif) | 0.6.23 | MIT/Apache-2.0 | Typed Exif read **and write** across JPEG/JXL/HEIF/PNG/TIFF/WebP | Active (Jan 2026) |
| [kamadak-exif](https://lib.rs/crates/kamadak-exif) | 0.6.1 | BSD-2-Clause | Exif read and inspection; 196 dependents | Mature, quasi-standard |
| [nom-exif](https://docs.rs/nom-exif) | 3.0.0 | MIT | Read-side Exif plus video/audio track metadata | Active (May 2026) |
| [quick-xml](https://docs.rs/crate/quick-xml) | 0.38+ | MIT | Parse and rewrite XMP, OOXML, ODF payloads | Very active |
| [zip](https://lib.rs/crates/zip) (zip2) | 8.x | MIT | OOXML/ODF with format-preserving entry control | Very active |
| [lopdf](https://github.com/J-F-Liu/lopdf) | 0.36.0 | MIT | PDF object-model edit; **full rewrite on save** | Mature, active |
| [c2pa](https://docs.rs/c2pa) | 0.x | MIT/Apache-2.0 | C2PA **read and validate only** | Active, beta |
| rexiv2 | 0.10.0 | **GPL-3.0** + C dep | Avoid for a permissive crate | Maintained |
| mupdf-rs | 0.6.0 | **AGPL-3.0** | Avoid | Active |

`img-parts` is the crate that guarantees "delete a chunk, write the file back
unchanged except for that chunk", which is exactly the primitive needed.

**PDF gotcha, confirmed**: incremental updates append rather than rewrite, so a naive
metadata edit leaves the original `/Info` and `/Metadata` objects fully recoverable
earlier in the byte stream. `lopdf` performs a full object-graph rewrite on save,
which avoids the trap. This validates the current crate's use of `qpdf --linearize`
as a structural rewrite after `exiftool`, but `lopdf` removes the need for both.

**OOXML metadata** lives in `docProps/core.xml` (`dc:creator`, `cp:lastModifiedBy`,
`dcterms:created`/`modified`, `cp:revision`), `docProps/app.xml` (`Application`,
`Company`, `TotalTime`, a strong behavioural fingerprint), `docProps/custom.xml`,
`word/comments.xml`, and `w:ins`/`w:del` plus `rsid` session fingerprints inside
`word/document.xml`. ODF consolidates this into `meta.xml`. **ZIP round-trip
gotcha**: preserve each untouched entry's original compression method and relative
order; a naive re-zip that reorders alphabetically or recompresses is itself a
detectable "repacked by a non-Office tool" signal.

**Pixel-domain watermarks cannot be touched by this crate.**
[SynthID-Image](https://arxiv.org/html/2510.09263v1) is a post-hoc encoder-decoder
carrying a 136-bit payload, deployed across more than ten billion images.
[Stable Signature](https://arxiv.org/abs/2303.15435) fine-tunes the LDM latent
decoder so every output carries a 48-bit signature.
[Tree-Ring](https://proceedings.neurips.cc/paper_files/paper/2023/file/b54d1757c190ba20dbc4f9e4a2f54149-Paper-Conference.pdf)
embeds in the Fourier transform of the initial noise vector.
[TrustMark](https://arxiv.org/abs/2311.18297) is a GAN encoder-decoder with a 100-bit
payload above 40 dB PSNR.

The [WAVES benchmark](https://arxiv.org/html/2401.08573v3) (26 attacks) found
StegaStamp most robust, Stable Signature most vulnerable to regeneration, and
Tree-Ring severely broken by grey-box adversarial attack. 2025-2026 follow-ups show
guided diffusion regeneration drives **all** tested schemes to 0 per cent decode
success ([arXiv:2511.05598](https://arxiv.org/html/2511.05598v1)), but this requires a
pretrained diffusion model and GPU. And the arms race has moved on: a late-2026 paper
reports detecting **that removal occurred** at over 98 per cent TPR at 1 per cent FPR
across six removal methods ([arXiv:2605.09203](https://arxiv.org/html/2605.09203v1)).

**Correct claim: the crate performs lossless, verifiable removal of container-level
provenance metadata only. It cannot detect, identify or remove pixel-domain
watermarks, and stripping the container manifest does not defeat a durable Content
Credential.**

### A5. UK English enforcement

**VarCon is the right data source and its licence is clean.**
[VarCon](https://wordlist.aspell.net/varcon-readme/) (Kevin Atkinson, SCOWL project)
encodes region codes `A` American, **`B` British -ise**, **`Z` British -ize (Oxford)**,
`C` Canadian, `D` Australian, with variant-status tags (`.` equal, `v` variant,
`V` seldom, `-` possible, `x` improper). Critically it encodes the Oxford/Cambridge
split as **two distinct British categories in the same table**, which is exactly the
primitive needed for an `--oxford` flag. The licence is Atkinson's own permissive
notice, functionally BSD/MIT-equivalent with no copyleft
([Open Hub](https://openhub.net/p/scowl),
[app.aspell.net](https://app.aspell.net/create)). **Safe to vendor into an
MIT/Apache-2.0 crate.** `typos` already vendors it, which is a useful precedent.

**Licence trap: do not take en_GB Hunspell dictionaries from LibreOffice.** Those are
[GPL 2.0 / LGPL 2.1 / MPL 1.1 tri-licensed](https://github.com/hunspell/hunspell/blob/master/license.hunspell).
Mozilla had to re-derive its dictionaries from SCOWL in 2007 for exactly this reason.
Use SCOWL/VarCon directly, or [wooorm/dictionaries](https://github.com/wooorm/dictionaries)
`dictionary-en-gb` (MIT AND BSD).

**The -ise/-ize question.** [Oxford spelling](https://en.wikipedia.org/wiki/Oxford_spelling)
(`en-GB-oxendict`) uses `-ize` for Greek `-izein` verbs and is used by OUP, *Nature*
and the TLS. Cambridge University Press, the *Guardian*, the BBC and UK government
use `-ise`; the BNC ratio is roughly 3:2 in favour of `-ise`.

The **always -ise** list (where the ending is not the Greek suffix but part of a
longer root: *-cise* cutting, *-mise* sending, *-vise* seeing, *-prise* taking,
*-guise* form), cross-checked against
[World Wide Words](https://www.worldwidewords.org/qa-ise1.html) and the
[Chatham House style guide](https://www.chathamhouse.org/sites/default/files/ch-style-guide-for-the-journal-of-cyber-policy.pdf):
advertise, advise, apprise, arise, chastise, circumcise, comprise, compromise,
demise, despise, devise, disguise, enfranchise/disfranchise, excise, exercise,
franchise, guise, improvise, incise, merchandise, premise, prise (open), promise,
revise, supervise, surmise, surprise, televise. Enterprise and reprise are disputed
between lists but are Latinate `-prise` formations and safe to add. Useful
cross-check: none of these form nouns in `-isation`/`-ization`/`-ism`, with
*improvisation* the sole exception.

**The -yse case is unconditional.** *Analyse, catalyse, hydrolyse, paralyse, dialyse,
electrolyse, breathalyse, psychoanalyse* are `-yse` in **both** Oxford and general
British, because the root is Greek *lysis*, not `-izein`
([hull-awe.org.uk](http://hull-awe.org.uk/index.php/-lyse_-_-lyze) citing Hart's
Rules: "there is therefore no parallel with -ize- words"). No Oxford exception.

**Recommendation: default to `-ise`, gate `-ize` behind `--oxford`, and leave the
always-ise list and the `-yse` set untouched regardless of flag.**

**Traps that break naive rules.**

- **-our derivatives are irregular within British English.** Suffixes `-ary, -ous,
  -ific, -ious, -icidal, -igenic, -al, -imeter` **drop** the u (honorary, humorous,
  laborious, vigorous, colorimeter); `-s, -ed, -ing, -less, -ful, -ise, -able, -ist,
  -ism` **keep** it (colourful, honourable, colourist); `-ant, -ation` are optional
  (colo(u)ration). This is a lookup-table problem, not a suffix regex.
- **meter versus metre.** British English keeps *meter* for the measuring instrument
  and uses *metre* only for the SI unit
  ([metricationmatters.org](https://metricationmatters.org/docs/Spelling_metre_or_meter.pdf):
  "All English-speaking nations agree that meter can be used for an instrument").
  A blind `meter -> metre` corrupts every *gas meter*, *voltmeter* and *speedometer*.
  **The current crate's regex does exactly this.**
- **Double-L is four-way asymmetric.** UK doubles before vowel suffixes (travelled,
  modelling, cancelled) where US does not. But US **doubles** the root l in enroll,
  install, distil, instil, fulfill, skillful, willful where UK keeps single (enrol,
  instal, distil, instil, fulfil, skilful, wilful); and before `-ment` UK **never**
  doubles (enrolment, fulfilment, instalment) while US **always** does. **The current
  crate flags `fulfill` but has no rule for `fulfilment` versus `fulfillment`.**
- **licence/practice is a noun-verb split inside British English**, not a dialect
  swap: noun `-ce` (a driving licence, general practice), verb `-se` (to license a
  doctor, to practise medicine)
  ([Stroppy Editor](https://stroppyeditor.wordpress.com/2015/10/28/licence-or-license-practice-or-practise/)).
  **The current crate flags `license` unconditionally and will produce wrong advice
  half the time.**
- **sulfur is the reverse trap.** The Royal Society of Chemistry adopted *sulfur* in
  1992 to match IUPAC, and BSI followed in 1993
  ([Chemistry World](https://www.chemistryworld.com/opinion/sulfur-or-sulphur/3005631.article)).
  Do not "correct" it to *sulphur* in technical registers.
- **fetus** is standard in UK biomedical usage (92.5 per cent of UK-indexed papers
  per [the BMJ](https://blogs.bmj.com/bmj/2018/05/18/jeffrey-aronson-when-i-use-a-word-oe-ae-oe-ae-oh/)),
  even though *foetus* survives in lay British writing.
- **Sense-dependent pairs never to blind-replace**: program/programme (a computer
  program stays *program* in UK English), disc/disk, story/storey, tire/tyre,
  draft/draught, check/cheque, curb/kerb, meter/metre, dialog box.

**Never safe to apply blindly**: proper nouns and organisation names (World Health
Organization is `-ize` by charter; International Labour Organization is `-our`;
Department of Defense, Pearl Harbor, the Australian Labor Party, Rockefeller Center);
direct quotations; code identifiers, CSS properties (`color`), function names
(`initialize`, `analyze`, `serialize`), JSON/YAML keys, CLI flags (`--color`), file
paths, package names; URLs and email addresses; anything inside code fences, inline
code spans, HTML attributes or front matter; technical terms of art.

**Required architecture: span exclusion first, then sense disambiguation, then
confidence-tiered fixes.** Only VarCon-certified unconditional pairs with no
organisation-name collision may auto-fix; everything sense-dependent or
gazetteer-adjacent is report-only.

### A6. Prior art and API design

**Harper is the most consequential finding for build-versus-buy.**
[harper-core](https://lib.rs/crates/harper-core) (Automattic) is **Apache-2.0**,
actively released (2.5.0, June 2026), 81K SLoC, and **already supports British,
American, Canadian, Australian and Indian dialects** via a `Dialect` enum
([writewithharper.com](https://writewithharper.com/)). Its API shape is
`LintGroup::new_curated(dict, Dialect::American)` over a `Document`. **Recommendation:
do not reimplement general grammar and style linting. Depend on or interoperate with
harper-core, and keep prose-sanitiser focused on the three things Harper does not do:
invisible-Unicode and provenance surgery, AI-tell detection, and sense-aware UK
spelling with span exclusion.**

Other Rust prior art: [typos](https://github.com/crate-ci/typos) (MIT/Apache-2.0,
v1.49.0 Aug 2026) already vendors VarCon and is the best CLI-convention template;
[cargo-spellcheck](https://docs.rs/crate/cargo-spellcheck/0.8.14) has an **LGPL-2.1
licence trap** via its default nlprule feature, and nlprule is effectively legacy;
[spellbook](https://github.com/helix-editor/spellbook) is a ground-up Rust Hunspell
reimplementation from the Helix team but is **MPL-2.0** (usable as a dependency, not
vendorable); [hunspell-rs](https://lib.rs/crates/hunspell-rs) is a thin FFI shim last
released 2022.

Unicode-adjacent: [unicode-security](https://lib.rs/crates/unicode-security)
(MIT/Apache, implements UTS #39 skeleton, mixed-script and restriction levels, but
its "Implement all of UTS 39" tracking issue is **still open**, so coverage is
partial); [ICU4X](https://docs.rs/icu) `icu_normalizer` and `icu_properties` (the
Unicode Consortium's own Rust implementation; `icu_properties` alone has 42,988
dependents, and is a better-governed bet than the small crates for anything beyond
narrow use); [decancer](https://lib.rs/crates/decancer) (MIT, 221,529 codepoints,
bidi-aware); [whatlang](https://github.com/greyblake/whatlang-rs) (MIT, 69 languages,
lightweight) for a language pre-filter so UK rules never fire on non-English spans.
[unicode_skeleton](https://github.com/PeterReid/unicode_skeleton) is **unmaintained**
(last release 2017).

**Deslop prior art** is a crowded but shallow field. GitHub's
[ai-slop-detection topic](https://github.com/topics/ai-slop-detection) lists
`scanaislop/aislop` (TypeScript, 40-plus deterministic rules, MIT, explicitly no
LLM), `distil-labs/distil-ai-slop-detector` (a 270M fine-tune quantised to 242 MB,
in-browser), `hardikpandya/stop-slop` (MIT, banned-phrase and structural-cliche
catalogue), and `petergyang/no-ai-slop` (20-plus documented patterns). **None of them
touch the technical provenance layer.** That combination, deterministic slop
detection plus lossless provenance surgery plus sense-aware UK English, is a genuine
unoccupied niche and is the crate's distinguishing claim.

**API design conclusions.**

- **Diagnostics**: keep a rustc/clippy-style `file:line:col` human format as primary,
  add JSON Lines (the ripgrep and typos convention), and add
  [SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html) as
  a secondary format. [GitHub code scanning requires exactly 2.1.0](https://docs.github.com/en/code-security/reference/code-scanning/sarif-files/sarif-support),
  gzipped under 10 MB, with `runs[].tool.driver.rules[]` separated from
  `runs[].results[]` and `partialFingerprints` for deduplication. It is a small
  serialisation on top of an internal `Finding` struct. Worth it.
- Copy Vale's `Action` pattern: represent a fix **as data** (`Name`, `Params`), not
  as pre-applied text.
- **Exit codes**: 0 clean, 1 findings at or above the gate severity, 2 tool error.
  This matches shellcheck and Vale. Note `typos` inverts this (2 for findings), which
  is the less common idiom.
- **Never rewrite by default.** Report-only default; `--write` for unambiguous fixes
  only; `--diff` to preview. `typos` is the reference: ambiguous cases are never
  auto-fixed even under `--write`.
- **Suppression**: adopt Vale's HTML-comment toggle (`<!-- prose-sanitiser off -->`
  and `... on -->`) plus a single-line `<!-- prose-sanitiser:ignore RULE_ID -->`.
  HTML comments are inert in every Markdown renderer.
- **Split severity from confidence**, as [Semgrep](https://docs.semgrep.dev/kb/rules/understand-severities)
  does and as [clippy's lint groups](https://doc.rust-lang.org/clippy/lints.html)
  imply. Severity rates impact; confidence rates whether the pattern is right.
- **Library shape**: a config builder, `check(&Document) -> Vec<Finding>` that never
  mutates, and a separate `fix(&Document, &[Finding]) -> Patch` returning an
  applyable diff. That single core then serves the CLI, an LSP server (findings as
  code actions) and the SARIF exporter.

---

## B. Capability matrix: the honest claims

This table is the contract. Nothing outside column one should appear in the crate
description, README or `--help`.

### B1. Can detect **and** losslessly strip (verifiable by diff)

| Capability | Basis |
|---|---|
| Invisible Cf-class controls in text: zero-width family, tag block, variation selectors, bidi controls, exotic whitespace, soft hyphen, Hangul fillers | Deterministic codepoint classification with context rules |
| Variation-selector smuggled payloads, **including decoding the payload** | [Butler 2025](https://paulbutler.org/2025/smuggling-arbitrary-data-through-an-emoji/) byte mapping is fully specified |
| Homoglyph and mixed-script substitution | [UTS #39](https://www.unicode.org/reports/tr39/) skeleton and restriction levels |
| C2PA JUMBF manifests embedded in JPEG APP11, PNG `caBX`, WebP `C2PA`, PDF embedded-file, SVG `c2pa:manifest` | Container structure is normatively specified; deletion is byte-level |
| EXIF, XMP (including Extended XMP), IPTC/Photoshop IRB, PNG text chunks, `tIME`, GIF comment extension | Well-delimited container structures |
| PDF `/Info` and `/Metadata`, with full object-graph rewrite so prior incremental revisions do not survive | `lopdf` full rewrite; `qpdf --remove-info --remove-metadata` as cross-check |
| OOXML `docProps/core.xml`, `app.xml`, `custom.xml`, `word/comments.xml`, `w:ins`/`w:del`, `rsid`; ODF `meta.xml` | ZIP part deletion with preserved compression and ordering |

### B2. Can detect and report, but **must not** claim to strip

| Capability | Why |
|---|---|
| Statistical sampling watermarks (Kirchenbauer, SynthID-Text, Aaronson, Claude's Aug-2026 watermark) | Detection requires the vendor key ([SynthID *Nature*](https://www.nature.com/articles/s41586-024-08025-4), [Anthropic](https://www.anthropic.com/news/claude-text-watermark)). The crate can note *that a watermark is likely present given the source model*, nothing more |
| Pixel-domain image watermarks (SynthID-Image, Stable Signature, Tree-Ring, TrustMark, StegaStamp, DwtDct) | Each needs a proprietary trained decoder or diffusion inversion. [WAVES](https://arxiv.org/html/2401.08573v3) |
| Durable Content Credentials (C2PA soft binding plus cloud repository) | The crate cannot know whether a soft binding exists. [Soft Binding Resolution API](https://spec.c2pa.org/specifications/specifications/2.4/softbinding/Decoupled.html) |
| AI stylistic tells (lexical, structural, narrative) | Heuristic, not forensic. Population-level evidence only ([Pew 2026](https://www.pewresearch.org/data-labs/2026/08/20/how-much-of-the-internet-is-written-with-ai/)) |

### B3. Can only degrade, never remove, and must say so

| Capability | Honest wording |
|---|---|
| Statistical watermark "removal" via paraphrase | "Paraphrase changes tokens, which degrades any sampling watermark as a side effect. This is lossy, cannot be verified without the vendor key, and is not removal." [Zhang et al.](https://arxiv.org/abs/2311.04378) proves no lossless removal exists |

### B4. Must never touch

| Never modify | Rule |
|---|---|
| U+200D inside a well-formed RGI emoji ZWJ sequence | [UTS #51](https://www.unicode.org/reports/tr51/) ED-16 |
| Mn/Mc combining marks | Never blanket-strip; only Cf-class controls are candidates |
| ZWNJ/ZWJ after an Indic virama, or between Persian morphemes | Orthographically load-bearing |
| Balanced bidi controls in genuine RTL prose | Only reject them in source-code contexts (Trojan Source) |
| U+FEFF at byte offset 0 | It is a BOM there and only there |
| Content inside code fences, inline code, HTML attributes, front matter, URLs, file paths | Span exclusion runs before any rule |
| US spelling in proper nouns, organisation names, and direct quotations | Gazetteer plus quotation detection |
| `program` (computing), `meter` (instrument), `disk` (hard), `sulfur` (chemistry), `fetus` (medical), `dialog box` (UI) | Sense-dependent; report-only at most |
| Pixel data of any image | The crate does lossless container surgery only. Never re-encode |
| NFKC normalisation of user-facing prose | Lossy by design ([UAX #15](https://www.unicode.org/standard/reports/tr15/tr15-21.html)); NFC only |

---

## C. Recommended architecture

### C1. Crate layout

Split the current single crate into a workspace. The publication candidate is the
library; the binaries and the server stay unpublished or become thin wrappers.

```
prose-sanitiser-core     # no I/O, no subprocesses. Finding/Patch/Config types.
prose-sanitiser-unicode  # Layer A: classification, UTS#39, VS payload decoding
prose-sanitiser-uk       # VarCon-backed UK English with span exclusion + senses
prose-sanitiser-slop     # versioned rule tables, confidence-tiered
prose-sanitiser-media    # img-parts / lopdf / zip container surgery
prose-sanitiser          # CLI + SARIF/JSONL emitters
prose-sanitiser-server   # axum service (unpublished)
```

The reason for the split is licence and dependency hygiene: `-core`, `-unicode` and
`-uk` can be pure Rust with no C dependencies and no subprocesses, which is the part
worth publishing. `-media` pulls the heavier tree.

### C2. Key decisions

**Replace 888 lines of hand-rolled image parsing with `img-parts`.** `src/image/png.rs`
(331), `jpeg.rs` (292) and `webp.rs` (265) hand-parse chunk and segment structures.
[img-parts](https://lib.rs/crates/img-parts) (0.4.0, MIT/Apache-2.0) provides exactly
this with a `segments_mut()` API and byte-identical re-encode. This directly satisfies
the "no hand-rolled parsing of formats where a trusted crate exists" rule.

**Eliminate the `exiftool`, `c2patool` and `qpdf` subprocesses.** `little_exif` covers
Exif write, `quick-xml` covers XMP, `img-parts` covers chunk deletion, `lopdf` covers
PDF with a full rewrite. Keep `c2pa` (the crate) for **read and validate only**, since
its removal API is internal and undocumented. Keep the external tools as an optional
verification cross-check behind a feature flag, not as the implementation.

**Adopt VarCon as vendored data.** BSD/MIT-equivalent licence, already encodes the
A/B/Z dialect split. Build the `--oxford` flag directly on the `B` versus `Z` tags.
Ship the always-ise and always-yse lists as hand-verified overrides.

**Adopt UTS #39 properly.** Replace the 40-entry hand-written `LATIN_CONFUSABLES`
table with `unicode-security` skeleton and mixed-script detection, noting its UTS #39
coverage is partial, and consider `icu_properties` for the property lookups.

**Add a language pre-filter.** `whatlang` (MIT, lightweight) so UK spelling rules
never fire on non-English spans.

**Do not reimplement grammar checking.** Interoperate with
[harper-core](https://lib.rs/crates/harper-core) (Apache-2.0) rather than competing
with it.

**Three-tier confidence taxonomy**, orthogonal to severity:

| Tier | Contents | Auto-fix |
|---|---|---|
| `certain-mechanical` | Invisible Unicode, container metadata, homoglyphs | Yes, always; verifiable by diff |
| `high-confidence-stylistic` | VarCon unconditional dialect pairs, always-ise/yse | Behind `--write` only |
| `low-confidence-judgement` | Sense-dependent pairs, slop phrasing, organisation-adjacent tokens | **Never**; report only |

### C3. Licence position

Publishing as **MIT OR Apache-2.0** is achievable. Dependencies that keep it clean:
img-parts, little_exif, quick-xml, zip, lopdf, c2pa, unicode-security, icu, whatlang,
decancer, harper-core (Apache-2.0), VarCon data (BSD/MIT-like). Avoid: rexiv2
(GPL-3.0), mupdf-rs (AGPL-3.0), LibreOffice en_GB dictionaries (GPL/LGPL/MPL),
LanguageTool rules (LGPL, reference only), spellbook (MPL-2.0, dependency not
vendorable). Wikipedia-derived word lists are fine as *facts* but the article prose is
CC BY-SA and must not be copied verbatim.

---

## D. Evaluation plan

The crate currently has no measured precision or recall. That is the gap between a
working tool and a publishable one.

### D1. Corpora

| Corpus | Size | Licence | Use |
|---|---|---|---|
| [RAID](https://github.com/liamdugan/raid) | 10M+ documents, 11 LLMs, 11 genres, 12 adversarial attacks | MIT | **Primary.** Its adversarial split already includes homoglyph and zero-width-space attacks, which directly exercises Layer A |
| [M4GT-Bench](https://github.com/mbzuai-nlp/M4GT-Bench) | ~138K, 9 languages, 6 domains, 9 generators | Mixed per source, documented | Cross-domain generalisation |
| [MAGE](https://arxiv.org/html/2310.14724v3) | 103K human + 182K machine, 27 generators, 14 domains | Apache-2.0 | Generator diversity |
| [HC3](https://github.com/Hello-SimpleAI/chatgpt-comparison-detection) | ~58K human + 27K ChatGPT | CC BY-SA | Legacy baseline for comparability |
| LLM-DetectAIve | ~382K, labels for machine-humanised text | CC BY-SA-4.0 | **The only corpus modelling the "humanised" category**, which is what this crate produces |
| **UK human prose set (to build)** | Target 2,000 documents | Public-domain and permissively licensed sources | **The crate's distinguishing evaluation.** No existing corpus has a UK-English human subset |

The UK set is the important one: no benchmark surveyed has UK-English-labelled human
prose, and no published study measures detector or linter false positives on British
English. Build it from Hansard, UK government publications under the Open Government
Licence, pre-2020 Guardian and BBC style-guide-conformant text, and public-domain
British literature. Pre-2022 material only, so it is provably not LLM-influenced.

### D2. Metrics

1. **Layer A precision and recall** on RAID's homoglyph and zero-width adversarial
   splits. Target: recall above 0.99, precision above 0.99. This layer is
   deterministic, so anything less is a table bug.
2. **False-positive rate on legitimate Unicode.** Zero tolerance. Fixture set of
   emoji ZWJ sequences, Indic virama forms, Persian compounds, RTL prose, BOMs,
   regional-indicator flags. Any strip is a hard failure.
3. **UK-English precision on the UK human prose set.** Report the false-positive rate
   per rule. The `us-spelling` rule as it stands should be measured before and after
   the redesign; expect the current version to fail badly on *meter*, *licence*,
   *program* and organisation names.
4. **Byte-exact round-trip on untouched content.** For every supported format, a file
   with no provenance marks must come out byte-identical. Hash before and after.
5. **Image visual identity after metadata strip.** Decode both images and assert
   pixel-exact equality. Not PSNR, exact. If pixels changed, the tool re-encoded and
   that is a bug.
6. **OOXML and ODF validity after strip.** Open the output in a validator; assert
   compression method and entry order preserved for untouched parts.
7. **Slop rules: report TPR at 1 per cent FPR, never AUROC**, per the
   [NAACL 2025 Findings](https://aclanthology.org/2025.findings-naacl.271.pdf)
   argument that AUROC hides near-zero TPR at usable thresholds.

### D3. Adversarial fixtures to build

- Variation-selector payload chains carrying a known byte string, per the Butler
  encoding, including chains after non-emoji bases. Assert both strip and correct
  payload decode.
- Tag-block smuggled ASCII (the prompt-injection vector), distinguished from
  legitimate England, Scotland and Wales flag tag sequences.
- Trojan Source bidi samples in Rust, Python and Markdown; assert rejection in code
  contexts and preservation in RTL prose.
- Mixed-script homoglyph substitution at SilverSpeak's 5, 10 and 20 per cent rates.
- Nested and unbalanced bidi isolates.
- A PDF with metadata written by an incremental update, asserting the original
  `/Info` is not recoverable in the output byte stream.
- A DOCX with `rsid` fingerprints, tracked changes and `TotalTime`, asserting all are
  gone and the file still opens.
- A JPEG with multi-segment APP11 JUMBF split across segment boundaries.
- A PNG with both `eXIf` and `iTXt XML:com.adobe.xmp`, plus `caBX`.
- Legitimate-content controls: a document that is entirely emoji ZWJ sequences; a
  Devanagari sample; a Persian sample; a Hebrew-Latin mixed-direction sample.
- UK prose containing "World Health Organization", "a driving licence", "to license a
  doctor", "the gas meter read 12 metres", "the computer program", "sulfur dioxide",
  "the dialog box". Assert zero auto-fixes.

---

## E. Prioritised improvements over the current behaviour

Effort is rough engineering days for one experienced Rust engineer.

| # | Improvement | Why | Effort |
|---|---|---|---|
| 1 | **Rewrite the UK-English rule as a real subsystem**: VarCon data, span exclusion (code fences, inline code, URLs, front matter, quotations), an organisation-name gazetteer, sense disambiguation for program/meter/licence/practice/disc/draft/curb/story/tyre/cheque, `--oxford` flag, always-ise and always-yse overrides, and report-only status for every sense-dependent pair | The current single regex is actively wrong: it flags `licence` verbs, `gas meter`, `dialog box` and `World Health Organization`. This is the crate's biggest correctness defect | 8-12 |
| 2 | **Correct the provenance claims in SKILL.md and the crate description** | Section E5 currently claims the harness proves "the mark was cleared". [Zhang et al.](https://arxiv.org/abs/2311.04378) shows no lossless removal exists, and Claude itself has been watermarked since [2 August 2026](https://www.anthropic.com/news/claude-text-watermark). Publishing the current wording would be a false claim | 1-2 |
| 3 | **Replace hand-rolled PNG/JPEG/WebP parsing with `img-parts`** | 888 lines of hand-written container parsing where a maintained MIT/Apache crate exists; violates the no-hand-rolled-parsing rule and is a latent bug surface | 4-6 |
| 4 | **Decode variation-selector and tag-block payloads, do not just strip them** | [Butler's technique](https://paulbutler.org/2025/smuggling-arbitrary-data-through-an-emoji/) is the live steganography vector, used in a real npm supply-chain attack. Reporting *what was hidden* is far more valuable than silently deleting it, and turns the crate into a security tool | 3-4 |
| 5 | **Drop the `exiftool`/`c2patool`/`qpdf` subprocess dependency**: `little_exif` + `quick-xml` + `img-parts` + `lopdf` | Removes the argv-injection surface `common/proc.rs` currently guards against, removes silent capability gaps when tools are absent, and makes the crate self-contained and publishable | 6-8 |
| 6 | **Add SARIF 2.1.0 and JSON Lines output; standardise exit codes 0/1/2** | [GitHub code scanning requires SARIF 2.1.0](https://docs.github.com/en/code-security/reference/code-scanning/sarif-files/sarif-support). Small serialisation on top of the existing `Finding` struct, and it is what makes the crate adoptable in CI | 2-3 |
| 7 | **Build the UK human-prose evaluation corpus and publish the false-positive rate** | No published study measures detector or linter false positives on British English. This is a genuine gap, it is the crate's differentiator, and it is the evidence that the UK rules are safe | 5-7 |
| 8 | **Replace the hand-written confusables table with UTS #39 skeleton via `unicode-security`**, and split bidi policy by context (reject in code, preserve in prose) | 40 hand-written entries versus the full [confusables.txt](https://www.unicode.org/reports/tr39/); and the current single bidi policy is wrong for one of the two contexts. [UTS #55](https://www.unicode.org/reports/tr55/) is the standards answer | 3-4 |
| 9 | **Version and date-stamp the slop rule tables; add a confidence tier to each rule** | Lexical markers decay as models update ([Pangram versus Pew](https://www.pewresearch.org/data-labs/2026/08/20/how-much-of-the-internet-is-written-with-ai/) disagree on "delve"), so a frozen table silently rots. Split severity from confidence as [Semgrep](https://docs.semgrep.dev/kb/rules/understand-severities) does | 2-3 |
| 10 | **Split the workspace and settle the library API**: `check() -> Vec<Finding>` and `fix() -> Patch`, never in-place mutation; add Vale-style HTML-comment suppression; document the capability matrix in the crate-level rustdoc | Required for a crates.io publication with full inline rustdoc, and the `check`/`fix` split is what lets the same core serve the CLI, an LSP and the SARIF exporter | 4-6 |

Further work, lower priority: add a language pre-filter (`whatlang`) so UK rules never
fire on non-English spans (1-2 days); add AVIF/HEIF and MP4 `uuid`-box coverage now
that C2PA 2.4 specifies them (3-4 days); interoperate with `harper-core` rather than
growing a second grammar checker (3-5 days).

---

## F. What to stop claiming

Three statements in the current SKILL.md and reference material should not survive
into a published crate.

1. "**Statistical sampling fingerprints**" as something the crate strips. It cannot.
   No third party can. Rewrite as: paraphrase degrades sampling watermarks as a side
   effect of changing tokens; this is lossy and unverifiable.
2. "**Proving the mark was cleared in a closed loop**" (Section E5). The MarkLLM
   harness proves a *self-applied* mark was cleared with a *known key*. It says
   nothing about a vendor's production watermark.
3. "**Pixel-domain watermark removal**" (Section E4) as a crate capability. It is an
   external GPU dependency, it is now itself
   [detectable at 98 per cent TPR](https://arxiv.org/html/2605.09203v1), and stripping
   a manifest does not defeat a
   [durable Content Credential](https://developer.adobe.com/cai-soft-binding-api/)
   anyway.

Replacing these with the section B matrix costs nothing in real capability and buys
the crate a defensible position: **the best available deterministic, verifiable,
lossless sanitiser for the things that genuinely can be sanitised, honest about
everything else.**

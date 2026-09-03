//! Codepoint tables for Layer A: invisible controls and space homoglyphs.
//!
//! Seeded from the Python `text_unicode` tables this crate replaces, then
//! extended with the reserved-ignorable, noncharacter and blank-carrier
//! classes described below; the counts are asserted in the tests at the foot
//! of this module so a hand edit that drops an entry fails the build rather
//! than silently weakening detection.
//!
//! There is deliberately **no** confusables table here. The hand-written
//! 71-entry Latin lookalike list this module used to carry was replaced by
//! UTS #39 `confusables.txt` data; see [`crate::confusables`] for what the
//! standard covers and for the one documented override that closes its
//! fullwidth gap.

/// Format / invisible controls commonly used for steganography or broken pastes.
pub const STRIP_CODEPOINTS: &[u32] = &[
    0x00AD, // SOFT HYPHEN
    0x034F, // COMBINING GRAPHEME JOINER
    0x061C, // ARABIC LETTER MARK
    0x115F, // HANGUL CHOSEONG FILLER
    0x1160, // HANGUL JUNGSEONG FILLER
    0x17B4, // KHMER VOWEL INHERENT AQ
    0x17B5, // KHMER VOWEL INHERENT AA
    0x180B, // MONGOLIAN FREE VARIATION SELECTOR ONE
    0x180C, // MONGOLIAN FREE VARIATION SELECTOR TWO
    0x180D, // MONGOLIAN FREE VARIATION SELECTOR THREE
    0x180E, // MONGOLIAN VOWEL SEPARATOR
    0x180F, // MONGOLIAN FREE VARIATION SELECTOR FOUR
    0x200B, // ZERO WIDTH SPACE
    0x200C, // ZERO WIDTH NON-JOINER
    0x200D, // ZERO WIDTH JOINER
    0x200E, // LEFT-TO-RIGHT MARK
    0x200F, // RIGHT-TO-LEFT MARK
    0x202A, // LEFT-TO-RIGHT EMBEDDING
    0x202B, // RIGHT-TO-LEFT EMBEDDING
    0x202C, // POP DIRECTIONAL FORMATTING
    0x202D, // LEFT-TO-RIGHT OVERRIDE
    0x202E, // RIGHT-TO-LEFT OVERRIDE
    0x2060, // WORD JOINER
    0x2061, // FUNCTION APPLICATION
    0x2062, // INVISIBLE TIMES
    0x2063, // INVISIBLE SEPARATOR
    0x2064, // INVISIBLE PLUS
    0x2066, // LEFT-TO-RIGHT ISOLATE
    0x2067, // RIGHT-TO-LEFT ISOLATE
    0x2068, // FIRST STRONG ISOLATE
    0x2069, // POP DIRECTIONAL ISOLATE
    0x206A, // INHIBIT SYMMETRIC SWAPPING
    0x206B, // ACTIVATE SYMMETRIC SWAPPING
    0x206C, // INHIBIT ARABIC FORM SHAPING
    0x206D, // ACTIVATE ARABIC FORM SHAPING
    0x206E, // NATIONAL DIGIT SHAPES
    0x206F, // NOMINAL DIGIT SHAPES
    0x3164, // HANGUL FILLER
    0xFE00, // VARIATION SELECTOR-1
    0xFE01, // VARIATION SELECTOR-2
    0xFE02, // VARIATION SELECTOR-3
    0xFE03, // VARIATION SELECTOR-4
    0xFE04, // VARIATION SELECTOR-5
    0xFE05, // VARIATION SELECTOR-6
    0xFE06, // VARIATION SELECTOR-7
    0xFE07, // VARIATION SELECTOR-8
    0xFE08, // VARIATION SELECTOR-9
    0xFE09, // VARIATION SELECTOR-10
    0xFE0A, // VARIATION SELECTOR-11
    0xFE0B, // VARIATION SELECTOR-12
    0xFE0C, // VARIATION SELECTOR-13
    0xFE0D, // VARIATION SELECTOR-14
    0xFE0E, // VARIATION SELECTOR-15
    0xFE0F, // VARIATION SELECTOR-16
    0xFEFF, // ZERO WIDTH NO-BREAK SPACE
    0xFFA0, // HALFWIDTH HANGUL FILLER
    0xFFF9, // INTERLINEAR ANNOTATION ANCHOR
    0xFFFA, // INTERLINEAR ANNOTATION SEPARATOR
    0xFFFB, // INTERLINEAR ANNOTATION TERMINATOR
];

/// Spaces that look like (or substitute for) U+0020.
pub const SPACE_HOMOGLYPHS: &[(u32, char)] = &[
    (0x00A0, ' '), // NO-BREAK SPACE
    (0x1680, ' '), // OGHAM SPACE MARK
    (0x2000, ' '), // EN QUAD
    (0x2001, ' '), // EM QUAD
    (0x2002, ' '), // EN SPACE
    (0x2003, ' '), // EM SPACE
    (0x2004, ' '), // THREE-PER-EM SPACE
    (0x2005, ' '), // FOUR-PER-EM SPACE
    (0x2006, ' '), // SIX-PER-EM SPACE
    (0x2007, ' '), // FIGURE SPACE
    (0x2008, ' '), // PUNCTUATION SPACE
    (0x2009, ' '), // THIN SPACE
    (0x200A, ' '), // HAIR SPACE
    (0x202F, ' '), // NARROW NO-BREAK SPACE
    (0x205F, ' '), // MEDIUM MATHEMATICAL SPACE
    (0x3000, ' '), // IDEOGRAPHIC SPACE
];

/// Bidi controls, for the `bidi` inspect kind. Policy lives in [`crate::bidi`].
pub const BIDI_CPS: &[u32] = &[
    0x061C, 0x200E, 0x200F, 0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x2066, 0x2067, 0x2068, 0x2069,
];

/// The zero-width family, for the `zwj_family` inspect kind.
pub const ZW_FAMILY: &[u32] = &[0x180E, 0x200B, 0x200C, 0x200D, 0x2060, 0xFEFF];

/// Joiner and presentation selectors that bind an emoji sequence together.
pub const EMOJI_GLUE_CODEPOINTS: &[u32] = &[0x200D, 0xFE0E, 0xFE0F];

/// ZWNJ and ZWJ, which are orthographic inside complex scripts.
pub const SCRIPT_JOINERS: &[u32] = &[0x200C, 0x200D];

/// Arabic and Syriac `Cf` marks that are orthographic wherever they appear.
pub const ORTHOGRAPHIC_CF: &[u32] = &[
    0x0600, 0x0601, 0x0602, 0x0603, 0x0604, 0x0605, 0x06DD, 0x070F, 0x08E2, 0x110BD, 0x110CD,
];

/// Mongolian free variation selectors, load-bearing after a Mongolian letter.
///
/// `U+180F` (FVS4, added in Unicode 14) is category `Mn`, so the `Cf` catch-all
/// never saw it and it was absent from the strip set: it rendered blank between
/// plain ASCII and passed through untouched. It is listed here so it is kept
/// after a Mongolian letter exactly like FVS1-3, and stripped elsewhere.
pub const MONGOLIAN_FVS: &[u32] = &[0x180B, 0x180C, 0x180D, 0x180F];

/// Khmer inherent vowels, load-bearing after a Khmer letter.
pub const KHMER_VOWELS: &[u32] = &[0x17B4, 0x17B5];

/// Hangul jamo fillers, load-bearing after a jamo.
pub const HANGUL_FILLERS: &[u32] = &[0x115F, 0x1160];

/// Hangul fillers that render blank but are categorised `Lo`, not `Cf`.
///
/// `U+3164` HANGUL FILLER and `U+FFA0` HALFWIDTH HANGUL FILLER are
/// `Default_Ignorable` and render as nothing, but their `Lo` category kept them
/// out of the `Cf` catch-all: both `inspect` and `clean` passed them through
/// untouched even between plain ASCII. Each is load-bearing only after a jamo of
/// its *own* presentation form, which is a narrower context than the conjoining
/// fillers in [`HANGUL_FILLERS`] use, so the pairing is spelled out here:
/// `(filler, jamo range)`.
pub const HANGUL_BLANK_FILLERS: &[(u32, (u32, u32))] = &[
    (0x3164, (0x3131, 0x318E)), // compatibility jamo
    (0xFFA0, (0xFFA1, 0xFFDC)), // halfwidth jamo
];

/// Reserved `Default_Ignorable` code points with no legitimate interchange use.
///
/// These are unassigned but carry `Other_Default_Ignorable_Code_Point=Yes`, so a
/// conformant renderer draws nothing, normalisation preserves them, and
/// category-based (`Cf`) scrubbing never sees them because their category is
/// `Cn`. That combination makes them ideal covert carriers, and unlike a
/// noncharacter they are invisible rather than tofu.
pub const RESERVED_IGNORABLE_RANGES: &[(u32, u32)] = &[
    (0x2065, 0x2065),
    (0xFFF0, 0xFFF8),
    (0xE0000, 0xE0000),
    (0xE0080, 0xE00FF),
    (0xE01F0, 0xE0FFF),
];

/// An inclusive code point range `(first, last)`.
pub type CodepointRange = (u32, u32);

/// Format controls that visibly govern how their own script renders.
///
/// Each entry is `(control range, own-script range)`. These are category `Cf`,
/// so the catch-all stripped them, yet they govern quadrat stacking, shorthand
/// overlap and musical beaming: removing one changes the rendered text, which
/// breaks the "a cleaner preserves the document body" invariant. They are kept
/// when the preceding character belongs to their own script and stripped when
/// they float between unrelated text, exactly like the Mongolian, Khmer and
/// Hangul handling. Paranoid mode still strips them everywhere.
pub const LAYOUT_FORMAT_CONTROLS: &[(CodepointRange, CodepointRange)] = &[
    // Egyptian hieroglyph quadrat controls, over the base block and Extended-A.
    ((0x13430, 0x1343F), (0x13000, 0x1342F)),
    ((0x13430, 0x1343F), (0x13460, 0x143FF)),
    // Duployan shorthand overlap controls.
    ((0x1BCA0, 0x1BCA3), (0x1BC00, 0x1BC9F)),
    // Musical beam, tie, slur and phrase controls.
    ((0x1D173, 0x1D17A), (0x1D100, 0x1D1FF)),
];

/// Whether a codepoint falls in one of the `(start, end)` inclusive ranges.
fn in_ranges(ranges: &[(u32, u32)], codepoint: u32) -> bool {
    ranges
        .iter()
        .any(|(start, end)| (*start..=*end).contains(&codepoint))
}

/// Whether this is a reserved `Default_Ignorable` code point.
pub fn is_reserved_ignorable(codepoint: u32) -> bool {
    in_ranges(RESERVED_IGNORABLE_RANGES, codepoint)
}

/// Whether this is one of the 66 Unicode noncharacters.
///
/// `U+FDD0`-`U+FDEF` plus `U+FFFE`/`U+FFFF` at the end of every plane are
/// permanently reserved for internal use and prohibited in interchange text.
/// They render as nothing or tofu and survive normalisation, so they are a
/// ready-made covert channel. Unlike the other reserved ranges they can never be
/// assigned, so stripping them carries no future-Unicode risk.
pub fn is_noncharacter(codepoint: u32) -> bool {
    (0xFDD0..=0xFDEF).contains(&codepoint) || codepoint & 0xFFFE == 0xFFFE
}

/// Whether `codepoint` is a layout control kept next to `previous`.
///
/// Returns false when there is no preceding character: a control that opens the
/// text governs nothing and is treated as free-floating.
pub fn is_layout_control_in_context(codepoint: u32, previous: Option<u32>) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    LAYOUT_FORMAT_CONTROLS.iter().any(|((cs, ce), (ss, se))| {
        (*cs..=*ce).contains(&codepoint) && (*ss..=*se).contains(&previous)
    })
}

/// Whether `codepoint` is any layout control, in context or not.
pub fn is_layout_control(codepoint: u32) -> bool {
    LAYOUT_FORMAT_CONTROLS
        .iter()
        .any(|((cs, ce), _)| (*cs..=*ce).contains(&codepoint))
}

/// Whether `codepoint` is a blank Hangul filler kept after `previous`.
pub fn is_blank_filler_in_context(codepoint: u32, previous: Option<u32>) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    HANGUL_BLANK_FILLERS
        .iter()
        .any(|(filler, (start, end))| *filler == codepoint && (*start..=*end).contains(&previous))
}

/// Whether `codepoint` is one of the blank Hangul fillers.
pub fn is_blank_filler(codepoint: u32) -> bool {
    HANGUL_BLANK_FILLERS
        .iter()
        .any(|(filler, _)| *filler == codepoint)
}

/// Look up a replacement in one of the homoglyph tables.
pub fn lookup(table: &[(u32, char)], codepoint: u32) -> Option<char> {
    table
        .iter()
        .find(|(from, _)| *from == codepoint)
        .map(|(_, to)| *to)
}

/// Membership test for the flat codepoint sets.
pub fn contains(table: &[u32], codepoint: u32) -> bool {
    table.contains(&codepoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_match_the_ported_python_sizes() {
        assert_eq!(STRIP_CODEPOINTS.len(), 59);
        assert_eq!(SPACE_HOMOGLYPHS.len(), 16);
        assert_eq!(BIDI_CPS.len(), 12);
        assert_eq!(ZW_FAMILY.len(), 6);
        assert_eq!(EMOJI_GLUE_CODEPOINTS.len(), 3);
        assert_eq!(ORTHOGRAPHIC_CF.len(), 11);
        assert_eq!(MONGOLIAN_FVS.len(), 4);
    }

    #[test]
    fn lookup_maps_spaces_and_nothing_else() {
        assert_eq!(lookup(SPACE_HOMOGLYPHS, 0x00A0), Some(' '));
        assert_eq!(lookup(SPACE_HOMOGLYPHS, 0x0041), None);
    }

    #[test]
    fn every_space_homoglyph_maps_to_plain_space() {
        assert!(SPACE_HOMOGLYPHS.iter().all(|(_, to)| *to == ' '));
    }

    #[test]
    fn space_homoglyphs_are_never_also_strip_codepoints() {
        assert!(!SPACE_HOMOGLYPHS
            .iter()
            .any(|(cp, _)| contains(STRIP_CODEPOINTS, *cp)));
    }
}

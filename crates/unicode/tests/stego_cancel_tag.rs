//! A stray CANCEL TAG is mechanically contraband.
//!
//! The tag carrier's only legitimate use is terminating an RGI
//! subdivision-flag sequence, and there it follows tag ASCII inside the same
//! chain. A CANCEL TAG that `scan` reaches on its own — with no tag ASCII
//! before it in the chain — can never be part of a well-formed flag, so it
//! must be reported as a payload rather than silently consumed.

use prose_sanitiser_core::surrogate;
use prose_sanitiser_unicode::stego::{scan, PayloadKind, RGI_SUBDIVISION_TAGS};

/// The flag shape `U+1F3F4` + `tag` encoded as tag characters + CANCEL TAG.
fn flag(tag: &str) -> String {
    let mut text = String::from("\u{1F3F4}");
    for byte in tag.bytes() {
        text.push(char::from_u32(0xE0000 + u32::from(byte)).expect("assigned tag codepoint"));
    }
    text.push('\u{E007F}');
    text
}

#[test]
fn lone_cancel_tag_is_a_payload() {
    let text = String::from("x\u{E007F}y");
    let payloads = scan(&surrogate::decode(text.as_bytes()));
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].kind, PayloadKind::TagBlock);
    assert_eq!(payloads[0].start, 1);
    assert_eq!(payloads[0].end, 2);
    assert_eq!(payloads[0].base, Some('x'));
    assert!(payloads[0].bytes.is_empty());
    assert_eq!(payloads[0].as_text().as_deref(), Some(""));
}

#[test]
fn lone_cancel_after_flag_base_is_a_payload() {
    // A cancel with no tag ASCII before it is never a well-formed flag,
    // even hanging directly off the waving black flag.
    let text = String::from("\u{1F3F4}\u{E007F}");
    let payloads = scan(&surrogate::decode(text.as_bytes()));
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].kind, PayloadKind::TagBlock);
    assert_eq!(payloads[0].start, 1);
    assert_eq!(payloads[0].len(), 1);
}

#[test]
fn rgi_subdivision_flags_report_nothing() {
    for tag in RGI_SUBDIVISION_TAGS {
        let payloads = scan(&surrogate::decode(flag(tag).as_bytes()));
        assert!(payloads.is_empty(), "the {tag} subdivision flag must survive");
    }
}

#[test]
fn a_second_cancel_after_a_flag_is_a_payload() {
    // The flag's own chain consumes its CANCEL TAG; a second one is a stray.
    let mut text = flag("gbeng");
    text.push('\u{E007F}');
    let payloads = scan(&surrogate::decode(text.as_bytes()));
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].kind, PayloadKind::TagBlock);
    assert_eq!(payloads[0].start, 7);
    assert_eq!(payloads[0].len(), 1);
}

use engine::rename::{expand_name, NameContext};

fn ctx<'a>(stem: &'a str, seq: usize, w: u32, h: u32, date: &'a str) -> NameContext<'a> {
    NameContext {
        stem,
        seq,
        width: w,
        height: h,
        date,
    }
}

#[test]
fn name_and_padded_seq() {
    let c = ctx("photo", 5, 800, 600, "2026-06-25");
    assert_eq!(expand_name("{name}-{seq:000}", &c), "photo-005");
}

#[test]
fn dimensions_and_unpadded_seq() {
    let c = ctx("shot", 12, 1920, 1080, "2026-06-25");
    assert_eq!(expand_name("{name}_{w}x{h}_{seq}", &c), "shot_1920x1080_12");
}

#[test]
fn date_prefix_and_literal_text() {
    let c = ctx("img", 1, 100, 100, "2026-06-25");
    assert_eq!(
        expand_name("{date}-house-{name}", &c),
        "2026-06-25-house-img"
    );
}

#[test]
fn unknown_tokens_are_literal() {
    let c = ctx("img", 1, 100, 100, "2026-06-25");
    assert_eq!(expand_name("{bogus}-{name}", &c), "{bogus}-img");
}

#[test]
fn seq_overflow_pad_prints_full_number() {
    let c = ctx("img", 1234, 1, 1, "d");
    assert_eq!(expand_name("{seq:00}", &c), "1234");
}

#[test]
fn illegal_characters_are_sanitized() {
    let c = ctx("a/b:c", 1, 1, 1, "d");
    // stem contains '/' and ':' -> both become '_'
    assert_eq!(expand_name("{name}", &c), "a_b_c");
}

#[test]
fn empty_pattern_falls_back_to_stem() {
    let c = ctx("photo", 1, 1, 1, "d");
    assert_eq!(expand_name("", &c), "photo");
}

#[test]
fn trailing_and_leading_dots_trimmed() {
    let c = ctx("  .photo.", 1, 1, 1, "d");
    assert_eq!(expand_name("{name}", &c), "photo");
}

#[test]
fn empty_stem_fallback() {
    let c = ctx("   ", 1, 1, 1, "d");
    assert_eq!(expand_name("", &c), "image");
}

#[test]
fn complex_pattern_with_padding() {
    let c = ctx("test", 7, 1024, 768, "2026-06-25");
    assert_eq!(
        expand_name("{name}_{seq:0000}_{w}x{h}_{date}", &c),
        "test_0007_1024x768_2026-06-25"
    );
}

#[test]
fn unmatched_opening_brace() {
    let c = ctx("test", 1, 1, 1, "d");
    assert_eq!(expand_name("prefix-{name", &c), "prefix-{name");
}

#[test]
fn multiple_unknown_tokens() {
    let c = ctx("file", 1, 1, 1, "d");
    assert_eq!(
        expand_name("{unknown1}-{name}-{unknown2}", &c),
        "{unknown1}-file-{unknown2}"
    );
}

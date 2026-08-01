use tmplx::tmplx_runtime::*;

// Entity table of §6, tested character by character.
#[test]
fn individual_entity_table() {
    let cases = [
        ("&", "&amp;"),
        ("<", "&lt;"),
        (">", "&gt;"),
        ("\"", "&quot;"),
        ("'", "&#39;"),
    ];
    for (input, expected) in cases {
        let mut out = String::new();
        write_escaped(&mut out, input);
        assert_eq!(out, expected);
    }
}

// Exact examples given in §6.
#[test]
fn spec_reference_examples() {
    let mut out = String::new();
    write_escaped(&mut out, "Paul \"the Great\"");
    assert_eq!(out, "Paul &quot;the Great&quot;");

    let mut out = String::new();
    write_escaped(&mut out, "O'Brien <script>");
    assert_eq!(out, "O&#39;Brien &lt;script&gt;");
}

// Regression test on the double-escaping risk described in §6,
// reason 2: an '&' produced by escaping a neighboring character
// must never be re-escaped. Here the input literally contains the
// characters '&', 'a', 'm', 'p', ';' — the only special character is
// the initial '&'.
#[test]
fn no_double_escaping() {
    let mut out = String::new();
    write_escaped(&mut out, "&amp;");
    assert_eq!(out, "&amp;amp;");
}

// Text without special character: copied without additional allocation
// of content (no substitution should occur).
#[test]
fn text_without_special_character_unchanged() {
    let mut out = String::new();
    write_escaped(&mut out, "normal text 123");
    assert_eq!(out, "normal text 123");
}

// output is an accumulator buffer, not recreated: two successive calls
// must concatenate, not overwrite (expected behavior from plan §5 —
// write_escaped writes into the single output buffer).
#[test]
fn accumulation_in_the_same_buffer() {
    let mut out = String::new();
    write_escaped(&mut out, "a<b");
    write_escaped(&mut out, "&c");
    assert_eq!(out, "a&lt;b&amp;c");
}

#[test]
fn test_write_escaped_js_comprehensive() {
    let input = "hello `world` \n and \"friends\" & <script> \\ 'test'";
    let mut out = String::new();
    write_escaped_js(&mut out, input);

    let expected =
        "hello \\`world\\` \\n and \\\"friends\\\" \\x26 \\x3Cscript\\x3E \\\\ \\'test\\'";
    assert_eq!(out, expected);

    // Ensure standard text passes through cleanly
    let mut out_clean = String::new();
    write_escaped_js(&mut out_clean, "safe text");
    assert_eq!(out_clean, "safe text");
}

#[test]
fn test_write_escaped_url_comprehensive() {
    let input = "hello world! ?&=";
    let mut out = String::new();
    write_escaped_url(&mut out, input);

    // spaces become %20, ! becomes %21, ? becomes %3F, & becomes %26, = becomes %3D
    let expected = "hello%20world%21%20%3F%26%3D";
    assert_eq!(out, expected);

    // Alphanumerics and . - _ ~ shouldn't be escaped
    let mut out_clean = String::new();
    let unreserved = "aA0-._~";
    write_escaped_url(&mut out_clean, unreserved);
    assert_eq!(out_clean, unreserved);
}

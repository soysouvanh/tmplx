use tmplx::build_logic::tokenizer::tokenize;
use tmplx::build_logic::validator::*;

#[test]
fn if_endif_simple_valid() {
    validate_pairing(&tokenize("{% if a %}x{% endif %}"));
}

#[test]
fn for_endfor_simple_valid() {
    validate_pairing(&tokenize("{% for x in list %}y{% endfor %}"));
    validate_pairing(&tokenize("{% for x in list { %}y{% } %}"));
}

#[test]
fn if_else_endif_valid() {
    validate_pairing(&tokenize("{% if a %}x{% else %}y{% endif %}"));
}

#[test]
fn if_else_if_else_endif_valid() {
    validate_pairing(&tokenize(
        "{% if a %}x{% else if b %}y{% else %}z{% endif %}",
    ));
}

#[test]
fn if_with_negation_valid_like_normal_if() {
    // Negation does not change the block structure (§5.3):
    // only pass 4 (step 5) cares for generation.
    validate_pairing(&tokenize("{% if !a %}x{% endif %}"));
}

#[test]
fn nesting_for_in_if_and_if_in_for_without_depth_limit() {
    let src = "{% if a %}{% for x in l %}{% if b %}{% for y in m %}z{% endfor %}{% endif %}{% endfor %}{% endif %}";
    validate_pairing(&tokenize(src));
}

#[test]
fn comment_containing_block_keywords_is_ignored() {
    // The content of a comment ({# #}) is NEVER examined for
    // block structure (rule 10, §3) — even if it contains "endif".
    validate_pairing(&tokenize(
        "{% if a %}{# endif endfor whatever #}{% endif %}",
    ));
}

#[test]
fn let_and_outputs_do_not_interfere_with_block_counting() {
    validate_pairing(&tokenize(
        "{% if a %}{% let x = 1; %}{%= x %}{%%= x %}{% endif %}",
    ));
}

/// EXACTLY reproduces the example of §7.1, message 4: if opened at line
/// 9, endfor encountered at line 17. Verifies a strict equality with the
/// verbatim text of the spec, not just a `contains`.
#[test]
fn message_endfor_vs_if_is_verbatim_compliant_with_spec() {
    let mut src = String::new();
    for _ in 0..8 {
        src.push_str("filler_line\n");
    } // lines 1-8
    src.push_str("{% if x %}\n"); // line 9
    for _ in 0..7 {
        src.push_str("filler_line\n");
    } // lines 10-16
    src.push_str("{% endfor %}\n"); // line 17

    let result = std::panic::catch_unwind(|| validate_pairing(&tokenize(&src)));
    let err = result.unwrap_err();
    let msg = err
        .downcast_ref::<String>()
        .map(|s| s.as_str())
        .or_else(|| err.downcast_ref::<&str>().copied())
        .expect("panic message expected");

    assert_eq!(
        msg,
        "tmplx build error line 17: '{% endfor %}' encountered while the block opened at line 9 is a 'if' (expected '{% endif %}')."
    );
}

#[test]
fn endif_facing_for_reports_wrong_type() {
    let result =
        std::panic::catch_unwind(|| validate_pairing(&tokenize("{% for x in l %}y{% endif %}")));
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<String>().map(|s| s.as_str()).unwrap();
    assert!(msg.contains("'{% endif %}' encountered"));
    assert!(msg.contains("is a 'for'"));
    assert!(msg.contains("expected '{% endfor %}'"));
}

#[test]
fn else_facing_for_reports_wrong_type() {
    let result = std::panic::catch_unwind(|| {
        validate_pairing(&tokenize("{% for x in l %}{% else %}{% endfor %}"))
    });
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<String>().map(|s| s.as_str()).unwrap();
    assert!(msg.contains("'{% else %}' encountered"));
    assert!(msg.contains("is a 'for'"));
}

#[test]
fn endif_without_any_open_block() {
    let result = std::panic::catch_unwind(|| validate_pairing(&tokenize("{% endif %}")));
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<String>().map(|s| s.as_str()).unwrap();
    assert!(msg.contains("while no block is open"));
}

#[test]
fn endfor_without_any_open_block() {
    let result = std::panic::catch_unwind(|| validate_pairing(&tokenize("{% endfor %}")));
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<String>().map(|s| s.as_str()).unwrap();
    assert!(msg.contains("while no block is open"));
}

#[test]
fn else_without_any_open_block() {
    let result = std::panic::catch_unwind(|| validate_pairing(&tokenize("{% else %}")));
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<String>().map(|s| s.as_str()).unwrap();
    assert!(msg.contains("while no block is open"));
}

/// A single open block at EOF: reproduces §7.1 message 1
/// verbatim (strict equality).
#[test]
fn message_block_never_closed_is_verbatim_compliant() {
    let mut src = String::new();
    for _ in 0..8 {
        src.push_str("filler_line\n");
    } // lines 1-8
    src.push_str("{% if x %}\n"); // line 9 — never closed

    let result = std::panic::catch_unwind(|| validate_pairing(&tokenize(&src)));
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<String>().map(|s| s.as_str()).unwrap();

    assert_eq!(
        msg,
        "tmplx build error: block tag '{% if %}' opened at line 9 never closed."
    );
}

/// Multiple blocks remain open: documents the choice (not dictated by
/// spec) to report the most recently opened, not the first.
#[test]
fn multiple_blocks_unclosed_reports_innermost() {
    // if opened line 1, for opened line 1 too (same line, consecutive
    // tags) : for is pushed second, so reported.
    let src = "{% if a %}{% for x in l %}";
    let result = std::panic::catch_unwind(|| validate_pairing(&tokenize(src)));
    let err = result.unwrap_err();
    let msg = err.downcast_ref::<String>().map(|s| s.as_str()).unwrap();
    assert!(msg.contains("'{% for %}'"), "message obtained : {msg}");
}

/// Integration check on the real reference mockup
/// (§10.2): if/else/endif + for containing a nested if/endif,
/// must validate without panicking.
#[test]
fn reference_mockup_validates_without_panic() {
    let src = include_str!("../../tmplx-test/templates/mockup.html");
    validate_pairing(&tokenize(src));
}

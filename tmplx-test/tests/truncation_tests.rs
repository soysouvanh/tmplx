use tmplx::build_logic::tokenizer::*;
use tmplx::build_logic::truncation::*;

/// Rule 9 (§3): exactly these four characters, no other Unicode
/// "whitespace" character (so NO non-breaking space U+00A0, for example).
fn is_whitespace_rule9(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

fn static_text<'a>(tokens: &'a [Token<'a>], index: usize) -> &'a str {
    match &tokens[index] {
        Token::Static { text } => text,
        other => panic!("expected Static at index {index}, got {other:?}"),
    }
}

#[test]
fn trim_left_truncates_preceding_static() {
    let src = "abc   {%- if x %}";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    assert_eq!(static_text(&toks, 0), "abc");
}

#[test]
fn trim_right_truncates_following_static() {
    let src = "{% if x -%}   def";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    assert_eq!(static_text(&toks, 2), "def");
}

#[test]
fn without_truncation_marker_nothing_changes() {
    let src = "abc   {% if x %}   def";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    assert_eq!(static_text(&toks, 0), "abc   ");
    assert_eq!(static_text(&toks, 2), "   def");
}

#[test]
fn both_sides_are_independent_only_marked_side_is_touched() {
    // Tag 1 touches left, tag 2 touches right.
    let src = "abc   {%- if x %}   def   {% if y -%}   ghi";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    // "abc   " -> "abc" (before tag 1)
    assert_eq!(static_text(&toks, 0), "abc");
    // "   def   " stays identical (between tag 1 and 2, no marker points to it)
    assert_eq!(static_text(&toks, 2), "   def   ");
    // "   ghi" -> "ghi" (after tag 2)
    assert_eq!(static_text(&toks, 4), "ghi");
}

#[test]
fn fully_blank_static_between_two_truncations_becomes_empty() {
    // trim_right of first tag + trim_left of second, both
    // targeting the SAME middle static, each on its end.
    let src = "{% a -%}   \t\n  {%- b %}";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    assert_eq!(static_text(&toks, 2), "");
}

#[test]
fn unlimited_quantity_lots_of_whitespace_fully_removed() {
    let src = "text\n\n\t\t   \r\n   {%- if a %}";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    assert_eq!(static_text(&toks, 0), "text");
}

#[test]
fn non_breaking_space_is_not_truncated() {
    let src = "text\u{00A0}\u{00A0}  {%- if a %}";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    // The two regular spaces drop, the two non-breaking spaces remain.
    assert_eq!(static_text(&toks, 0), "text\u{00A0}\u{00A0}");
}

#[test]
fn first_tag_in_file_trim_left_empties_initial_static() {
    let src = "   {%- if a %}";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    assert_eq!(static_text(&toks, 0), "");
}

#[test]
fn last_tag_in_file_trim_right_empties_final_static() {
    let src = "{% if a -%}   ";
    let mut toks = tokenize(src);
    apply_truncation(&mut toks);
    assert_eq!(static_text(&toks, 2), "");
}

#[test]
fn tagged_tokens_are_never_modified() {
    let src = "a  {%- if x -%}  b  {%- endif -%}  c";
    let mut toks = tokenize(src);
    let tagged_before: Vec<Token> = toks
        .iter()
        .filter(|t| matches!(t, Token::Tagged { .. }))
        .cloned()
        .collect();
    apply_truncation(&mut toks);
    let tagged_after: Vec<Token> = toks
        .iter()
        .filter(|t| matches!(t, Token::Tagged { .. }))
        .cloned()
        .collect();
    assert_eq!(tagged_before, tagged_after);
}

#[test]
fn reference_mockup_has_no_whitespace_on_marked_side_after_truncation() {
    let src = include_str!("../../tmplx-test/templates/mockup.html");
    let mut toks = tokenize(src);

    let nb_trim_left = toks
        .iter()
        .filter(|t| {
            matches!(
                t,
                Token::Tagged {
                    trim_left: true,
                    ..
                }
            )
        })
        .count();
    assert_eq!(nb_trim_left, 5, "reference mockup changed?");

    apply_truncation(&mut toks);

    for i in 0..toks.len() {
        let (trim_left, trim_right) = match &toks[i] {
            Token::Tagged {
                trim_left,
                trim_right,
                ..
            } => (*trim_left, *trim_right),
            Token::Static { .. } => continue,
        };
        if trim_left && i > 0 {
            let t = static_text(&toks, i - 1);
            assert!(
                !t.ends_with(is_whitespace_rule9),
                "Static before index {i} still ends with whitespace: {t:?}"
            );
        }
        if trim_right && i + 1 < toks.len() {
            let t = static_text(&toks, i + 1);
            assert!(
                !t.starts_with(is_whitespace_rule9),
                "Static after index {i} still starts with whitespace: {t:?}"
            );
        }
    }
}

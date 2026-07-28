// build_logic/truncation.rs — Pass 3 of the pipeline (§5.3).
//
// Same shared inclusion pattern as tokenizer.rs / validator.rs: single source of
// truth, included by build.rs (actual usage) and by src/main.rs under
// #[cfg(test)].

use crate::tokenizer::Token;

/// Rule 9 (§3): exactly these four characters, no other Unicode
/// "whitespace" character (so NO non-breaking space U+00A0, for example).
fn is_whitespace_rule9(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// Pass 3 (§5.3): for each `Tagged` token carrying a truncation
/// flag, truncates the corresponding end of the `Static` token
/// immediately adjacent — the one before for `trim_left` (`{%-`),
/// the one after for `trim_right` (`-%}`). Without quantity limit
/// (rule 9). Mutates `tokens` in place; `Tagged` tokens are never
/// modified by this pass.
///
/// No possible conflict between two tags on the same `Static`: by
/// construction (pass 1), each `Static` never has more than one
/// `Tagged` neighbor on each side, so each end is only ever targeted
/// by a single truncation (see `double_truncation_...` tests).
pub fn apply_truncation<'a>(tokens: &mut [Token<'a>]) {
    let n = tokens.len();
    for i in 0..n {
        let (trim_left, trim_right) = match &tokens[i] {
            Token::Tagged {
                trim_left,
                trim_right,
                ..
            } => (*trim_left, *trim_right),
            Token::Static { .. } => continue,
        };

        if trim_left
            && i > 0
            && let Token::Static { text } = &mut tokens[i - 1]
        {
            *text = text.trim_end_matches(is_whitespace_rule9);
        }
        if trim_right
            && i + 1 < n
            && let Token::Static { text } = &mut tokens[i + 1]
        {
            *text = text.trim_start_matches(is_whitespace_rule9);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    fn static_text<'a>(tokens: &'a [Token<'a>], index: usize) -> &'a str {
        match &tokens[index] {
            Token::Static { text } => text,
            other => panic!("expected Static at index {index}, got {other:?}"),
        }
    }

    #[test]
    fn trim_left_truncates_preceding_static() {
        let src = "text   \n  {%- if a %}after";
        let mut toks = tokenize(src);
        apply_truncation(&mut toks);
        // "text   \n  " -> all trailing whitespace (space, \n, space) drops.
        assert_eq!(static_text(&toks, 0), "text");
    }

    #[test]
    fn trim_right_truncates_following_static() {
        let src = "before{% if a -%}   \n  text";
        let mut toks = tokenize(src);
        apply_truncation(&mut toks);
        assert_eq!(static_text(&toks, 2), "text");
    }

    #[test]
    fn without_truncation_marker_nothing_changes() {
        let src = "before  {% if a %}  after";
        let mut toks = tokenize(src);
        let before_static = static_text(&toks, 0).to_string();
        let after_static = static_text(&toks, 2).to_string();
        apply_truncation(&mut toks);
        assert_eq!(static_text(&toks, 0), before_static);
        assert_eq!(static_text(&toks, 2), after_static);
    }

    #[test]
    fn both_sides_are_independent_only_marked_side_is_touched() {
        // trim_left only: right side (after the tag) must not
        // be touched.
        let src = "before   {%- if a %}   after";
        let mut toks = tokenize(src);
        apply_truncation(&mut toks);
        assert_eq!(static_text(&toks, 0), "before");
        assert_eq!(static_text(&toks, 2), "   after"); // unchanged
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

    /// Explicit Rule 9: "no other Unicode character" — a
    /// non-breaking space (U+00A0) is NOT one of the four characters and must
    /// survive the truncation.
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

    /// Integration check on the real reference mockup
    /// (§10.2): general structural property instead of handcrafted
    /// expected text (so no risk of typos on my end
    /// regarding expected content) — after truncation, no
    /// Static adjacent to a trim_left/trim_right tag keeps
    /// whitespace on the marked side.
    #[test]
    fn reference_mockup_has_no_whitespace_on_marked_side_after_truncation() {
        let src = include_str!("../templates/mockup.html");
        let mut toks = tokenize(src);

        // The real mockup uses `{%-` five times (if/else/endif/for/
        // endfor) and no `-%}`: at least one real case of each side is
        // exercised by the global test suite (trim_right tested elsewhere
        // on constructed cases). We verify it rather than assuming it:
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
}

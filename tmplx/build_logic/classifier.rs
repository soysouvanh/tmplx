// build_logic/classifier.rs — Full classification of tags +
// resolution of the first identifier segment (§3.1).
//
// Replaces the minimal classifier introduced in validator.rs at step 3
// (announced back then as intentionally partial). Same shared inclusion
// pattern as tokenizer.rs / validator.rs / truncation.rs.

use crate::tokenizer::DelimiterType;

/// §3.1, para 2: only the first segment before a dot is resolved
/// against the scope / view_data; the rest (`.name`) is copied as-is
/// into the generated Rust, without validation by build.rs.
///
/// Not yet called by wired code (only by its own tests): the
/// scope resolution that will use it is the next sub-step.
/// `#[allow(dead_code)]` assumed, not an oversight.
#[allow(dead_code)]
pub fn first_segment(ident: &str) -> &str {
    match ident.find('.') {
        Some(i) => &ident[..i],
        None => ident,
    }
}

/// Exhaustive classification of the (already trimmed) content of a `Tagged` token,
/// covering the ten rules of §3. A comment (`{# #}`) is always
/// `Comment` regardless of its content (rule 10) — never examined.
///
/// SECURITY (§14 of the tmplx specification — intentional choice,
/// not an oversight): `RawOutput` (`{%= %}`, NO HTML escaping) is the
/// shortest syntax in the grammar; `EscapedOutput` (`{%%= %}`)
/// must be explicitly requested. This is the opposite of the convention
/// of typical Rust template engines (Tera, Askama escape by default).
/// Anyone binding a parameter of `render_extreme` to untrusted user
/// input MUST use `{%%= %}` in the template, otherwise it's a direct HTML/XSS injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassifiedTag<'a> {
    /// Rule 1: `{%= ident %}` — RAW output, no escaping. See the
    /// SECURITY note above before using this variant on untrusted data.
    RawOutput { ident: &'a str },
    /// Rule 2: `{%%= ident %}` — HTML escaping (`write_escaped`,
    /// §6). To be used for all untrusted data (see SECURITY above).
    EscapedOutput { ident: &'a str },
    /// Rule 2.1: `{%js= ident %}` — JavaScript escaping
    EscapedOutputJs { ident: &'a str },
    /// Rule 2.2: `{%url= ident %}` — URL escaping
    EscapedOutputUrl { ident: &'a str },
    /// Rule 3: `{% if expr %}`
    If { expr: &'a str },
    /// Rule 4.5
    ElseIf { expr: &'a str },
    /// Rule 11
    Extends { path: &'a str },
    /// Rule 11 (include)
    Include { path: &'a str },
    /// Rule 12
    Block { name: &'a str },
    /// Rule 4
    Else,
    /// Rule 5
    EndIf,
    /// Generic rule: `{% } %}`
    EndBlock,
    /// Rule 6: `{% for item in iterable %}`
    For { item: &'a str, iterable: &'a str },
    /// Rule 7
    EndFor,
    /// Rule 5: `{% } %}` — Generic closer for if or for
    GenericClose,
    /// Rule 8: `{% let ident = expr_rust; %}`. `body` = the complete trimmed
    /// content ("let ident = expr_rust;"), for verbatim passthrough
    /// as-is in code generation — see §5.3 pass 4, "let → verbatim passthrough".
    Let { ident: &'a str, body: &'a str },
    /// Rule 10
    Comment,
}

/// Classifies a `Tagged` token. Panics on unrecognized syntax
/// inside an instruction tag (`for` without "in", `let` without
/// `=`, or content not matching any of the ten rules): none of
/// these messages are given verbatim by the spec (§7.1 only covers
/// scope resolution, not internal tag syntax) — implementation
/// decisions, in the style of existing messages.
pub fn classify<'a>(content: &'a str, delimiter: DelimiterType, line: usize) -> ClassifiedTag<'a> {
    if delimiter == DelimiterType::Comment {
        return ClassifiedTag::Comment;
    }

    let t = content.trim();

    if t == "else" || t == "} else {" || t == "else {" {
        return ClassifiedTag::Else;
    }
    if t.starts_with("else if") || t.starts_with("} else if") {
        let mut rest = t.trim_start_matches('}').trim_start();
        if rest.starts_with("else if") {
            rest = rest["else if".len()..].trim_start();
        }
        let expr = rest.strip_suffix('{').unwrap_or(rest).trim();
        return ClassifiedTag::ElseIf { expr };
    }
    if t.starts_with("extends ") {
        let path = t.strip_prefix("extends").unwrap().trim();
        let path = path.trim_matches('"').trim_matches('\'');
        return ClassifiedTag::Extends { path };
    }
    if t.starts_with("include ") {
        let path = t.strip_prefix("include").unwrap().trim();
        let path = path.trim_matches('"').trim_matches('\'');
        return ClassifiedTag::Include { path };
    }
    if t.starts_with("block ") {
        let name = t.strip_prefix("block").unwrap().trim();
        let name = name.strip_suffix('{').unwrap_or(name).trim();
        return ClassifiedTag::Block { name };
    }
    if t == "endblock" {
        return ClassifiedTag::EndBlock;
    }
    if t == "}" {
        return ClassifiedTag::GenericClose;
    }
    if t == "endif" {
        return ClassifiedTag::EndIf;
    }
    if t == "endfor" {
        return ClassifiedTag::EndFor;
    }
    if let Some(rest) = t.strip_prefix("%=") {
        return ClassifiedTag::EscapedOutput { ident: rest.trim() };
    }
    if let Some(rest) = t.strip_prefix("js=") {
        return ClassifiedTag::EscapedOutputJs { ident: rest.trim() };
    }
    if let Some(rest) = t.strip_prefix("url=") {
        return ClassifiedTag::EscapedOutputUrl { ident: rest.trim() };
    }
    if let Some(rest) = t.strip_prefix('=') {
        return ClassifiedTag::RawOutput { ident: rest.trim() };
    }
    if t == "if" || t.starts_with("if ") {
        let rest = t.strip_prefix("if").unwrap().trim();
        let expr = rest.strip_suffix('{').unwrap_or(rest).trim();
        return ClassifiedTag::If { expr };
    }
    if t == "for" || t.starts_with("for ") {
        let rest = t.strip_prefix("for").unwrap().trim();
        let rest = rest.strip_suffix('{').unwrap_or(rest).trim();
        let mut in_pos = None;
        let bytes = rest.as_bytes();
        for i in 1..bytes.len().saturating_sub(2) {
            if bytes[i..i + 2] == b"in"[..]
                && bytes[i - 1].is_ascii_whitespace()
                && bytes[i + 2].is_ascii_whitespace()
            {
                in_pos = Some(i);
                break;
            }
        }

        if let Some(pos) = in_pos {
            let item = rest[..pos].trim();
            let iterable = rest[pos + 2..].trim();
            if !item.is_empty() && !iterable.is_empty() {
                return ClassifiedTag::For { item, iterable };
            }
        }

        panic!(
            "tmplx build error line {line}: '{{% for %}}' malformed, 'in' expected (syntax: '{{% for item in list %}}')."
        );
    }
    if t == "let" || t.starts_with("let ") {
        let rest = t.strip_prefix("let").unwrap().trim();
        return match rest.find('=') {
            Some(i) => ClassifiedTag::Let {
                ident: rest[..i].trim(),
                body: t,
            },
            None => panic!(
                "tmplx build error line {line}: '{{% let %}}' malformed, '=' sign expected (syntax: '{{% let ident = expr_rust; %}}')."
            ),
        };
    }

    panic!("tmplx build error line {line}: tag '{{% {t} %}}' unrecognized.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const I: DelimiterType = DelimiterType::Instruction;
    const C: DelimiterType = DelimiterType::Comment;

    #[test]
    fn first_segment_without_dot() {
        assert_eq!(first_segment("user"), "user");
    }

    #[test]
    fn first_segment_with_dot() {
        assert_eq!(first_segment("item.name"), "item");
    }

    #[test]
    fn first_segment_with_multiple_dots_only_splits_at_first() {
        // Not tested elsewhere by the spec (depth >1 out of scope,
        // §1) but the function must remain well-defined on this case.
        assert_eq!(first_segment("a.b.c"), "a");
    }

    #[test]
    fn raw_output() {
        assert_eq!(
            classify("= user ", I, 1),
            ClassifiedTag::RawOutput { ident: "user" }
        );
    }

    #[test]
    fn raw_output_pointed_identifier() {
        assert_eq!(
            classify("= item.name ", I, 1),
            ClassifiedTag::RawOutput { ident: "item.name" }
        );
    }

    #[test]
    fn escaped_output() {
        assert_eq!(
            classify("%= item.name ", I, 1),
            ClassifiedTag::EscapedOutput { ident: "item.name" }
        );
    }

    #[test]
    fn escaped_output_js_and_url() {
        assert_eq!(
            classify("js= data ", I, 1),
            ClassifiedTag::EscapedOutputJs { ident: "data" }
        );
        assert_eq!(
            classify("url= link ", I, 1),
            ClassifiedTag::EscapedOutputUrl { ident: "link" }
        );
    }

    #[test]
    fn if_simple() {
        assert_eq!(
            classify(" if is_admin ", I, 1),
            ClassifiedTag::If { expr: "is_admin" }
        );
    }

    #[test]
    fn if_negation() {
        assert_eq!(
            classify(" if !is_admin ", I, 1),
            ClassifiedTag::If { expr: "!is_admin" }
        );
    }

    #[test]
    fn if_complex_expression() {
        assert_eq!(
            classify(" if a == b && c > 2 ", I, 1),
            ClassifiedTag::If {
                expr: "a == b && c > 2"
            }
        );
    }

    #[test]
    fn else_endif_endfor() {
        assert_eq!(classify(" else ", I, 1), ClassifiedTag::Else);
        assert_eq!(classify(" } else { ", I, 1), ClassifiedTag::Else);
        assert_eq!(classify(" endif ", I, 1), ClassifiedTag::EndIf);
        assert_eq!(classify(" endfor ", I, 1), ClassifiedTag::EndFor);
        assert_eq!(classify(" } ", I, 1), ClassifiedTag::GenericClose);
    }

    #[test]
    fn else_if_test() {
        assert_eq!(
            classify(" else if a == b ", I, 1),
            ClassifiedTag::ElseIf { expr: "a == b" }
        );
        assert_eq!(
            classify(" } else if a == b { ", I, 1),
            ClassifiedTag::ElseIf { expr: "a == b" }
        );
    }

    #[test]
    fn for_simple() {
        assert_eq!(
            classify(" for item in user_list ", I, 1),
            ClassifiedTag::For {
                item: "item",
                iterable: "user_list"
            }
        );
        assert_eq!(
            classify(" for item in user_list { ", I, 1),
            ClassifiedTag::For {
                item: "item",
                iterable: "user_list"
            }
        );
    }

    #[test]
    fn for_with_multiple_spaces_remains_robust() {
        assert_eq!(
            classify(" for   item   in   list  ", I, 1),
            ClassifiedTag::For {
                item: "item",
                iterable: "list"
            }
        );
    }

    #[test]
    #[should_panic(expected = "'in' expected")]
    fn for_without_in_panics() {
        classify(" for item list ", I, 1);
    }

    #[test]
    fn let_extracts_ident_and_keeps_full_body() {
        assert_eq!(
            classify(" let is_adult = item.age >= 18; ", I, 1),
            ClassifiedTag::Let {
                ident: "is_adult",
                body: "let is_adult = item.age >= 18;"
            }
        );
    }

    #[test]
    #[should_panic(expected = "'=' sign expected")]
    fn let_without_equal_panics() {
        classify(" let is_adult ", I, 1);
    }

    #[test]
    fn comment_is_always_comment_even_with_content_looking_like_something_else() {
        assert_eq!(classify(" if endfor let %= ", C, 1), ClassifiedTag::Comment);
    }

    #[test]
    #[should_panic(expected = "unrecognized")]
    fn unrecognized_content_panics() {
        classify(" this is not a valid tag ", I, 1);
    }

    #[test]
    fn identifier_starting_like_keyword_is_not_confused() {
        // "fortune" must not be mistaken for a malformed "for": it
        // falls under "unrecognized", not in the for branch.
        let result = std::panic::catch_unwind(|| classify(" fortune ", I, 1));
        assert!(result.is_err());
    }
}

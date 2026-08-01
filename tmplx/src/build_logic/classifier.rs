// build_logic/classifier.rs — Full classification of tags +
// resolution of the first identifier segment (§3.1).
//
// Replaces the minimal classifier introduced in validator.rs at step 3
// (announced back then as intentionally partial). Same shared inclusion
// pattern as tokenizer.rs / validator.rs / truncation.rs.

use super::tokenizer::DelimiterType;

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

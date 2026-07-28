// build_logic/tokenizer.rs — Pass 1 of the pipeline (§5.3).
//
// Included as-is from two places:
//   - build.rs (actual usage, during crate compilation)
//   - src/main.rs, under #[cfg(test)] (to ensure `cargo test` executes the
//     tests below — build.rs is NOT a Cargo test target,
//     verified empirically before writing this file)
// Single source of truth: no copy, no risk of divergence
// between the tested version and the version actually executed at build time.

/// The pair of delimiters that opened a `Tagged` token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterType {
    /// `{% ... %}`
    Instruction,
    /// `{# ... #}`
    Comment,
}

/// A token produced by pass 1 (§5.3). Borrows `source`: zero
/// allocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    Static {
        text: &'a str,
    },
    Tagged {
        /// Raw content between delimiters, truncation `-` markers
        /// excluded. Neither trimmed (regular spaces kept), nor classified
        /// (if / for / let / output / comment : Next steps).
        content: &'a str,
        /// 1-indexed, line where the opening delimiter begins.
        line: usize,
        delimiter: DelimiterType,
        /// `{%-` or `{#-`
        trim_left: bool,
        /// `-%}` or `-#}`
        trim_right: bool,
    },
}

/// 1-indexed line of the byte position `pos` in `source` (§5.3:
/// recalculated on demand by counting preceding `\n`, never
/// maintained as a running counter during scan — eliminates all risk of drift).
fn line_at(source: &str, pos: usize) -> usize {
    1 + source.as_bytes()[..pos]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
}

/// Pass 1 (§5.3): splits `source` into a flat sequence of
/// `Static` / `Tagged` tokens. Panics if an opening tag (`{%` or `{#`)
/// doesn't have a matching closing tag before the EOF.
pub fn tokenize(source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut pos = 0usize;

    loop {
        let next_instr = source[pos..].find("{%");
        let next_comm = source[pos..].find("{#");

        let (rel, delimiter, opening, closing) = match (next_instr, next_comm) {
            (None, None) => {
                tokens.push(Token::Static {
                    text: &source[pos..],
                });
                break;
            }
            (Some(i), None) => (i, DelimiterType::Instruction, "{%", "%}"),
            (None, Some(c)) => (c, DelimiterType::Comment, "{#", "#}"),
            (Some(i), Some(c)) if i <= c => (i, DelimiterType::Instruction, "{%", "%}"),
            (Some(_), Some(c)) => (c, DelimiterType::Comment, "{#", "#}"),
        };

        let tag_start = pos + rel;
        tokens.push(Token::Static {
            text: &source[pos..tag_start],
        });

        let line = line_at(source, tag_start);
        let after_opening = tag_start + opening.len();

        let (trim_left, content_start) = match source[after_opening..].strip_prefix('-') {
            Some(_) => (true, after_opening + 1),
            None => (false, after_opening),
        };

        let closing_start = match source[content_start..].find(closing) {
            Some(f) => content_start + f,
            None => panic!(
                "tmplx build error line {line}: opened tag '{opening}' never closed (no matching '{closing}' found)."
            ),
        };

        let (content_end, trim_right) =
            if closing_start > content_start && source[..closing_start].ends_with('-') {
                (closing_start - 1, true)
            } else {
                (closing_start, false)
            };

        tokens.push(Token::Tagged {
            content: &source[content_start..content_end],
            line,
            delimiter,
            trim_left,
            trim_right,
        });

        pos = closing_start + closing.len();
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reconstructs the original source from tokens: guarantees that
    /// pass 1 never loses, duplicates, nor moves any byte.
    fn reconstruct(tokens: &[Token]) -> String {
        let mut out = String::new();
        for t in tokens {
            match t {
                Token::Static { text } => out.push_str(text),
                Token::Tagged {
                    content,
                    delimiter,
                    trim_left,
                    trim_right,
                    ..
                } => {
                    let (opening, closing) = match delimiter {
                        DelimiterType::Instruction => ("{%", "%}"),
                        DelimiterType::Comment => ("{#", "#}"),
                    };
                    out.push_str(opening);
                    if *trim_left {
                        out.push('-');
                    }
                    out.push_str(content);
                    if *trim_right {
                        out.push('-');
                    }
                    out.push_str(closing);
                }
            }
        }
        out
    }

    #[test]
    fn source_without_any_tag() {
        let src = "just text, no tags.";
        let toks = tokenize(src);
        assert_eq!(toks, vec![Token::Static { text: src }]);
    }

    #[test]
    fn empty_source() {
        let toks = tokenize("");
        assert_eq!(toks, vec![Token::Static { text: "" }]);
    }

    #[test]
    fn single_tag_without_surrounding_text() {
        let toks = tokenize("{% endif %}");
        assert_eq!(
            toks,
            vec![
                Token::Static { text: "" },
                Token::Tagged {
                    content: " endif ",
                    line: 1,
                    delimiter: DelimiterType::Instruction,
                    trim_left: false,
                    trim_right: false,
                },
                Token::Static { text: "" },
            ]
        );
    }

    #[test]
    fn static_tagged_static_alternation() {
        let src = "before {%= x %} after";
        let toks = tokenize(src);
        assert_eq!(
            toks,
            vec![
                Token::Static { text: "before " },
                Token::Tagged {
                    content: "= x ",
                    line: 1,
                    delimiter: DelimiterType::Instruction,
                    trim_left: false,
                    trim_right: false,
                },
                Token::Static { text: " after" },
            ]
        );
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn comment_produces_a_comment_delimiter() {
        let src = "{# this is ignored #}";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged {
                delimiter, content, ..
            } => {
                assert_eq!(*delimiter, DelimiterType::Comment);
                assert_eq!(*content, " this is ignored ");
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn left_truncation_only() {
        let src = "x {%- if a %} y";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged {
                trim_left,
                trim_right,
                content,
                ..
            } => {
                assert!(*trim_left);
                assert!(!*trim_right);
                assert_eq!(*content, " if a ");
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn right_truncation_only() {
        let src = "x {% if a -%} y";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged {
                trim_left,
                trim_right,
                content,
                ..
            } => {
                assert!(!*trim_left);
                assert!(*trim_right);
                assert_eq!(*content, " if a ");
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn truncation_both_sides() {
        let src = "x {%- if a -%} y";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged {
                trim_left,
                trim_right,
                content,
                ..
            } => {
                assert!(*trim_left);
                assert!(*trim_right);
                assert_eq!(*content, " if a ");
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn double_truncation_without_content() {
        let src = "{%--%}";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged {
                trim_left,
                trim_right,
                content,
                ..
            } => {
                assert!(*trim_left);
                assert!(*trim_right);
                assert_eq!(*content, "");
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn left_truncation_only_one_char_content() {
        let src = "{%- %}";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged {
                trim_left,
                trim_right,
                content,
                ..
            } => {
                assert!(*trim_left);
                assert!(!*trim_right);
                assert_eq!(*content, " ");
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn multiple_successive_tags() {
        let src = "{% if a %}{% if b %}{% endif %}{% endif %}";
        let toks = tokenize(src);
        let nb_tagged = toks
            .iter()
            .filter(|t| matches!(t, Token::Tagged { .. }))
            .count();
        assert_eq!(nb_tagged, 4);
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn correct_line_number_across_multiple_lines() {
        let src = "line1\nline2\nline3 {% x %}\nline4";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged { line, .. } => assert_eq!(*line, 3),
            other => panic!("expected Tagged, got {other:?}"),
        }
    }

    #[test]
    fn multiline_tag_line_number_is_opening_delimiter() {
        let src = "before\n{% let x =\n  42; %}\nafter";
        let toks = tokenize(src);
        match &toks[1] {
            Token::Tagged { line, content, .. } => {
                assert_eq!(*line, 2);
                assert_eq!(*content, " let x =\n  42; ");
            }
            other => panic!("expected Tagged, got {other:?}"),
        }
        assert_eq!(reconstruct(&toks), src);
    }

    #[test]
    fn static_text_accented_chars_dont_panic() {
        let src = "Bonjour, événement à Grasse ! {%= name %} Voilà.";
        let toks = tokenize(src);
        assert_eq!(reconstruct(&toks), src);
        match &toks[0] {
            Token::Static { text } => assert_eq!(*text, "Bonjour, événement à Grasse ! "),
            other => panic!("expected Static, got {other:?}"),
        }
    }

    #[test]
    #[should_panic(expected = "never closed")]
    fn unclosed_instruction_tag_panics() {
        tokenize("before {% if x without closer");
    }

    #[test]
    #[should_panic(expected = "never closed")]
    fn unclosed_comment_panics() {
        tokenize("before {# without closer");
    }

    #[test]
    fn panic_message_names_the_right_line() {
        let res = std::panic::catch_unwind(|| tokenize("l1\nl2\n{% if x"));
        let err = res.unwrap_err();
        let msg = err
            .downcast_ref::<String>()
            .map(|s| s.as_str())
            .or_else(|| err.downcast_ref::<&str>().copied())
            .expect("expected panic msg");
        assert!(msg.contains("line 3"), "got: {msg}");
    }

    #[test]
    fn mockup_tokenizes_without_crashing() {
        let src = include_str!("../templates/mockup.html");
        let toks = tokenize(src);

        let expected = src.matches("{%").count() + src.matches("{#").count();
        let actual = toks
            .iter()
            .filter(|t| matches!(t, Token::Tagged { .. }))
            .count();
        assert_eq!(actual, expected);

        assert_eq!(reconstruct(&toks), src);
    }
}

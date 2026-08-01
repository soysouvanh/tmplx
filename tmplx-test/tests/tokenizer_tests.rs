use tmplx::build_logic::tokenizer::*;

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
    let src = include_str!("../../tmplx-test/templates/mockup.html");
    let toks = tokenize(src);

    let expected = src.matches("{%").count() + src.matches("{#").count();
    let actual = toks
        .iter()
        .filter(|t| matches!(t, Token::Tagged { .. }))
        .count();
    assert_eq!(actual, expected);

    assert_eq!(reconstruct(&toks), src);
}

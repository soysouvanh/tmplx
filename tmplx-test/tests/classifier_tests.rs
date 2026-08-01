use tmplx::build_logic::classifier::*;
use tmplx::build_logic::tokenizer::DelimiterType;

const I: DelimiterType = DelimiterType::Instruction;
const C: DelimiterType = DelimiterType::Comment;

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

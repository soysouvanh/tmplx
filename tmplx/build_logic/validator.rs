// build_logic/validator.rs — Pass 2 (§5.3), classifier updated
// at step 5b to use build_logic/classifier.rs.
//
// Same sharing pattern as tokenizer.rs: this file is the single
// source of truth, included by build.rs (actual usage) and by src/main.rs
// under #[cfg(test)] (so `cargo test` executes it).
//
// The minimal local classifier introduced here at step 3 (announced as
// intentionally partial) was removed: this pass now uses
// the full classifier from classifier.rs (step 5b), to prevent
// two separate classifiers from drifting apart over time.
// Observable behavior unchanged for block validation; only the
// internal syntax of tags (output/let) is now also checked
// in passing, with panic on unrecognized content — strictly more
// strict than before, never less.

use crate::classifier::{ClassifiedTag, classify};
use crate::tokenizer::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockType {
    If,
    For,
}

impl BlockType {
    fn keyword(self) -> &'static str {
        match self {
            BlockType::If => "if",
            BlockType::For => "for",
        }
    }
    fn closer(self) -> &'static str {
        match self {
            BlockType::If => "endif",
            BlockType::For => "endfor",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct OpenBlock {
    block_type: BlockType,
    line: usize,
}

/// Message for '{% else|endif|endfor %}' encountered while the top of
/// the stack is of the wrong type. The endfor-facing-if direction is given
/// verbatim by the spec (§7.1, message 4); symmetrical directions
/// (endif facing for, else facing for) reuse the same template for
/// consistency, but are NOT spec citations.
fn message_wrong_type(line_here: usize, tag_encountered: &str, open: &OpenBlock) -> String {
    format!(
        "tmplx build error line {line_here}: '{{% {tag_encountered} %}}' encountered while the block opened at line {} is a '{}' (expected '{{% {} %}}').",
        open.line,
        open.block_type.keyword(),
        open.block_type.closer(),
    )
}

/// Message for '{% else|endif|endfor %}' encountered while no block
/// is open. The spec cites "an else/endif/endfor without open block"
/// as trigger (§5.3) but does not give the exact text — this
/// is an implementation decision, in the style of other messages.
fn message_no_block_open(line_here: usize, tag_encountered: &str) -> String {
    format!(
        "tmplx build error line {line_here}: '{{% {tag_encountered} %}}' encountered while no block is open."
    )
}

/// Pass 2 (§5.3): checks if/for ↔ endif/endfor pairing on
/// the entire token stream. Panics at the first mismatch
/// encountered, or if the stack is not empty at EOF. No
/// nesting depth limit (§5.3).
///
/// Implementation choice not dictated by the spec: if MULTIPLE blocks
/// remain open at EOF, the reported block is the most
/// recently opened (top of stack) — not the first. See the test
/// `multiple_blocks_unclosed_reports_innermost` below.
pub fn validate_pairing(tokens: &[Token]) {
    let mut stack: Vec<OpenBlock> = Vec::new();

    for tok in tokens {
        let (content, line, delimiter) = match tok {
            Token::Tagged {
                content,
                line,
                delimiter,
                ..
            } => (*content, *line, *delimiter),
            Token::Static { .. } => continue,
        };

        match classify(content, delimiter, line) {
            ClassifiedTag::If { .. } => stack.push(OpenBlock {
                block_type: BlockType::If,
                line,
            }),
            ClassifiedTag::For { .. } => stack.push(OpenBlock {
                block_type: BlockType::For,
                line,
            }),

            ClassifiedTag::Else => match stack.last() {
                Some(b) if b.block_type == BlockType::If => { /* ok: the if block stays open */ }
                Some(b) => panic!("{}", message_wrong_type(line, "else", b)),
                None => panic!("{}", message_no_block_open(line, "else")),
            },

            ClassifiedTag::ElseIf { .. } => match stack.last() {
                Some(b) if b.block_type == BlockType::If => { /* ok */ }
                Some(b) => panic!("{}", message_wrong_type(line, "else if", b)),
                None => panic!("{}", message_no_block_open(line, "else if")),
            },

            ClassifiedTag::EndIf => match stack.pop() {
                Some(b) if b.block_type == BlockType::If => {}
                Some(b) => panic!("{}", message_wrong_type(line, "endif", &b)),
                None => panic!("{}", message_no_block_open(line, "endif")),
            },

            ClassifiedTag::EndFor => match stack.pop() {
                Some(b) if b.block_type == BlockType::For => {}
                Some(b) => panic!("{}", message_wrong_type(line, "endfor", &b)),
                None => panic!("{}", message_no_block_open(line, "endfor")),
            },

            ClassifiedTag::GenericClose => match stack.pop() {
                Some(_) => {} // ok: GenericClose generically closes any block
                None => panic!("{}", message_no_block_open(line, "}")),
            },

            ClassifiedTag::RawOutput { .. }
            | ClassifiedTag::EscapedOutput { .. }
            | ClassifiedTag::EscapedOutputJs { .. }
            | ClassifiedTag::EscapedOutputUrl { .. }
            | ClassifiedTag::Let { .. }
            | ClassifiedTag::Extends { .. }
            | ClassifiedTag::Include { .. }
            | ClassifiedTag::Block { .. }
            | ClassifiedTag::EndBlock
            | ClassifiedTag::Comment => {}
        }
    }

    // Stack not empty at EOF (§5.3) — verbatim message §7.1 #1.
    if let Some(b) = stack.last() {
        panic!(
            "tmplx build error: block tag '{{% {} %}}' opened at line {} never closed.",
            b.block_type.keyword(),
            b.line
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

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
        let result = std::panic::catch_unwind(|| {
            validate_pairing(&tokenize("{% for x in l %}y{% endif %}"))
        });
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
        let src = include_str!("../templates/mockup.html");
        validate_pairing(&tokenize(src));
    }
}

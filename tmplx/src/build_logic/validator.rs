// build_logic/validator.rs — Pass 2 (§5.3), uses the full
// classifier from classifier.rs.
//
// The minimal local classifier introduced here at step 3 (announced as
// intentionally partial) was removed: this pass now uses
// the full classifier from classifier.rs (step 5b), to prevent
// two separate classifiers from drifting apart over time.
// Observable behavior unchanged for block validation; only the
// internal syntax of tags (output/let) is now also checked
// in passing, with panic on unrecognized content — strictly more
// strict than before, never less.

use super::classifier::{ClassifiedTag, classify};
use super::tokenizer::Token;

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

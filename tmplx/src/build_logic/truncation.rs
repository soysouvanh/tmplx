// build_logic/truncation.rs — Pass 3 of the pipeline (§5.3).

use super::tokenizer::Token;

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

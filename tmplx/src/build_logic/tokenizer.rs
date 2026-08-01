// build_logic/tokenizer.rs — Pass 1 of the pipeline (§5.3).
//
// Single source of truth for the tokenizer. Zero allocations.

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

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

fn find_next_delimiter(source: &str, start: usize) -> Option<(usize, DelimiterType)> {
    let bytes = source.as_bytes();
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' {
            if bytes[i + 1] == b'%' {
                return Some((i, DelimiterType::Instruction));
            }
            if bytes[i + 1] == b'#' {
                return Some((i, DelimiterType::Comment));
            }
        }
        i += 1;
    }
    None
}

/// Pass 1 (§5.3): splits `source` into a flat sequence of
/// `Static` / `Tagged` tokens. Panics if an opening tag (`{%` or `{#`)
/// doesn't have a matching closing tag before the EOF.
pub fn tokenize(source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut pos = 0usize;
    let mut line = 1usize;
    let mut last_counted = 0usize;

    loop {
        let (rel, delimiter, opening, closing) = match find_next_delimiter(source, pos) {
            None => {
                tokens.push(Token::Static {
                    text: &source[pos..],
                });
                break;
            }
            Some((abs_pos, DelimiterType::Instruction)) => {
                (abs_pos - pos, DelimiterType::Instruction, "{%", "%}")
            }
            Some((abs_pos, DelimiterType::Comment)) => {
                (abs_pos - pos, DelimiterType::Comment, "{#", "#}")
            }
        };

        let tag_start = pos + rel;

        line += source[last_counted..tag_start]
            .bytes()
            .filter(|&b| b == b'\n')
            .count();
        last_counted = tag_start;

        tokens.push(Token::Static {
            text: &source[pos..tag_start],
        });

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

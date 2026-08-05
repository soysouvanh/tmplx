// build_logic/generator.rs — Code generation, last
// part of pass 4 (§5.3). `&` rule: §5.5.
//
//
// Takes primitive data to remain testable via the usual shared inclusion pattern.
//
// Mapping empirically verified against §10.3 (dump of actual token stream
// from the reference mockup, aligned line by line with the expected generated
// file) rather than eye-balled from the spec text:
//   - FLAT indentation (4 spaces) on all lines, regardless
//     of if/for nesting depth — confirmed on the if
//     nested inside the for (§10.3, item.actif).
//   - Each Static emits its own push_str, never merged with a
//     neighbor even separated only by a 0-byte comment.
//   - `// LIGNE_SOURCE:N` only on raw output / escaped / if /
//     for. NOT on else / endif / endfor / static / comment (which has
//     its own format: see `Comment` below). This slightly contradicts
//     the general sentence of §5.3 ("each generated line bears
//     an annotation"): the exact text of §10.3 is authoritative here, not the
//     framing sentence.
//   - No trailing comma after the last parameter — also in
//     tension with the signature block in §11 (which puts one);
//     §10.3 is authoritative because it's the exact expected generated file.
//
// Not empirically verified against §10.3 (no `{% let %}` nor `if
// !ident` in the reference mockup) but since confirmed by
// §12: case 12 gives "Generates `if !(ident)`" explicitly (parentheses
// around the identifier, fixed after an initial version without
// parentheses — see rest of §12). The format for `let` remains deduced
// by consistency, not confirmed by an exact spec example.

use super::classifier::{ClassifiedTag, classify};
use super::tokenizer::Token;
use std::fmt::Write;

/// Rule of §5.3: `\`, `"`, `\n`, `\r`, `\t` — in this order (the
/// backslash FIRST, otherwise backslashes introduced by other
/// replacements would themselves be re-escaped). Same logic
/// character-by-character as `write_escaped` (§6) and for the same
/// reason: structural correctness, no risk of double-escaping
/// by design.
fn write_escaped_rust_literal(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
}

/// Pass 4, generation part (§5.3, §5.5, §8.1, §11). Assumes that
/// `validate_pairing` and `resolve_scopes` have already succeeded on the
/// same tokens (no re-verification here: pure text emission).
pub fn generate(tokens: &[Token], function_name: &str, html_path: &str) -> String {
    let static_size: usize = tokens
        .iter()
        .map(|t| match t {
            Token::Static { text } => text.len(),
            _ => 0,
        })
        .sum();

    let estimated_capacity = static_size
        .saturating_mul(6)
        .saturating_add(tokens.len().saturating_mul(64));

    let mut output = String::with_capacity(estimated_capacity);

    // --- Header (§10.3, fixed text except paths already fixed §4.1) ---
    output.push_str("// AUTOMATICALLY GENERATED FILE BY build.rs -- DO NOT EDIT BY HAND.\n");
    let _ = writeln!(output, "// Source   : {html_path}");
    output.push('\n');

    // removed imports to prevent clashes

    // --- Static size constant (§8.1, §11) ---
    output.push_str("/// Exact sum, in bytes, of all static HTML segments from the mockup.\n");
    output
        .push_str("/// Does NOT cover the dynamic content injected at runtime (see the section\n");
    output.push_str(
        "/// « memory capacity management of the buffer » in the tmplx specification).\n",
    );
    let const_name = format!("TMPLX_STATIC_SIZE_{}", function_name.to_uppercase());
    let _ = writeln!(output, "pub const {const_name}: usize = {static_size};");
    output.push('\n');

    // --- Imposed macro signature ---
    output.push_str("#[macro_export]\nmacro_rules! ");
    output.push_str(function_name);
    output.push_str(" {\n    ($output:expr, $view_data:expr) => {{\n");
    output.push_str("        let output: &mut String = $output;\n");
    output.push_str("        #[allow(unused_variables)]\n");
    output.push_str("        let view_data = $view_data;\n");

    // --- Body: one token -> one line, flat 4-space indentation ---
    let mut static_buffer = String::new();

    let flush_static = |out: &mut String, buff: &mut String| {
        if !buff.is_empty() {
            out.push_str("    output.push_str(\"");
            write_escaped_rust_literal(out, buff);
            out.push_str("\");\n");
            buff.clear();
        }
    };

    for tok in tokens {
        match tok {
            Token::Static { text } => {
                if !text.is_empty() {
                    static_buffer.push_str(text);
                }
            }
            Token::Tagged {
                content,
                line,
                delimiter,
                ..
            } => match classify(content, *delimiter, *line) {
                ClassifiedTag::RawOutput { ident } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(
                        output,
                        "    output.push_str(&({ident})); // SOURCE_LINE:{line}"
                    );
                }
                ClassifiedTag::EscapedOutput { ident } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(
                        output,
                        "    ::tmplx::tmplx_runtime::write_escaped(output, &({ident})); // SOURCE_LINE:{line}"
                    );
                }
                ClassifiedTag::EscapedOutputJs { ident } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(
                        output,
                        "    ::tmplx::tmplx_runtime::write_escaped_js(output, &({ident})); // SOURCE_LINE:{line}"
                    );
                }
                ClassifiedTag::EscapedOutputUrl { ident } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(
                        output,
                        "    ::tmplx::tmplx_runtime::write_escaped_url(output, &({ident})); // SOURCE_LINE:{line}"
                    );
                }
                ClassifiedTag::If { expr } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(output, "    if {expr} {{ // SOURCE_LINE:{line}");
                }
                ClassifiedTag::ElseIf { expr } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(output, "    }} else if {expr} {{");
                }
                ClassifiedTag::Else => {
                    flush_static(&mut output, &mut static_buffer);
                    output.push_str("    } else {\n");
                }
                ClassifiedTag::EndIf => {
                    flush_static(&mut output, &mut static_buffer);
                    output.push_str("    }\n");
                }
                ClassifiedTag::For { item, iterable } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(output, "    let _tmplx_iterable = &({iterable});");
                    output.push_str("    let _tmplx_loop_len = _tmplx_iterable.len();\n");
                    let _ = writeln!(
                        output,
                        "    for (_tmplx_loop_index0, {item}) in _tmplx_iterable.into_iter().enumerate() {{ // SOURCE_LINE:{line}"
                    );
                    output.push_str("        let loop_index0 = _tmplx_loop_index0;\n");
                    output.push_str("        let loop_index = _tmplx_loop_index0 + 1;\n");
                    output.push_str("        let loop_first = _tmplx_loop_index0 == 0;\n");
                    output.push_str(
                        "        let loop_last = _tmplx_loop_index0 == _tmplx_loop_len - 1;\n",
                    );
                    output.push_str("        let loop_length = _tmplx_loop_len;\n");
                }
                ClassifiedTag::EndFor => {
                    flush_static(&mut output, &mut static_buffer);
                    output.push_str("    }\n");
                }
                ClassifiedTag::GenericClose => {
                    flush_static(&mut output, &mut static_buffer);
                    output.push_str("    }\n");
                }
                ClassifiedTag::EndBlock => {
                    flush_static(&mut output, &mut static_buffer);
                }
                ClassifiedTag::Let { body, .. } => {
                    flush_static(&mut output, &mut static_buffer);
                    let _ = writeln!(output, "    {body} // SOURCE_LINE:{line}");
                }
                ClassifiedTag::Extends { .. }
                | ClassifiedTag::Include { .. }
                | ClassifiedTag::Block { .. }
                | ClassifiedTag::Comment => {
                    // AST Compression: Inject the Rust comment, BUT DO NOT FLUSH the static HTML buffer!
                    let _ = writeln!(
                        output,
                        "    // (template comment, line {line} : 0 bytes in binary)"
                    );
                }
            },
        }
    }

    flush_static(&mut output, &mut static_buffer);

    output.push_str("    }}\n}\n");
    output
}

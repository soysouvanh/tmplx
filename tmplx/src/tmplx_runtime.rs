//! Zero-Allocation Runtime Utilities.
//!
//! This module is the **only** framework code that remains in the compiled
//! final application (runtime). It exclusively contains escaping routines necessary
//! for an airtight defense against XSS attacks and code injections.
//!
//! # Design Choice: Security by Explicit Code
//! Unlike engines that blindly escape everything (wasting CPU cycles searching
//! for metacharacters in all harmless strings), Tmplx mandates an explicit choice directly within your templates:
//! - `{%= %}`: Raw output (no escaping), the fastest option.
//! - `{%%= %}`: Output with HTML escaping via [`write_escaped`].
//! - `{%js= %}` / `{%url= %}`: Contextual JavaScript and URL escaping.
//!
//! # Performance
//! These algorithms process strings fluidly (at the byte-level) to perform
//! contiguous replacements into the target text stream, thereby entirely eliminating intermediate allocations.

/// Escapes standard HTML entities (`&`, `<`, `>`, `"`, `'`) to prevent XSS attacks.
/// The sanitized text is appended directly into a pre-allocated `.push_str()` String buffer,
/// preventing the creation of useless substrings, conforming strictly to the `zero-allocation` philosophy.
#[inline(always)]
pub fn write_escaped(output: &mut String, value: &str) {
    let mut last = 0;
    for (i, b) in value.bytes().enumerate() {
        let esc = match b {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            b'\'' => "&#39;",
            _ => continue,
        };
        output.push_str(&value[last..i]);
        output.push_str(esc);
        last = i + 1;
    }
    output.push_str(&value[last..]);
}

/// Constructs a string compatible with JavaScript contexts.
///
/// Replaces backslashes, quotes (single, double, and backticks), line endings,
/// and HTML tags (using `\x3E` and `\x26` hex syntax) to prevent escapes from
/// both the `<script>` context and template literals (`` ` ``).
#[inline(always)]
pub fn write_escaped_js(output: &mut String, value: &str) {
    let mut last = 0;
    for (i, b) in value.bytes().enumerate() {
        let esc = match b {
            b'\\' => "\\\\",
            b'"' => "\\\"",
            b'\'' => "\\'",
            b'`' => "\\`",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            b'<' => "\\x3C",
            b'>' => "\\x3E",
            b'&' => "\\x26",
            _ => continue,
        };
        output.push_str(&value[last..i]);
        output.push_str(esc);
        last = i + 1;
    }
    output.push_str(&value[last..]);
}

/// Encodes the value to be safely inserted as a key or component within a URL,
/// by converting all non-standard characters into their `%XX` form.
#[inline(always)]
pub fn write_escaped_url(output: &mut String, value: &str) {
    const HEX_CHARS: &[u8; 16] = b"0123456789ABCDEF";
    let mut last = 0;
    let bytes = value.as_bytes();
    for i in 0..bytes.len() {
        let b = bytes[i];
        if matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~') {
            continue;
        }
        output.push_str(&value[last..i]);
        let mut enc = [b'%', 0, 0];
        enc[1] = HEX_CHARS[(b >> 4) as usize];
        enc[2] = HEX_CHARS[(b & 15) as usize];
        output.push_str(unsafe { std::str::from_utf8_unchecked(&enc) });
        last = i + 1;
    }
    output.push_str(&value[last..]);
}

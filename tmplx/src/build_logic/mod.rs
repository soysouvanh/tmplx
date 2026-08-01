//! Tmplx Internal Compilation Engine (The 4-Pass Pipeline)
//!
//! This module contains all the engineering required to parse source files (HTML)
//! and metamorphose them into an AST, then into static native code.
//! Its architecture is designed to be fully auditable and mathematically verifiable,
//! via a succession of strict, well-defined steps (passes).
//!
//! # The Passes
//!
//! 1. **Tokenizer** ([`tokenizer`]): Low-level byte-by-byte analysis (zero-allocation) strictly
//!    separating raw text (static) from active tags `{% ... %}` (dynamic).
//! 2. **Classifier & Validator** ([`classifier`], [`validator`]): Active tokens
//!    are verified, classified (`If`, `For`, `Block`...) and paired (scope closure).
//! 3. **Truncation** ([`truncation`]): Rigorously applies whitespace trimming directives
//!    (`{%-` and `-%}`) on the associated static syntax tree.
//! 4. **Generator** ([`generator`]): Synthesizes the single resulting Rust code (a nested
//!    static macro ready for user compilation).
//!
//! (Internal tool exclusively orchestrated by `tmplx::compiler::build_workspace`)

pub mod classifier;
pub mod generator;
pub mod tokenizer;
pub mod truncation;
pub mod validator;

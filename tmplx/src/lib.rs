//! # Tmplx: Zero-Allocation, Statically Compiled Template Engine
//!
//! **Tmplx** is an HTML template engine engineered for extreme performance (State of the Art)
//! and absolute security. Unlike traditional engines (which interpret at runtime
//! or dynamically allocate memory), Tmplx translates your HTML templates directly
//! into highly optimized Rust macros during the compilation phase (via `build.rs`).
//!
//! ## Framework Architecture
//!
//! The framework is divided into two strictly isolated domains:
//! - **Compile-Time**: The [`compiler`] module and the internal logic
//!   ([`build_logic`]) are executed. They meticulously parse your HTML files (tokenization,
//!   pairing validation, application of truncation rules) and generate native Rust code.
//! - **Runtime**: Only the [`tmplx_runtime`] module remains, providing minimalist
//!   tooling (exclusively dedicated to XSS escaping) designed to write data
//!   directly into the final buffer without *any* double allocation.
//!
//! ## SOTA (State of the Art) Design Principles
//!
//! 1. **Extreme Performance**: The size of the static HTML segments is computed at compile-time.
//!    At runtime, a single pre-sized allocation is sufficient for the entire page rendering.
//! 2. **Security by Default**: XSS injections are structurally mitigated via explicit
//!    stream-based escaping routines (`{%%= %}` for HTML, `{%js= %}` for JS).
//! 3. **Pedagogical Architecture**: The 4-pass pipeline is designed to be read and audited.
//!    Everything is crystal clear, explicit, with no hidden abstractions.

pub mod build_logic;
pub mod compiler;
pub mod tmplx_runtime;

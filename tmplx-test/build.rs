// build.rs (for tmplx-test)
// Executes the tmplx engine compiler on the local templates/ dir,
// and additionally generates a small synthetic mockup snippet for §12 tests.

use tmplx::build_logic::{generator, tokenizer, truncation, validator};
use tmplx::compiler::build_workspace;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not defined by Cargo");

    // We call the public API exposed by the framework!
    let template_dir = std::path::Path::new("templates");
    let output_path = std::path::Path::new(&out_dir);
    build_workspace(template_dir, output_path);

    // --- Extra generation, from synthetic
    // templates (not templates/mockup.html), to provide
    // "actually compiles and runs" coverage to §12 cases that the
    // reference mockup doesn't exercise: case 10 (nested `{% for %}`
    // via a field, item.sub_list) and case 12 (`{% if !ident %}`).
    // Written to a SEPARATE file: template_gen.rs (§10.3 golden test)
    // is untouched.
    let extra_template = "{%- for g in groups %}{%- for m in g.members %}[{%= m.name %}]{%- endfor %}{%- endfor %}{% if !is_active %}(INACTIVE){% endif %}";
    let mut tokens_extra = tokenizer::tokenize(extra_template);
    validator::validate_pairing(&tokens_extra);
    truncation::apply_truncation(&mut tokens_extra);
    let code_extra = generator::generate(&tokens_extra, "render_extra_12", "extra_template");
    let extra_path = output_path.join("extra_tests_gen.rs");
    std::fs::write(&extra_path, &code_extra)
        .unwrap_or_else(|e| panic!("writing {extra_path:?} failed: {e}"));
}

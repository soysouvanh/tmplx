// src/main.rs (in tmplx-test)

use tmplx_test::generated::TMPLX_STATIC_SIZE_RENDER_MOCKUP;
use tmplx_test::models::User;
use tmplx_test::render_mockup;

pub struct MockupViewData<'a> {
    pub user: &'a str,
    pub html_inject_code: &'a str,
    pub is_admin: bool,
    pub user_list: &'a [User],
}

fn main() {
    // Exact example from §10.4.
    let user = "Marie <Admin> & Cie";
    let html_inject_code = "<strong>Bienvenue</strong>";
    let is_admin = true;
    let user_list = vec![
        User {
            name: "Paul \"the Great\"".to_string(),
            is_active: true,
        },
        User {
            name: "O'Brien <script>".to_string(),
            is_active: false,
        },
    ];

    let mut output = String::with_capacity(TMPLX_STATIC_SIZE_RENDER_MOCKUP + 256);
    let capacity_before = output.capacity();

    let view_data = MockupViewData {
        user,
        html_inject_code,
        is_admin,
        user_list: &user_list,
    };
    render_mockup!(&mut output, &view_data);
    let capacity_after = output.capacity();

    println!("TMPLX_STATIC_SIZE = {TMPLX_STATIC_SIZE_RENDER_MOCKUP}");
    println!(
        "Output size                = {} bytes (§10.4 expects 615)",
        output.len()
    );
    println!(
        "Capacity before/after call = {capacity_before} / {capacity_after} (realloc = {})",
        capacity_before != capacity_after
    );
    println!("--- HTML generated ---");
    println!("{output}");

    assert_eq!(
        output.len(),
        615,
        "the output must be exactly 615 bytes (§10.4)"
    );
    assert_eq!(
        capacity_before, capacity_after,
        "no reallocation must occur (§10.4)"
    );
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Integrally reproduces the example from §10.4: exact call, exact
    /// HTML output (615 bytes, text identical character by
    /// character), and no reallocation with the capacity suggested by
    /// the formula in §8.2. Expected text programmatically extracted from
    /// the spec file, not hand-retyped.
    #[test]
    fn paragraph_10_4_example_reproduced_identically() {
        let user = "Marie <Admin> & Cie";
        let html_inject_code = "<strong>Bienvenue</strong>";
        let is_admin = true;
        let user_list = vec![
            User {
                name: "Paul \"the Great\"".to_string(),
                is_active: true,
            },
            User {
                name: "O'Brien <script>".to_string(),
                is_active: false,
            },
        ];

        let mut output = String::with_capacity(TMPLX_STATIC_SIZE_RENDER_MOCKUP + 256);
        let capacity_before = output.capacity();

        let view_data = MockupViewData {
            user,
            html_inject_code,
            is_admin,
            user_list: &user_list,
        };
        render_mockup!(&mut output, &view_data);
        let capacity_after = output.capacity();

        let expected = r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="utf-8">
    <title>Marie <Admin> & Cie</title>
</head>
<body>
    <h1>Bonjour, Marie <Admin> & Cie</h1>
    <div class="bandeau-admin">
        <p>Zone d'administration active.</p>
        
        <div class="contenu-injecte"><strong>Bienvenue</strong></div>
    </div>

    <ul class="liste-utilisateurs">
        <li>
            <span>Paul &quot;the Great&quot;</span>
            <span class="badge-actif">actif</span>
        </li>
        <li>
            <span>O&#39;Brien &lt;script&gt;</span>
            
        </li>
    </ul>
</body>
</html>
"#;

        assert_eq!(output, expected);
        assert_eq!(output.len(), 615);
        assert_eq!(capacity_before, capacity_after, "unexpected reallocation");
    }

    /// Same example, but with the 1024 bytes capacity mentioned in
    /// the sentence of §10.4 (instead of 473+256=729 from the code of the same
    /// section): the two figures appear in the spec without being
    /// explicitly reconciled — both are tested separately for
    /// total fidelity, rather than choosing only one.
    #[test]
    fn no_reallocation_with_capacity_1024_as_well() {
        let user = "Marie <Admin> & Cie";
        let html_inject_code = "<strong>Bienvenue</strong>";
        let is_admin = true;
        let user_list = vec![
            User {
                name: "Paul \"the Great\"".to_string(),
                is_active: true,
            },
            User {
                name: "O'Brien <script>".to_string(),
                is_active: false,
            },
        ];

        let mut output = String::with_capacity(1024);
        let capacity_before = output.capacity();

        let view_data = MockupViewData {
            user,
            html_inject_code,
            is_admin,
            user_list: &user_list,
        };
        render_mockup!(&mut output, &view_data);
        assert_eq!(
            output.capacity(),
            capacity_before,
            "unexpected reallocation"
        );
        assert_eq!(output.len(), 615);
    }

    #[test]
    fn static_size_constant_is_473() {
        assert_eq!(TMPLX_STATIC_SIZE_RENDER_MOCKUP, 473);
    }

    /// §12, case 8: `{% for x in list %}` with an empty `list` -> zero
    /// iteration, no panic. Real call with an empty Vec.
    #[test]
    fn case_12_08_loop_on_empty_list_zero_iteration_no_panic() {
        let mut output = String::new();
        let view_data = MockupViewData {
            user: "u",
            html_inject_code: "c",
            is_admin: false,
            user_list: &[],
        };
        render_mockup!(&mut output, &view_data); // No <li> tag should appear.
        assert!(!output.contains("<li>"), "output: {output}");
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    /// Example: Modular Architecture
    /// Demonstrates layout inheritance (`extends`) and component inclusion (`include`).
    #[test]
    fn example_01_modular_architecture() {
        // Data for both the main page and the included partial
        struct PageData {
            title: &'static str,
            user: User,
        }

        let mut output =
            String::with_capacity(tmplx_test::generated::TMPLX_STATIC_SIZE_RENDER_PAGE + 128);
        assert_eq!(
            output.capacity(),
            tmplx_test::generated::TMPLX_STATIC_SIZE_RENDER_PAGE + 128
        );

        let data = PageData {
            title: "Admin Dashboard",
            user: User {
                name: "John Doe".to_string(),
                is_active: true,
            },
        };

        // Note: The `render_page!` macro is generated by `build.rs` from `page.html`
        tmplx_test::render_page!(&mut output, &data);

        // Verify that the layout, page content, and partial content are all present
        assert!(output.contains("<title>Tmplx Example</title>"));
        assert!(output.contains("<h1>Admin Dashboard</h1>"));
        assert!(output.contains("<div class=\"card\">"));
        assert!(output.contains("<h2>John Doe</h2>"));
        assert!(output.contains("<span class=\"active\">Active</span>"));

        println!("Modular Architecture Output:\n{}", output);
    }

    /// Example: Local Assignment & Loops
    /// Demonstrates server-side variable assignment (`let`) and magic loop variables (`loop_index`).
    #[test]
    fn example_02_local_vars_and_loops() {
        struct VarsData {
            user: User,
            items: Vec<&'static str>,
        }

        let mut output =
            String::with_capacity(tmplx_test::generated::TMPLX_STATIC_SIZE_RENDER_VARIABLES + 256);

        let data = VarsData {
            user: User {
                name: "Alice".to_string(),
                is_active: false,
            },
            items: vec!["Item 1", "Item 2", "Item 3"],
        };

        // Note: The `render_variables!` macro is generated from `variables.html`
        tmplx_test::render_variables!(&mut output, &data);

        // Verify the local variable formatting
        assert!(output.contains("<p>Hello, Alice!</p>"));

        // Verify the logic from `is_even = loop_index % 2 == 0`
        assert!(output.contains("<li class=\"odd\">Item 1</li>"));
        assert!(output.contains("<li class=\"even\">Item 2</li>"));
        assert!(output.contains("<li class=\"odd\">Item 3</li>"));

        println!("Local Assignment Output:\n{}", output);
    }
}

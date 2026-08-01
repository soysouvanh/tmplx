use askama::Template;
use criterion::Criterion;
use maud::{PreEscaped, html};
use sailfish::TemplateOnce;
use std::hint::black_box;
use tmplx_test::models::User;
use tmplx_test::render_mockup;

// Askama Setup
#[derive(Template)]
#[template(path = "mockup.askama.html")]
struct AskamaExtremeTemplate<'a> {
    user: &'a str,
    html_inject_code: &'a str,
    is_admin: bool,
    user_list: &'a [User],
}

// Sailfish Setup
#[derive(TemplateOnce)]
#[template(path = "mockup.sailfish.stpl")]
struct SailfishExtremeTemplate<'a> {
    user: &'a str,
    html_inject_code: &'a str,
    is_admin: bool,
    user_list: &'a [User],
}

// Markup Setup
markup::define! {
    MarkupExtremeTemplate<'a>(
        user: &'a str,
        html_inject_code: &'a str,
        is_admin: bool,
        user_list: &'a [User],
    ) {
        @markup::doctype()
        html[lang="fr"] {
            head {
                meta[charset="utf-8"];
                title { @user }
            }
            body {
                h1 { "Bonjour, " @user }
                @if *is_admin {
                    div[class="bandeau-admin"] {
                        p { "Zone d'administration active." }
                        div[class="contenu-injecte"] {
                            @markup::raw(*html_inject_code)
                        }
                    }
                } else {
                    p { "Espace utilisateur standard." }
                }

                ul[class="liste-utilisateurs"] {
                    @for item in *user_list {
                        li {
                            span { @item.name }
                            @if item.is_active {
                                span[class="badge-actif"] { "actif" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Tmplx Setup
struct MockupViewData<'a> {
    pub user: &'a str,
    pub html_inject_code: &'a str,
    pub is_admin: bool,
    pub user_list: &'a [User],
}

fn bench_engines(c: &mut Criterion) {
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

    let mut group = c.benchmark_group("tmplx_vs_ecosystem_render_extreme");

    // 1. Tmplx
    group.bench_function("tmplx", |b| {
        b.iter(|| {
            output.clear();
            let view_data = MockupViewData {
                user: black_box(user),
                html_inject_code: black_box(html_inject_code),
                is_admin: black_box(is_admin),
                user_list: black_box(&user_list),
            };
            render_mockup!(black_box(&mut output), &view_data);
        })
    });

    // 2. Askama
    group.bench_function("askama", |b| {
        b.iter(|| {
            let tmpl = AskamaExtremeTemplate {
                user: black_box(user),
                html_inject_code: black_box(html_inject_code),
                is_admin: black_box(is_admin),
                user_list: black_box(&user_list),
            };
            let _ = black_box(tmpl.render().unwrap());
        })
    });

    // 3. Sailfish
    group.bench_function("sailfish", |b| {
        b.iter(|| {
            let tmpl = SailfishExtremeTemplate {
                user: black_box(user),
                html_inject_code: black_box(html_inject_code),
                is_admin: black_box(is_admin),
                user_list: black_box(&user_list),
            };
            let _ = black_box(tmpl.render_once().unwrap());
        })
    });

    // 4. Maud
    group.bench_function("maud", |b| {
        b.iter(|| {
            let user = black_box(user);
            let html_inject_code = black_box(html_inject_code);
            let is_admin = black_box(is_admin);
            let user_list = black_box(&user_list);

            let result = html! {
                (maud::DOCTYPE)
                html lang="fr" {
                    head {
                        meta charset="utf-8";
                        title { (user) }
                    }
                    body {
                        h1 { "Bonjour, " (user) }
                        @if is_admin {
                            div.bandeau-admin {
                                p { "Zone d'administration active." }
                                div.contenu-injecte {
                                    (PreEscaped(html_inject_code))
                                }
                            }
                        } @else {
                            p { "Espace utilisateur standard." }
                        }

                        ul.liste-utilisateurs {
                            @for item in user_list {
                                li {
                                    span { (item.name) }
                                    @if item.is_active {
                                        span.badge-actif { "actif" }
                                    }
                                }
                            }
                        }
                    }
                }
            };
            let _ = black_box(result.into_string());
        })
    });

    // 5. Markup
    group.bench_function("markup", |b| {
        b.iter(|| {
            let tmpl = MarkupExtremeTemplate {
                user: black_box(user),
                html_inject_code: black_box(html_inject_code),
                is_admin: black_box(is_admin),
                user_list: black_box(&user_list),
            };
            let _ = black_box(tmpl.to_string());
        })
    });

    group.finish();
}

pub fn main() {
    // 1. Run Criterion benchmarks
    let mut criterion = Criterion::default().configure_from_args();
    bench_engines(&mut criterion);
    criterion.final_summary();

    // 2. Parse results and print a beautiful table
    println!("\n\n{}", "=".repeat(80));
    println!("=== RÉSULTATS DES BENCHMARKS (Généré automatiquement) ===");
    println!("{}\n", "=".repeat(80));

    let mut results = Vec::new();
    let engines = ["tmplx", "sailfish", "maud", "askama", "markup"];

    for engine in engines {
        let path = format!(
            "../target/criterion/tmplx_vs_ecosystem_render_extreme/{}/new/estimates.json",
            engine
        );
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(mean) = json["mean"]["point_estimate"].as_f64() {
                    results.push((engine.to_string(), mean));
                }
            }
        }
    }

    if results.is_empty() {
        println!("Erreur : Impossible de lire les résultats de Criterion.");
        return;
    }

    // Sort by performance (fastest first)
    results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Tmplx should be the fastest (index 0)
    let baseline = results[0].1;

    println!(
        "| {:<10} | {:<20} | {:<15} | {:<25} |",
        "Moteur", "Temps par rendu", "Écart Brut", "Gain apporté"
    );
    println!("|:-----------|:---------------------|:----------------|:--------------------------|");

    for (i, (engine, time_ns)) in results.iter().enumerate() {
        let time_formatted = format!("~ {:.0} ns", time_ns);
        let diff_formatted = format!("+ {:.0} ns", time_ns - baseline);

        let gain_formatted = if i == 0 {
            "— (Le plus rapide)".to_string()
        } else {
            let gain = ((time_ns - baseline) / baseline) * 100.0;
            format!("+ {:.0} % plus rapide", gain)
        };

        println!(
            "| {:<10} | {:<20} | {:<15} | {:<25} |",
            engine, time_formatted, diff_formatted, gain_formatted
        );
    }
    println!("\n");
}

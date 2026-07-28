use askama::Template;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tmplx::models::User;
use tmplx::render_mockup;

// Askama Setup
#[derive(Template)]
#[template(path = "mockup.askama.html")]
struct AskamaExtremeTemplate<'a> {
    user: &'a str,
    html_inject_code: &'a str,
    is_admin: bool,
    user_list: &'a [User],
}

struct MockupViewData<'a> {
    pub user: &'a str,
    pub html_inject_code: &'a str,
    pub is_admin: bool,
    pub user_list: &'a [User],
}

fn bench_tmplx_vs_askama(c: &mut Criterion) {
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

    let mut group = c.benchmark_group("tmplx_vs_askama_render_extreme");

    group.bench_function("tmplx_render", |b| {
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

    group.bench_function("askama_render", |b| {
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

    group.finish();
}

criterion_group!(benches, bench_tmplx_vs_askama);
criterion_main!(benches);

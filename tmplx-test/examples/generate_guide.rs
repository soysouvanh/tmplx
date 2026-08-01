use tmplx_test::models::User;
use tmplx_test::render_ux_guide_en;
use tmplx_test::render_ux_guide_fr;

struct GuideViewData<'a> {
    pub raw_payload: &'a str,
    pub html_payload: &'a str,
    pub js_payload: &'a str,
    pub url_payload: &'a str,
    pub user: User,
    pub users_list: Vec<&'a str>,
}

fn main() {
    println!("Génération du guide interactif Tmplx en cours...");

    let data = GuideViewData {
        raw_payload: "<span class='color-accent' style='display:inline-block; padding:4px 8px; border:1px solid #f43f5e; border-radius:4px; font-weight:bold;'>[INJECTION DOM NON ÉCHAPPÉE]</span>",
        html_payload: "<script>alert('XSS bloquée, affichage sécurisé et converti en entités HTML');</script>",
        js_payload: "Chaîne injectée de manière sécurisée en JS avec \"guillemets\"",
        url_payload: "search term?&=",
        user: User {
            name: "Administrateur".to_string(),
            is_active: false,
        },
        users_list: vec![
            "Jean-Luc (client)",
            "Marie (super-admin du système)",
            "Sophie (audit externe)",
        ],
    };

    let mut output = String::with_capacity(4096);
    render_ux_guide_fr!(&mut output, &data);

    let output_path = concat!(env!("CARGO_MANIFEST_DIR"), "/ux_guide_fr.html");
    std::fs::write(output_path, &output)
        .expect("Erreur lors de l'écriture du fichier HTML du guide (FR).");

    let mut output_en = String::with_capacity(4096);
    render_ux_guide_en!(&mut output_en, &data);

    let output_path_en = concat!(env!("CARGO_MANIFEST_DIR"), "/ux_guide_en.html");
    std::fs::write(output_path_en, &output_en)
        .expect("Erreur lors de l'écriture du fichier HTML du guide (EN).");

    println!("{}", std::iter::repeat('=').take(70).collect::<String>());
    println!("Génération réussie de la vitrine complète UX/UI !");
    println!(
        "- Les fichiers ( {} et {} ) ont été mis à jour dans le dossier tmplx-test.",
        output_path, output_path_en
    );
    println!("- Ouvrez ce fichier dans votre navigateur web local pour admirer le résultat.");
    println!("{}", std::iter::repeat('=').take(70).collect::<String>());
}

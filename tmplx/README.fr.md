# Tmplx : Moteur de Templates HTML Haute-Performance

[English](README.md) | [Français](README.fr.md)

**Tmplx** est un moteur de templates HTML _Code-Gen-First_ pour Rust, garantissant par contrainte architecturale le **zéro allocation dynamique** (0 byte de heap pour le balisage statique) à l'exécution.

Plutôt que d'interpréter des chaînes de caractères au runtime de votre serveur HTTP, Tmplx décortique vos maquettes HTML lors de la phase de compilation (via `build.rs`). Il convertit chaque élément statique en appels système `output.push_str()` nus, et délègue toute la vérification de type `rustc` en amont.

Le résultat opérationnel : aucune lecture de fichier `.html` ne survient en production, et le typage structurel de vos interfaces est prouvé avant même le lancement du binaire.

---

## Installation & Configuration

Tmplx repose sur un pipeline de compilation très spécifique.

### 1. Activer Tmplx dans votre projet

Ajoutez la dépendance de production et le compilateur dédié dans votre fichier `Cargo.toml` :

```toml
[dependencies]
tmplx = "0.1"

[build-dependencies]
tmplx-compiler = "0.1"
```

_(Cargo, le gestionnaire de paquets de Rust, se chargera automatiquement de télécharger ces dépendances de façon sécurisée depuis **crates.io** — le registre officiel —, puis de les configurer de manière invisible au prochain `cargo build` !)_

### 2. L'orchestrateur de compilation (`build.rs`)

Tmplx compile vos pages **en même temps que** votre code Rust. \
Créez un fichier `build.rs` exactement à la racine de votre projet (à côté de `Cargo.toml`) et copiez/collez ce code prêt à l'emploi :

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    // 1. On dit à Cargo de relancer la compilation si un fichier HTML change
    println!("cargo:rerun-if-changed=templates");

    // 2. On récupère le dossier caché cible généré par Rust ($OUT_DIR)
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR manquant");
    let dest_path = PathBuf::from(out_dir).join("template_gen.rs");

    // 3. On lance la compilation magique : du pur HTML vers du code natif
    tmplx_compiler::build_workspace("templates", &dest_path);
}
```

### 3. Dossier des Templates HTML

Créez vos maquettes dans le dossier `templates/` (au même endroit que le dossier `src/`) avec un simple fichier :

- `index.html` (Votre structure)

---

## Syntaxe et Guide d'Utilisation interactif

Ici, vos pages HTML s'écrivent très simplement par de petites balises dynamiques sous format `{% ... %}`. Tmplx analyse cette grammaire pour injecter de l'intelligence contextuelle.

### 1. Afficher des Variables (Echappées & Brutes)

La règle primordiale est la sécurité des données utilisateur. Tmplx offre deux affichages :

- `{%%= view_data.user.name %}` **(Échappé / Securisé)** : À utiliser 99% du temps. Cette balise échappe le HTML dangereux pour vous protéger des failles XSS.
- `{%= view_data.html_inject_code %}` **(Brut / Dangereux)** : Affiche **exactement** le texte non altéré (à réserver aux morceaux de code HTML approuvés / statiques).

```html
<h1>Bienvenue, {%%= view_data.user.name %} !</h1>
```

### 2. Contrôle Spatial & Espaces Blancs (Troncature)

Si vous voulez éliminer les retours à la ligne ou espaces blancs générés par inadvertance autour des balises, ajoutez un petit trait d'union (`-`) :

- `{%-` : Supprime tout l'espace statique **avant** la balise.
- `-%}` : Supprime tout l'espace statique **après** la balise.

```html
<p>{%- if view_data.is_active -%} Connecté {%- endif -%}</p>
```

### 3. La Logique Conditionnelle (`if`, `else if`, `else`)

Basculez des éléments HTML via vos booléens avec clarté :

```html
{% if view_data.is_admin %}
<button>Panel Administrateur</button>
{% else if view_data.is_premium %}
<span class="badge">Premium</span>
{% else %}
<span>Utilisateur basique</span>
{% endif %}
```

_Astuce : Vous pouvez aussi tester une inversion avec le point d'exclamation (Ex: `{% if !view_data.is_active %}`)._

**Syntaxe alternative par accolades (`{ }`)** :
Pour les développeurs préférant le style Rust, il est également possible d'écrire vos blocs avec des accolades plutôt que des mots-clés (`endif`) :

```html
{% if view_data.is_admin { %}
<p>Admin</p>
{% } else if view_data.is_premium { %}
<p>Premium</p>
{% } else { %}
<p>Standard</p>
{% } %}
```

### 4. Parcourir et Boucler (`for`)

Affichez des listes HTML directement depuis des listes (`Vec<T>` ou slices) en Rust :

```html
<ul>
  {% for item in view_data.user_list %}
  <li>{%%= item.name %}</li>
  {% endfor %}
</ul>
```

_(De la même manière que pour le if, vous pouvez écrire `{% for item in view_data.user_list { %}` et clôturer avec `{% } %}` pour une ambiance purement Rust !)_

**Mode Avancé "Variables Magiques"** :
Dans vos boucles, Tmplx met passivement des variables puissantes à disposition :

- `loop_index` : L'index actuel de l'itération, démarrant à `1`.
- `loop_index0` : L'index actuel de l'itération, démarrant à `0` (très pratique pour des calculs d'interface ou du JS).
- `loop_length` : Taille totale de ma liste ? (ex: `15`)
- `loop_first` / `loop_last` : Booléens vrais sur le premier ou dernier élément.

```html
{% for msg in view_data.unread_messages %}
<div class="{% if loop_index % 2 == 0 %}pair{% else %}impair{% endif %}">
  Message {%= loop_index %} sur {%= loop_length %}
</div>
{% endfor %}
```

### 5. Affectation Locale (`let`)

Pré-calculez ou manipulez une variable côté serveur sans modifier votre logique principale de composant :

```html
{% let formatted_date = format!("{}-2024", view_data.month); %}
<span>Période : {%%= formatted_date %}</span>
```

### 6. Les Commentaires Invisibles (`{# ... #}`)

Laissez des notes sans polluer l'interface utilisateur ou le réseau.
Contrairement aux commentaires HTML (`<!-- -->`), les commentaires Tmplx ne font **absolument pas** partie du binaire final (ils comptent pour 0 octet) et disparaissent dès la compilation.

```html
{# FIXME: Ce bloc doit être refactorisé à la prochaine mise à jour #}
```

### 7. Architecture Modulaire (Héritage `extends` & `block`)

Gérez des "layouts" maîtres facilement (fini le copié/collé !) :

**Le fichier maître (`layout.html`) :**

```html
<!DOCTYPE html>
<html>
  <head>
    <title>Mon Site</title>
  </head>
  <body>
    <nav>Menu principal</nav>
    <main>{% block content %}{% endblock %}</main>
  </body>
</html>
```

**La page fille (`page.html`) :**

```html
{% extends "layout.html" %} {% block content %}
<h1>Je suis le contenu injecté au bon endroit !</h1>
{% endblock %}
```

### 8. Composants réutilisables (`include`)

Importez des sous-composants sans vous répéter !

```html
<div>
  <h1>Résumé Utilisateur</h1>
  {% include "partials/_user_card.html" %}
</div>
```

---

### Le Typage Canard (Duck Typing & Macros)

Grâce à sa nouvelle architecture orientée macros (`#[macro_export]`), Tmplx a éliminé le besoin de fichiers manifestes `.toml` ou de contrats explicites générés manuellement.

L'approche repose sur un "Duck Typing" compilé via un argument central appelé `view_data`, dont la structure est vérifiée entièrement et nativement par le compilateur **Rust** (`rustc`) à l'endroit de l'appel.
Il vous suffit d'insérer des variables préfixées par `view_data.` dans votre fichier `index.html` (exemple: `view_data.user.name` ou `view_data.unread_count`).

Si vous utilisez des champs ou appels inexistants sur la structure `view_data` envoyée à la macro lors de l'exécution, la compilation échouera immédiatement. Zéro bug latent en phase de production, tout est garanti syntaxiquement !

---

## Utilisation Côté Rust

### 4. Inclusion du Code Magique

Pour interagir avec vos templates, Rust a besoin de récupérer ce code nouvellement compilé. Dans votre code Rust principal (ex. `src/main.rs`), ajoutez le fameux macro d'insertion en haut du fichier :

```rust
// Récupère nos macros templates depuis le dossier caché cible de Cargo ($OUT_DIR)
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/template_gen.rs"));
}
```

Voici à quel point appeler votre page générée depuis vos routes devient simple via le système de Macro généré :

```rust
// On va invoquer notre macro `render_dashboard` nouvellement créé !
use generated::TMPLX_STATIC_SIZE_RENDER_DASHBOARD;
use crate::render_dashboard; // La macro est exportée globalement par #[macro_export]

fn show_page() -> String {
    // Structure locale jetable ou globale (Le Duck Typing s'en accommode)
    struct DashboardViewData {
        user: String,
        unread_count: usize,
    }

    // On prépare notre Aggrégat de vue strict
    let my_data = DashboardViewData {
        user: "Vincent".to_string(),
        unread_count: 12,
    };

    // Astuce Zéro-Allocation: Nous pré-allouons l'espace exact requis au lieu de faire un "new()" à l'aveugle !
    let mut html_output = String::with_capacity(TMPLX_STATIC_SIZE_RENDER_DASHBOARD + 100);

    // Le miracle de la compilation macro
    render_dashboard!(&mut html_output, &my_data);

    html_output
}
```

Et c'est tout ! L'API offre une traçabilité idéale pour vos applications Backend d'une vitesse redoutable.
Bon code !

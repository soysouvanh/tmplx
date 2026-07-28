# Tmplx workspace

[English](README.md) | [Français](README.fr.md)

Un moteur de templates HTML _Code-Gen-First_ pour Rust, garantissant **zéro allocation dynamique** à l'exécution et une validation stricte dès la compilation.

## Philosophie

Contrairement aux moteurs traditionnels (Tera, Askama, Handlebars) qui chargent les templates à l'exécution ou interpolent des chaînes, **Tmplx compile vos maquettes HTML en fonctions Rust natives de très bas niveau (`output.push_str`)**.

- **Zéro allocation du balisage** : Les parties statiques sont encodées en dur dans le binaire Rust.
- **Zéro parsing au runtime** : La validité structurelle est garantie par `build.rs`.
- **Typage absolu** : Les variables injectées (`view_data`) sont vérifiées sémantiquement au strict moment de la compilation par un système de macros (Duck-Typing).

## Architecture du workspace

Voici comment s'organise l'écosystème Tmplx :

```text
tmplx-workspace/
├── tmplx/                  # Le moteur principal (publié sur crates.io)
│   ├── build_logic/        # Logique de compilation (parsing, tokens, génération)
│   ├── src/                # Code d'exécution (macro, sécurité, duck-typing)
│   └── templates/          # Templates internes (mockups pour intégration système)
│
└── tmplx-test/             # Le projet vitrine (Documentation vivante)
    ├── benches/            # Algorithmes de tests de charge et comparatif de performance
    ├── src/                # Exemples exécutables et tests d'intégration complets
    └── templates/          # Cas d'usages de templates (héritage, logique, locales)
        └── partials/       # Démonstration de composants web réutilisables
```

### Exécuter la suite de tests (Documentation vivante)

Pour vérifier la fiabilité absolue du moteur et explorer des exemples métiers concrets (architecture modulaire, assignations locales, limites de troncature), vous pouvez exécuter toutes les validations d'intégration :

```bash
cargo test -p tmplx-test
```

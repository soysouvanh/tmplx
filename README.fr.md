# Tmplx Workspace

[English](README.md) | [Français](README.fr.md)

Un moteur de templates HTML _Code-Gen-First_ pour Rust, garantissant **zéro allocation dynamique** à l'exécution et une validation stricte dès la compilation.

## Philosophie

Contrairement aux moteurs traditionnels (Tera, Askama, Handlebars) qui chargent les templates à l'exécution ou interpolent des chaînes, **Tmplx compile vos maquettes HTML en fonctions Rust natives de très bas niveau (`output.push_str`)**.

- **Zéro allocation du balisage** : Les parties statiques sont encodées en dur dans le binaire Rust.
- **Zéro parsing au runtime** : La validité structurelle est garantie par `build.rs`.
- **Typage absolu** : Les variables injectées (`view_data`) sont vérifiées sémantiquement au strict moment de la compilation par un système de macros (Duck-Typing).

## Architecture du Workspace

- `tmplx/` : Crate principal contenant la logique du moteur de build (`build_logic/`) et les traits génériques.
- `tmplx-test/` : Crate de tests d'intégration, validant le pipeline complet de generation avec `templates/`.

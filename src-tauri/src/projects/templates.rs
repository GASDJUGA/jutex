//! Project templates bundled from the repository's `templates/` folder.
//!
//! Each template is a folder with a `template.json` descriptor plus the files that are copied
//! into a new project.

use crate::error::{AppError, AppResult};
use crate::projects::meta::Compiler;
use crate::util::fs::read_json;
use crate::util::paths::validate_name;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const DESCRIPTOR: &str = "template.json";

/// A text of a template in each language (by code of [`crate::lang`]).
pub type Localized = std::collections::BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    order: u32,
    #[serde(default)]
    default: bool,
    name: Localized,
    description: Localized,
    main_file: String,
    compiler: Compiler,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInfo {
    pub id: String,
    pub name: Localized,
    pub description: Localized,
    pub main_file: String,
    pub compiler: Compiler,
    pub default: bool,
    /// The languages it comes in (its folders `<id>/<code>/`), in the table's order.
    pub languages: Vec<String>,
}

/// Lists the templates under `root`, sorted by their `order`.
pub fn list(root: &Path) -> AppResult<Vec<TemplateInfo>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| AppError::io(root, e))? {
        let entry = entry.map_err(|e| AppError::io(root, e))?;
        let dir = entry.path();
        let Some(descriptor) = read_json::<Descriptor>(&dir.join(DESCRIPTOR))? else {
            continue;
        };
        let id = entry.file_name().to_string_lossy().into_owned();
        let languages = crate::lang::all()
            .iter()
            .filter(|l| dir.join(&l.code).is_dir())
            .map(|l| l.code.clone())
            .collect();
        found.push((
            descriptor.order,
            TemplateInfo {
                id,
                name: descriptor.name,
                description: descriptor.description,
                main_file: descriptor.main_file,
                compiler: descriptor.compiler,
                default: descriptor.default,
                languages,
            },
        ));
    }
    found.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)));
    Ok(found.into_iter().map(|(_, t)| t).collect())
}

/// Finds template `id` under `root`.
pub fn find(root: &Path, id: &str) -> AppResult<TemplateInfo> {
    validate_name(id).map_err(|_| AppError::UnknownTemplate(id.to_owned()))?;
    list(root)?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::UnknownTemplate(id.to_owned()))
}

/// Copies the files of template `id` in `language` (its folder `<id>/<language>/`) into
/// `dest`, which must already exist.
pub fn copy_into(root: &Path, id: &str, language: &str, dest: &Path) -> AppResult<TemplateInfo> {
    let info = find(root, id)?;
    // Only one of its languages (never a path).
    if !info.languages.iter().any(|l| l == language) {
        return Err(AppError::UnknownTemplate(format!("{id} ({language})")));
    }
    let src = root.join(&info.id).join(language);
    for entry in WalkDir::new(&src).min_depth(1).follow_links(false) {
        let entry = entry.map_err(|e| AppError::Other(e.to_string()))?;
        let rel = entry
            .path()
            .strip_prefix(&src)
            .expect("walkdir yields paths under src");
        if rel == Path::new(DESCRIPTOR) {
            continue;
        }
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| AppError::io(&target, e))?;
        } else if entry.file_type().is_file() {
            fs::copy(entry.path(), &target).map_err(|e| AppError::io(&target, e))?;
        }
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn repo_templates() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates")
    }

    /// The shape of a folder: for each file, its depth and extension (names may be
    /// translated: `capitulos/` and `chapters/`).
    fn structure(dir: &Path) -> Vec<(usize, String)> {
        let mut out: Vec<_> = WalkDir::new(dir)
            .min_depth(1)
            .into_iter()
            .map(|e| e.unwrap())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let ext = e
                    .path()
                    .extension()
                    .map(|x| x.to_string_lossy().into_owned());
                (e.depth(), ext.unwrap_or_default())
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn las_plantillas_del_repo_son_validas() {
        let templates = list(&repo_templates()).unwrap();
        let ids: Vec<_> = templates.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, ["articulo", "apuntes", "tarea", "beamer"]);

        let defaults: Vec<_> = templates.iter().filter(|t| t.default).collect();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].id, "articulo");
        assert_eq!(defaults[0].compiler, Compiler::Xelatex);

        for t in &templates {
            for l in crate::lang::all() {
                assert!(
                    t.name.get(&l.code).is_some_and(|n| !n.is_empty())
                        && t.description.get(&l.code).is_some_and(|d| !d.is_empty()),
                    "{} {}",
                    t.id,
                    l.code
                );
            }
            assert!(t.languages.iter().any(|l| l == "es"), "{}", t.id);
            assert!(t.languages.iter().any(|l| l == "en"), "{}", t.id);
        }
    }

    #[test]
    fn cada_plantilla_tiene_la_misma_estructura_en_cada_idioma() {
        for t in list(&repo_templates()).unwrap() {
            let dir = repo_templates().join(&t.id);
            let es = structure(&dir.join("es"));
            assert!(!es.is_empty(), "{}", t.id);
            for code in &t.languages {
                assert_eq!(structure(&dir.join(code)), es, "{} {code}", t.id);
            }
        }
    }

    #[test]
    fn cada_main_carga_babel_con_su_idioma() {
        for t in list(&repo_templates()).unwrap() {
            for code in &t.languages {
                let main = repo_templates().join(&t.id).join(code).join(&t.main_file);
                let source = fs::read_to_string(&main).unwrap_or_else(|_| panic!("{main:?}"));
                assert!(source.contains("\\documentclass"), "{main:?}");
                let babel = &crate::lang::get(code).unwrap().babel;
                assert!(source.contains(&format!("[{babel}]{{babel}}")), "{main:?}");
            }
        }
    }

    #[test]
    fn copia_los_archivos_del_idioma_sin_el_descriptor() {
        let dest = TempDir::new().unwrap();
        let info = copy_into(&repo_templates(), "apuntes", "es", dest.path()).unwrap();
        assert_eq!(info.main_file, "main.tex");
        assert!(dest.path().join("main.tex").is_file());
        assert!(dest.path().join("capitulos/01-introduccion.tex").is_file());
        assert!(!dest.path().join(DESCRIPTOR).exists());

        let dest = TempDir::new().unwrap();
        copy_into(&repo_templates(), "apuntes", "en", dest.path()).unwrap();
        let main = fs::read_to_string(dest.path().join("main.tex")).unwrap();
        assert!(main.contains("[english]{babel}"));
        assert!(dest.path().join("chapters/01-introduction.tex").is_file());
    }

    #[test]
    fn rechaza_ids_o_idiomas_desconocidos_o_peligrosos() {
        let dest = TempDir::new().unwrap();
        for (id, lang) in [
            ("no-existe", "es"),
            ("../src", "es"),
            ("", "es"),
            (".", "es"),
            ("apuntes", "ru"),
            ("apuntes", "../tarea"),
            ("apuntes", ""),
        ] {
            assert!(
                matches!(
                    copy_into(&repo_templates(), id, lang, dest.path()),
                    Err(AppError::UnknownTemplate(_))
                ),
                "{id} {lang}"
            );
        }
    }
}

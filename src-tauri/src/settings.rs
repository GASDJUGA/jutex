//! `~/.config/jutex/settings.json` (5.8). Read leniently: a missing, unknown or invalid
//! value falls back to its default (a hand-edited file never stops the app), and keys this
//! version does not know are kept when saving.

use crate::backup::Identity;
use crate::error::{AppError, AppResult};
use crate::lang;
use crate::projects::meta::Compiler;
use crate::tex::backends::TexChoice;
use crate::util::fs::{read_json, write_json};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const MIN_FONT_SIZE: u32 = 8;
pub const MAX_FONT_SIZE: u32 = 32;
pub const MAX_HISTORY_MINUTES: u32 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditorTheme {
    #[default]
    Light,
    Dark,
}

/// Interface colors (`data-ui-theme`, styles/index.css).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UiTheme {
    #[default]
    Indigo,
    Graphite,
    Forest,
    Ocean,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Keymap {
    #[default]
    Default,
    Vim,
    Emacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IndentMode {
    #[default]
    Environments,
    Full,
    None,
}

/// `locale` that follows the system's language.
pub const SYSTEM: &str = "system";
/// `documentLanguage` that follows the interface's language.
pub const INTERFACE: &str = "interface";

/// Every setting, with its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Interface language: `system` (the system's, English if JuTeX does not have it) or a
    /// code of [`crate::lang`].
    pub locale: String,
    /// Editor font family; empty = the default monospace fonts.
    pub editor_font_family: String,
    pub editor_font_size: u32,
    pub ui_theme: UiTheme,
    pub editor_theme: EditorTheme,
    pub keymap: Keymap,
    pub indent_mode: IndentMode,
    pub live_check: bool,
    pub math_preview: bool,
    pub math_preview_hover: bool,
    pub pdf_dark_mode: bool,
    /// Compiler of new blank projects.
    pub default_compiler: Compiler,
    /// Language of new documents: `interface` (the interface's) or a code of
    /// [`crate::lang`]. Their spelling is checked in it.
    pub document_language: String,
    /// New projects start with spelling checked (in their language) or off.
    pub new_project_spellcheck: bool,
    /// Minutes between automatic versions of the history (0: only manual and after compiling).
    pub history_minutes: u32,
    /// Author of the Git backups when `git config` has none (asked once; empty = not set).
    pub git_name: String,
    pub git_email: String,
    /// Where new projects are created (default `~/JuTeX`).
    pub projects_dir: Option<PathBuf>,
    pub tex_distribution: TexChoice,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: SYSTEM.into(),
            editor_font_family: String::new(),
            editor_font_size: 14,
            ui_theme: UiTheme::default(),
            editor_theme: EditorTheme::default(),
            keymap: Keymap::default(),
            indent_mode: IndentMode::default(),
            live_check: true,
            math_preview: true,
            math_preview_hover: false,
            pdf_dark_mode: false,
            default_compiler: Compiler::default(),
            document_language: INTERFACE.into(),
            new_project_spellcheck: true,
            history_minutes: 10,
            git_name: String::new(),
            git_email: String::new(),
            projects_dir: None,
            tex_distribution: TexChoice::default(),
        }
    }
}

/// Keys the webview may change with `update`; the others have their own commands
/// (the projects folder is chosen in a native dialog, the TeX with `set_tex_distribution`).
const EDITABLE: [&str; 17] = [
    "locale",
    "editorFontFamily",
    "editorFontSize",
    "uiTheme",
    "editorTheme",
    "keymap",
    "indentMode",
    "liveCheck",
    "mathPreview",
    "mathPreviewHover",
    "pdfDarkMode",
    "defaultCompiler",
    "documentLanguage",
    "newProjectSpellcheck",
    "historyMinutes",
    "gitName",
    "gitEmail",
];

fn valid(key: &str, settings: &Settings) -> bool {
    match key {
        "locale" => settings.locale == SYSTEM || lang::get(&settings.locale).is_some(),
        "documentLanguage" => {
            settings.document_language == INTERFACE
                || lang::get(&settings.document_language).is_some()
        }
        "editorFontFamily" => {
            settings.editor_font_family.chars().count() <= 100
                && !settings
                    .editor_font_family
                    .chars()
                    .any(|c| c.is_control() || matches!(c, ';' | '{' | '}' | '"' | '\\'))
        }
        "editorFontSize" => (MIN_FONT_SIZE..=MAX_FONT_SIZE).contains(&settings.editor_font_size),
        "historyMinutes" => settings.history_minutes <= MAX_HISTORY_MINUTES,
        // Empty (not set) or what `Identity::clean` accepts, as typed.
        "gitName" | "gitEmail" => {
            let value = if key == "gitName" {
                &settings.git_name
            } else {
                &settings.git_email
            };
            let probe = Identity {
                name: if key == "gitName" {
                    value.clone()
                } else {
                    "x".into()
                },
                email: if key == "gitEmail" {
                    value.clone()
                } else {
                    "x@y".into()
                },
            };
            value.is_empty() || probe.clean().is_ok_and(|c| c == probe)
        }
        "projectsDir" => settings
            .projects_dir
            .as_ref()
            .is_none_or(|p| p.is_absolute()),
        _ => true,
    }
}

impl Settings {
    /// The interface language, with `system` resolved to `system` (the system's language,
    /// already one of JuTeX's).
    pub fn effective_locale(&self, system: &str) -> String {
        if self.locale == SYSTEM {
            system.to_owned()
        } else {
            self.locale.clone()
        }
    }

    /// The language of new documents, with `interface` resolved.
    pub fn effective_document_language(&self, system: &str) -> String {
        if self.document_language == INTERFACE {
            self.effective_locale(system)
        } else {
            self.document_language.clone()
        }
    }

    /// The backup author JuTeX asked for, if set.
    pub fn git_identity(&self) -> Option<Identity> {
        (!self.git_name.is_empty() && !self.git_email.is_empty()).then(|| Identity {
            name: self.git_name.clone(),
            email: self.git_email.clone(),
        })
    }

    /// Settings from a JSON object, field by field: what is missing or invalid gets its
    /// default.
    pub fn from_map(map: &Map<String, Value>) -> Self {
        let defaults = Self::default();
        let mut out = serde_json::to_value(&defaults)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        for (key, value) in out.iter_mut() {
            let Some(stored) = map.get(key) else { continue };
            // Try the stored value alone on top of the defaults.
            let mut candidate = serde_json::to_value(&defaults).unwrap_or_default();
            candidate[key.as_str()] = stored.clone();
            if let Ok(parsed) = serde_json::from_value::<Settings>(candidate)
                && valid(key, &parsed)
            {
                *value = stored.clone();
            }
        }
        serde_json::from_value(Value::Object(out)).unwrap_or(defaults)
    }

    /// The setting `key` as JSON.
    fn get(&self, key: &str) -> Value {
        serde_json::to_value(self)
            .ok()
            .and_then(|v| v.get(key).cloned())
            .unwrap_or(Value::Null)
    }
}

/// The stored file as a JSON object (empty if it does not exist).
fn read_map(path: &Path) -> AppResult<Map<String, Value>> {
    Ok(read_json::<Map<String, Value>>(path)?.unwrap_or_default())
}

pub fn load(path: &Path) -> AppResult<Settings> {
    Ok(Settings::from_map(&read_map(path)?))
}

/// Applies `changes` (only editable keys, each one valid) and saves; unknown keys of the
/// file are kept. Returns the new settings.
pub fn update(path: &Path, changes: &Map<String, Value>) -> AppResult<Settings> {
    let mut map = read_map(path)?;
    for (key, value) in changes {
        if !EDITABLE.contains(&key.as_str()) {
            return Err(AppError::Forbidden(format!("ajuste «{key}»")));
        }
        map.insert(key.clone(), value.clone());
    }
    let settings = Settings::from_map(&map);
    for (key, value) in changes {
        if &settings.get(key) != value {
            return Err(AppError::Forbidden(format!("valor no válido para «{key}»")));
        }
    }
    write_json(path, &map)?;
    Ok(settings)
}

/// On each start, before reading the settings: if no language was ever stored, stores one.
/// A JuTeX that was already in use (there is a `settings.json` or a `projects.json`) showed
/// Spanish until 2.1.0, so it stays in Spanish, and its old `defaultSpellLang` becomes the
/// language of new documents; a new installation follows the system (`system`). Writing it
/// the first time keeps a new installation from looking like an old one on its next start.
pub fn settle_first_run(settings_file: &Path, projects_file: &Path) -> AppResult<()> {
    let mut map = read_map(settings_file)?;
    if map.contains_key("locale") {
        return Ok(());
    }
    let previous = settings_file.exists() || projects_file.exists();
    if previous {
        map.insert("locale".into(), Value::from("es"));
        match map.get("defaultSpellLang").and_then(Value::as_str) {
            Some("off") => {
                map.insert("newProjectSpellcheck".into(), Value::from(false));
            }
            Some(code) if lang::get(code).is_some() => {
                map.insert("documentLanguage".into(), Value::from(code));
            }
            _ => {}
        }
    } else {
        map.insert("locale".into(), Value::from(SYSTEM));
    }
    write_json(settings_file, &map)
}

/// Sets one key that has its own command (`texDistribution`, `projectsDir`), keeping the
/// rest.
pub fn set(path: &Path, key: &str, value: Value) -> AppResult<Settings> {
    let mut map = read_map(path)?;
    map.insert(key.to_owned(), value);
    write_json(path, &map)?;
    Ok(Settings::from_map(&map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    fn changes(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    /// A config folder with `settings.json` (as `stored`, or none) and maybe `projects.json`.
    fn config(stored: Option<Value>, projects: bool) -> (TempDir, PathBuf, PathBuf) {
        let dir = TempDir::new().unwrap();
        let settings = dir.path().join("settings.json");
        let registry = dir.path().join("projects.json");
        if let Some(value) = stored {
            std::fs::write(&settings, value.to_string()).unwrap();
        }
        if projects {
            std::fs::write(&registry, "[]").unwrap();
        }
        (dir, settings, registry)
    }

    #[test]
    fn instalacion_nueva_sigue_al_sistema_tambien_al_segundo_arranque() {
        let (_dir, settings, registry) = config(None, false);
        settle_first_run(&settings, &registry).unwrap();
        assert_eq!(load(&settings).unwrap().locale, "system");
        // Next start: both files exist now, and nobody chose a language.
        std::fs::write(&registry, "[]").unwrap();
        settle_first_run(&settings, &registry).unwrap();
        let s = load(&settings).unwrap();
        assert_eq!(s.locale, "system");
        assert_eq!(s.effective_locale("fr"), "fr");
        assert_eq!(s.effective_document_language("fr"), "fr");
    }

    #[test]
    fn usuario_actual_sin_locale_queda_en_espanol() {
        // The user's real settings.json: neither locale nor defaultSpellLang.
        let stored =
            json!({"editorFontFamily": "", "keymap": "default", "texDistribution": "system"});
        let (_dir, settings, registry) = config(Some(stored), false);
        settle_first_run(&settings, &registry).unwrap();
        let s = load(&settings).unwrap();
        assert_eq!(s.locale, "es");
        assert_eq!(s.effective_locale("en"), "es");
        assert_eq!(s.effective_document_language("en"), "es");
        assert!(s.new_project_spellcheck);
        // What was there is kept.
        assert_eq!(s.tex_distribution, TexChoice::System);
    }

    #[test]
    fn solo_projects_json_tambien_es_instalacion_anterior() {
        let (_dir, settings, registry) = config(None, true);
        settle_first_run(&settings, &registry).unwrap();
        assert_eq!(load(&settings).unwrap().locale, "es");
    }

    #[test]
    fn default_spell_lang_viejo_pasa_a_idioma_de_documentos() {
        let (_dir, settings, registry) = config(Some(json!({"defaultSpellLang": "en"})), false);
        settle_first_run(&settings, &registry).unwrap();
        let s = load(&settings).unwrap();
        assert_eq!(
            (s.locale.as_str(), s.document_language.as_str()),
            ("es", "en")
        );
        assert_eq!(s.effective_document_language("de"), "en");

        let (_dir, settings, registry) = config(Some(json!({"defaultSpellLang": "off"})), false);
        settle_first_run(&settings, &registry).unwrap();
        let s = load(&settings).unwrap();
        assert!(!s.new_project_spellcheck);
        assert_eq!(s.effective_document_language("de"), "es");
    }

    #[test]
    fn un_locale_guardado_no_se_toca() {
        let (_dir, settings, registry) = config(
            Some(json!({"locale": "de", "defaultSpellLang": "en"})),
            true,
        );
        settle_first_run(&settings, &registry).unwrap();
        let s = load(&settings).unwrap();
        assert_eq!(
            (s.locale.as_str(), s.document_language.as_str()),
            ("de", "interface")
        );
    }

    #[test]
    fn valores_de_idioma_validos() {
        let (_dir, settings, _registry) = config(None, false);
        for good in [
            json!({ "locale": "system" }),
            json!({ "locale": "pt" }),
            json!({ "documentLanguage": "interface" }),
            json!({ "documentLanguage": "it" }),
            json!({ "newProjectSpellcheck": false }),
        ] {
            update(&settings, &changes(good.clone())).unwrap_or_else(|e| panic!("{good}: {e}"));
        }
        let s = load(&settings).unwrap();
        assert_eq!(
            (s.locale.as_str(), s.document_language.as_str()),
            ("pt", "it")
        );
    }

    #[test]
    fn idiomas_efectivos() {
        let s = Settings {
            locale: "system".into(),
            ..Settings::default()
        };
        assert_eq!(s.effective_locale("fr"), "fr");
        assert_eq!(s.effective_document_language("fr"), "fr");
        let s = Settings {
            locale: "de".into(),
            document_language: "it".into(),
            ..Settings::default()
        };
        assert_eq!(s.effective_locale("fr"), "de");
        assert_eq!(s.effective_document_language("fr"), "it");
    }

    #[test]
    fn acepta_texdistribution_libretex() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{ "texDistribution": "libretex" }"#).unwrap();
        assert_eq!(load(&path).unwrap().tex_distribution, TexChoice::Jutex);
        assert_eq!(
            serde_json::to_value(TexChoice::Jutex).unwrap(),
            json!("jutex")
        );
    }

    #[test]
    fn sin_archivo_todo_por_defecto() {
        let dir = TempDir::new().unwrap();
        assert_eq!(
            load(&dir.path().join("settings.json")).unwrap(),
            Settings::default()
        );
    }

    #[test]
    fn un_valor_invalido_no_tumba_los_demas() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{ "keymap": "vim", "editorFontSize": 400, "editorTheme": "neón",
                 "uiTheme": "rosa", "liveCheck": "sí", "texDistribution": "system", "futuro": 1 }"#,
        )
        .unwrap();
        let s = load(&path).unwrap();
        assert_eq!(s.keymap, Keymap::Vim);
        assert_eq!(s.editor_font_size, 14);
        assert_eq!(s.editor_theme, EditorTheme::Light);
        assert_eq!(s.ui_theme, UiTheme::Indigo);
        assert!(s.live_check);
        assert_eq!(s.tex_distribution, TexChoice::System);
    }

    #[test]
    fn guarda_cambios_validos_y_conserva_las_claves_desconocidas() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config/settings.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, r#"{ "futuro": 1, "texDistribution": "jutex" }"#).unwrap();
        let s = update(
            &path,
            &changes(
                json!({ "editorTheme": "dark", "editorFontSize": 16, "keymap": "emacs",
                             "defaultCompiler": "lualatex", "locale": "en", "uiTheme": "forest" }),
            ),
        )
        .unwrap();
        assert_eq!(s.ui_theme, UiTheme::Forest);
        assert_eq!(
            (
                s.editor_theme,
                s.editor_font_size,
                s.keymap,
                s.default_compiler,
                s.locale.as_str()
            ),
            (
                EditorTheme::Dark,
                16,
                Keymap::Emacs,
                Compiler::Lualatex,
                "en"
            )
        );
        let stored: Map<String, Value> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(stored["futuro"], json!(1));
        assert_eq!(stored["texDistribution"], json!("jutex"));
        // A restart reads the same.
        assert_eq!(load(&path).unwrap(), s);
    }

    #[test]
    fn rechaza_valores_invalidos_y_claves_ajenas() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        for bad in [
            json!({ "editorFontSize": 2 }),
            json!({ "keymap": "nano" }),
            json!({ "locale": "klingon" }),
            json!({ "documentLanguage": "xx" }),
            json!({ "defaultSpellLang": "en" }),
            json!({ "editorFontFamily": "x; background: red" }),
            json!({ "projectsDir": "/tmp" }),
            json!({ "texDistribution": "system" }),
            json!({ "liveCheck": "no" }),
            json!({ "historyMinutes": 500 }),
            json!({ "gitEmail": "sin-arroba" }),
            json!({ "gitName": "<x>" }),
            json!({ "gitEmail": " a@b.c" }),
        ] {
            assert!(
                matches!(
                    update(&path, &changes(bad.clone())),
                    Err(AppError::Forbidden(_))
                ),
                "{bad}"
            );
        }
        // Nothing was written.
        assert!(!path.exists());
    }

    #[test]
    fn una_carpeta_de_proyectos_relativa_se_ignora() {
        let mut map = Map::new();
        map.insert("projectsDir".into(), json!("relativa"));
        assert_eq!(Settings::from_map(&map).projects_dir, None);
        let absolute = crate::util::paths::absolute("/home/x/Proyectos");
        map.insert("projectsDir".into(), json!(absolute));
        assert_eq!(
            Settings::from_map(&map).projects_dir,
            Some(PathBuf::from(absolute))
        );
    }

    #[test]
    fn la_distribucion_tex_se_guarda_aparte() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json");
        update(&path, &changes(json!({ "keymap": "vim" }))).unwrap();
        let s = set(&path, "texDistribution", json!("system")).unwrap();
        assert_eq!(
            (s.keymap, s.tex_distribution),
            (Keymap::Vim, TexChoice::System)
        );
    }
}

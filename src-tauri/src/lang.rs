//! The languages JuTeX speaks: interface, new documents, spelling and TeX. One table
//! (`src/lib/languages.json`) shared with the frontend.

use serde::Deserialize;
use std::sync::LazyLock;

/// One language of the table.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    /// `es`, `en`, `pt` (Brazil), `fr`, `de`, `it`.
    pub code: String,
    /// Its own name («Deutsch»).
    pub name: String,
    /// Option of babel for new documents.
    pub babel: String,
    /// Names of babel and polyglossia that mean this language (lowercase).
    pub aliases: Vec<String>,
    /// TeX Live packages with its babel support and hyphenation.
    pub texlive: Vec<String>,
    /// The pacman group of Arch with them.
    pub pacman: String,
    /// Primary language of a Windows `LANGID`.
    pub windows_lang: u16,
    /// The LaTeX manual of «Ayuda» (the English one if there is none).
    pub wikibook: String,
    /// First comment of a blank project.
    pub blank_comment: String,
}

static LANGUAGES: LazyLock<Vec<Language>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../src/lib/languages.json"))
        .expect("src/lib/languages.json is valid")
});

/// Every language, in the table's order.
pub fn all() -> &'static [Language] {
    &LANGUAGES
}

/// The language with this code.
pub fn get(code: &str) -> Option<&'static Language> {
    all().iter().find(|l| l.code == code)
}

/// The language of a system locale tag (`pt_BR.UTF-8`, `de-AT`, `fr`), by its primary part;
/// `None` for `C`, `POSIX` or a language JuTeX does not have.
pub fn from_locale_tag(tag: &str) -> Option<&'static str> {
    let primary = tag
        .split(['_', '-', '.', '@'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    get(&primary).map(|l| l.code.as_str())
}

/// The first supported language of `candidates` (system tags, most preferred first), or
/// English.
pub fn resolve(candidates: &[String]) -> &'static str {
    candidates
        .iter()
        .find_map(|tag| from_locale_tag(tag))
        .unwrap_or("en")
}

/// The language a babel or polyglossia name means (`ngerman` → `de`).
pub fn from_tex_name(name: &str) -> Option<&'static str> {
    let name = name.trim().to_ascii_lowercase();
    all()
        .iter()
        .find(|l| l.aliases.contains(&name))
        .map(|l| l.code.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_tabla_tiene_los_seis_en_orden() {
        let codes: Vec<_> = all().iter().map(|l| l.code.as_str()).collect();
        assert_eq!(codes, ["es", "en", "pt", "fr", "de", "it"]);
        assert_eq!(get("de").unwrap().babel, "ngerman");
        assert!(get("ru").is_none());
    }

    #[test]
    fn etiquetas_del_sistema() {
        assert_eq!(from_locale_tag("pt_PT.UTF-8"), Some("pt"));
        assert_eq!(from_locale_tag("de-AT"), Some("de"));
        assert_eq!(from_locale_tag("es_CL.UTF-8@euro"), Some("es"));
        assert_eq!(from_locale_tag("C.UTF-8"), None);
        assert_eq!(from_locale_tag("POSIX"), None);
        assert_eq!(from_locale_tag("ru_RU.UTF-8"), None);
        assert_eq!(from_locale_tag(""), None);
    }

    #[test]
    fn sin_idioma_soportado_es_ingles() {
        assert_eq!(resolve(&["ru".into(), "C".into()]), "en");
        assert_eq!(resolve(&["ja".into(), "fr_CA".into()]), "fr");
        assert_eq!(resolve(&[]), "en");
    }

    #[test]
    fn nombres_de_tex() {
        assert_eq!(from_tex_name("NGerman"), Some("de"));
        assert_eq!(from_tex_name("brazil"), Some("pt"));
        assert_eq!(from_tex_name("USenglish"), Some("en"));
        assert_eq!(from_tex_name("russian"), None);
    }
}

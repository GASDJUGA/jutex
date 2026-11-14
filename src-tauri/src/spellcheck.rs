//! Spell checking with the bundled Hunspell dictionaries (crate `spellbook`), one per
//! language of [`crate::lang`], loaded the first time it is needed. The personal dictionary
//! (`spell.rs`) counts as correct in every language.

use crate::error::{AppError, AppResult};
use spellbook::Dictionary;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Most suggestions sent to the interface (it orders them and shows the closest ones).
pub const MAX_SUGGESTIONS: usize = 20;

/// Reads a Hunspell dictionary (`.aff` and `.dic`).
pub fn load(aff: &Path, dic: &Path) -> AppResult<Dictionary> {
    let read = |path: &Path| std::fs::read_to_string(path).map_err(|e| AppError::io(path, e));
    Dictionary::new(&read(aff)?, &read(dic)?)
        .map_err(|e| AppError::Other(format!("diccionario {}: {e}", aff.display())))
}

/// Whether `word` is in the personal dictionary. As Hunspell does with its own words, one
/// added in lowercase also counts capitalized or in capitals; one with capitals, only so.
fn personal_has(personal: &HashSet<String>, word: &str) -> bool {
    if personal.contains(word) {
        return true;
    }
    let lower = word.to_lowercase();
    if lower == word || !personal.contains(&lower) {
        return false;
    }
    let mut chars = word.chars();
    let first_upper = chars.next().is_some_and(char::is_uppercase);
    let rest: String = chars.collect();
    word.to_uppercase() == word || (first_upper && rest.to_lowercase() == rest)
}

/// The words of `words` that are not correct.
pub fn wrong(dictionary: &Dictionary, personal: &HashSet<String>, words: &[String]) -> Vec<String> {
    words
        .iter()
        .filter(|w| !personal_has(personal, w) && !dictionary.check(w))
        .cloned()
        .collect()
}

/// Corrections for `word` (unordered: the interface ranks them by typing distance).
pub fn suggest(dictionary: &Dictionary, word: &str) -> Vec<String> {
    let mut out = Vec::new();
    dictionary.suggest(word, &mut out);
    out.truncate(MAX_SUGGESTIONS);
    out
}

/// The dictionaries of the languages in use, loaded the first time (`<dir>/<code>.aff` and
/// `.dic`, bundled with the app).
pub struct Checkers {
    dir: PathBuf,
    loaded: Mutex<HashMap<String, Arc<Dictionary>>>,
}

impl Checkers {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            loaded: Mutex::new(HashMap::new()),
        }
    }

    /// The dictionary of language `code` (only a language of the table: never a path).
    pub fn get(&self, code: &str) -> AppResult<Arc<Dictionary>> {
        if crate::lang::get(code).is_none() {
            return Err(AppError::Forbidden(format!("idioma «{code}»")));
        }
        let mut loaded = self.loaded.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(d) = loaded.get(code) {
            return Ok(d.clone());
        }
        let dictionary = Arc::new(load(
            &self.dir.join(format!("{code}.aff")),
            &self.dir.join(format!("{code}.dic")),
        )?);
        loaded.insert(code.to_owned(), dictionary.clone());
        Ok(dictionary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dictionaries as npm installed them (the app bundles them as resources).
    fn files(code: &str) -> (PathBuf, PathBuf) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../node_modules")
            .join(format!("dictionary-{code}"));
        (dir.join("index.aff"), dir.join("index.dic"))
    }

    fn checker(code: &str) -> Dictionary {
        let (aff, dic) = files(code);
        load(&aff, &dic).unwrap()
    }

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn cada_idioma_acepta_y_marca() {
        let none = HashSet::new();
        for (code, good, bad) in [
            ("es", "eléctrico", "ecuacion"),
            ("en", "electric", "teh"),
            ("pt", "ação", "equasão"),
            ("fr", "maison", "maisonn"),
            ("de", "Haus", "Gleichng"),
            ("it", "equazione", "equazone"),
        ] {
            let d = checker(code);
            assert_eq!(wrong(&d, &none, &words(&[good, bad])), [bad], "{code}");
        }
    }

    #[test]
    fn sugerencias() {
        assert!(suggest(&checker("es"), "ecuacion").contains(&"ecuación".to_owned()));
        assert!(suggest(&checker("en"), "teh").contains(&"the".to_owned()));
        assert!(suggest(&checker("de"), "Gleichng").contains(&"Gleichung".to_owned()));
        assert!(suggest(&checker("es"), "ecuacion").len() <= MAX_SUGGESTIONS);
    }

    #[test]
    fn el_diccionario_personal_tambien_vale() {
        let d = checker("es");
        let personal: HashSet<String> = ["autovalores", "Kirchhoff"].map(String::from).into();
        let list = words(&[
            "autovalores",
            "Autovalores",
            "AUTOVALORES",
            "Kirchhoff",
            "kirchhoff",
        ]);
        // A lowercase word also counts capitalized or in capitals; a capitalized one, not in
        // lowercase (as Hunspell does).
        assert_eq!(wrong(&d, &personal, &list), ["kirchhoff"]);
    }

    #[test]
    fn los_diccionarios_se_cargan_una_vez() {
        let dir = tempfile::TempDir::new().unwrap();
        let (aff, dic) = files("en");
        std::fs::copy(aff, dir.path().join("en.aff")).unwrap();
        std::fs::copy(dic, dir.path().join("en.dic")).unwrap();
        let checkers = Checkers::new(dir.path().to_owned());
        let first = checkers.get("en").unwrap();
        let second = checkers.get("en").unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        // Only languages of the table, never a path.
        assert!(checkers.get("../en").is_err());
        assert!(checkers.get("ru").is_err());
    }
}

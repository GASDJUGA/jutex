//! The language of the operating system's interface.

use crate::lang;

/// Language tags of a Linux session, most preferred first: `LANGUAGE` (a list separated by
/// `:`), then `LC_ALL`, `LC_MESSAGES` and `LANG`, as gettext reads them. `var` reads a
/// variable (the environment, or a fixed table in the tests).
pub fn linux_candidates(var: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let mut tags: Vec<String> = var("LANGUAGE")
        .map(|list| list.split(':').map(str::to_owned).collect())
        .unwrap_or_default();
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(value) = var(name) {
            tags.push(value);
        }
    }
    tags.retain(|t| !t.is_empty());
    tags
}

/// The interface languages of the system, most preferred first.
#[cfg(not(any(target_os = "macos", windows)))]
pub fn system_languages() -> Vec<String> {
    linux_candidates(|k| std::env::var(k).ok())
}

/// The interface languages of the system, most preferred first (System Settings → Language).
#[cfg(target_os = "macos")]
pub fn system_languages() -> Vec<String> {
    use objc2_foundation::NSLocale;
    NSLocale::preferredLanguages()
        .iter()
        .map(|tag| tag.to_string())
        .collect()
}

/// The language of a Windows `LANGID` (by its primary language: es-MX and es-ES are `es`).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn code_of_langid(langid: u16) -> Option<&'static str> {
    let primary = langid & 0x3ff;
    lang::all()
        .iter()
        .find(|l| l.windows_lang == primary)
        .map(|l| l.code.as_str())
}

/// The interface language of Windows (the user's display language).
#[cfg(windows)]
pub fn system_languages() -> Vec<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // SAFETY: reads the user's language.
    code_of_langid(unsafe { GetUserDefaultUILanguage() })
        .map(|code| vec![code.to_owned()])
        .unwrap_or_default()
}

/// The language JuTeX uses when the setting is «same as the system».
pub fn system_locale() -> &'static str {
    lang::resolve(&system_languages())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::resolve;

    fn env(vars: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            vars.iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn idioma_de_windows() {
        assert_eq!(code_of_langid(0x080a), Some("es")); // es-MX
        assert_eq!(code_of_langid(0x2c0a), Some("es")); // es-AR
        assert_eq!(code_of_langid(0x0409), Some("en")); // en-US
        assert_eq!(code_of_langid(0x0416), Some("pt")); // pt-BR
        assert_eq!(code_of_langid(0x0816), Some("pt")); // pt-PT
        assert_eq!(code_of_langid(0x0c07), Some("de")); // de-AT
        assert_eq!(code_of_langid(0x040c), Some("fr"));
        assert_eq!(code_of_langid(0x0410), Some("it"));
        assert_eq!(code_of_langid(0x0419), None); // ru-RU
    }

    #[test]
    fn orden_de_variables_en_linux() {
        // LANGUAGE (a list with «:») first, then LC_ALL, LC_MESSAGES, LANG.
        let fr = linux_candidates(env(&[("LANGUAGE", "fr:en"), ("LANG", "es_CL.UTF-8")]));
        assert_eq!(resolve(&fr), "fr");
        let es = linux_candidates(env(&[
            ("LANG", "en_US.UTF-8"),
            ("LC_MESSAGES", "es_CL.UTF-8"),
        ]));
        assert_eq!(resolve(&es), "es");
        let all = linux_candidates(env(&[("LC_ALL", "de_DE.UTF-8"), ("LC_MESSAGES", "it_IT")]));
        assert_eq!(resolve(&all), "de");
        assert_eq!(
            resolve(&linux_candidates(env(&[("LANG", "C.UTF-8")]))),
            "en"
        );
        assert_eq!(resolve(&linux_candidates(env(&[]))), "en");
    }
}

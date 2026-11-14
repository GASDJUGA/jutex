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

/// The interface language of Windows (the user's display language).
#[cfg(windows)]
pub fn system_languages() -> Vec<String> {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    // SAFETY: reads the user's language.
    let primary = unsafe { GetUserDefaultUILanguage() } & 0x3ff;
    lang::all()
        .iter()
        .filter(|l| l.windows_lang == primary)
        .map(|l| l.code.clone())
        .collect()
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

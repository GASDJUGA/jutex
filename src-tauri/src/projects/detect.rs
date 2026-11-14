//! Detection of a project's main file and compiler from its sources.

use crate::projects::meta::{Compiler, META_DIR};
use crate::util::paths::to_relative;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// How deep to look for `.tex` files when opening a folder.
const MAX_DEPTH: usize = 4;
/// Only the start of a file is read to look for `\documentclass`.
const MAX_HEAD_BYTES: usize = 64 * 1024;

/// Removes a `%` comment from a LaTeX line (ignoring escaped `\%`).
pub(crate) fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'%' => return &line[..i],
            _ => i += 1,
        }
    }
    line
}

/// Whether `source` declares `\documentclass` outside a comment.
pub fn has_documentclass(source: &str) -> bool {
    source
        .lines()
        .any(|line| strip_comment(line).contains("\\documentclass"))
}

/// `.tex` files (relative paths) that contain `\documentclass`: candidates for the main file.
/// `main.tex` comes first, then shallower files, then alphabetical order.
pub fn main_file_candidates(root: &Path) -> Vec<String> {
    let mut found: Vec<(bool, usize, String)> = WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden_or_meta(e.file_name()))
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "tex"))
        .filter(|e| read_head(e.path()).is_some_and(|s| has_documentclass(&s)))
        .filter_map(|e| {
            let rel = to_relative(root, e.path()).ok()?;
            let is_main = rel == "main.tex";
            Some((!is_main, e.depth(), rel))
        })
        .collect();
    found.sort();
    found.into_iter().map(|(_, _, rel)| rel).collect()
}

/// Guesses the compiler: a `% !TEX program = …` magic comment wins; otherwise packages that
/// need XeLaTeX or LuaLaTeX; otherwise pdfLaTeX.
pub fn detect_compiler(source: &str) -> Compiler {
    for line in source.lines().take(20) {
        let lower = line.trim().to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix('%') {
            let rest = rest.trim_start();
            if let Some(value) = rest
                .strip_prefix("!tex program")
                .or_else(|| rest.strip_prefix("!tex ts-program"))
                .and_then(|r| r.trim_start().strip_prefix('='))
            {
                match value.trim() {
                    "xelatex" => return Compiler::Xelatex,
                    "lualatex" => return Compiler::Lualatex,
                    "pdflatex" => return Compiler::Pdflatex,
                    _ => {}
                }
            }
        }
    }
    let code: String = source
        .lines()
        .map(strip_comment)
        .collect::<Vec<_>>()
        .join("\n");
    let uses = |needle: &str| code.contains(needle);
    if uses("\\directlua") || uses("{luacode}") || uses("{luatextra}") || uses("{luacode") {
        Compiler::Lualatex
    } else if uses("{fontspec}")
        || uses("{polyglossia}")
        || uses("{unicode-math}")
        || uses("{xeCJK}")
    {
        Compiler::Xelatex
    } else {
        Compiler::Pdflatex
    }
}

fn is_hidden_or_meta(name: &std::ffi::OsStr) -> bool {
    let name = name.to_string_lossy();
    name.starts_with('.') || name == META_DIR || name == "node_modules"
}

fn read_head(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let head = &bytes[..bytes.len().min(MAX_HEAD_BYTES)];
    Some(String::from_utf8_lossy(head).into_owned())
}

/// The main language of a document, from its preamble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedLanguage {
    /// One of JuTeX's languages (its code).
    Supported(&'static str),
    /// A language JuTeX has no dictionary for (`russian`).
    Unsupported,
    /// No language declared (no babel or polyglossia language).
    Unknown,
}

/// Options of babel that are not languages.
const BABEL_FLAGS: [&str; 12] = [
    "activeacute",
    "activegrave",
    "base",
    "keepshorthands",
    "math",
    "nocase",
    "noconfigs",
    "safe",
    "showlanguages",
    "silent",
    "strings",
    "hyphenmap",
];

fn options(list: &str) -> impl Iterator<Item = &str> {
    list.split(',').map(str::trim).filter(|o| !o.is_empty())
}

fn regex(pattern: &str) -> regex::Regex {
    regex::Regex::new(pattern).expect("valid regex")
}

/// The main language of the document whose main file is `source`: polyglossia's
/// `\setdefaultlanguage`/`\setmainlanguage`, a `\babelprovide[main]`, or the last language
/// of babel's options (or `main=`), also those of `\documentclass` (babel takes them as
/// global). Only the preamble, without comments.
pub fn detect_language(source: &str) -> DetectedLanguage {
    let preamble: String = source
        .lines()
        .map(strip_comment)
        .take_while(|line| !line.contains("\\begin{document}"))
        .collect::<Vec<_>>()
        .join("\n");
    let named = |name: &str| match crate::lang::from_tex_name(name) {
        Some(code) => DetectedLanguage::Supported(code),
        None => DetectedLanguage::Unsupported,
    };

    let polyglossia = regex(r"\\set(?:default|main)language\s*(?:\[[^\]]*\])?\s*\{([^}]*)\}");
    if let Some(c) = polyglossia.captures_iter(&preamble).last() {
        return named(&c[1]);
    }
    let provide = regex(r"\\babelprovide\s*\[([^\]]*)\]\s*\{([^}]*)\}");
    if let Some(c) = provide
        .captures_iter(&preamble)
        .filter(|c| options(&c[1]).any(|o| o == "main"))
        .last()
    {
        return named(&c[2]);
    }

    let usepackage = regex(r"\\usepackage\s*(?:\[([^\]]*)\])?\s*\{([^}]*)\}");
    let mut babel = false;
    let mut languages: Vec<&str> = Vec::new();
    for c in usepackage.captures_iter(&preamble) {
        if !options(&c[2]).any(|p| p == "babel") {
            continue;
        }
        babel = true;
        let Some(list) = c.get(1) else { continue };
        for option in options(list.as_str()) {
            if let Some(main) = option.strip_prefix("main=") {
                return named(main.trim());
            }
            let word = option.chars().all(|ch| ch.is_ascii_alphabetic());
            if word && !BABEL_FLAGS.contains(&option.to_ascii_lowercase().as_str()) {
                languages.push(option);
            }
        }
    }
    if let Some(last) = languages.last() {
        return named(last);
    }
    if babel {
        // babel without languages takes those of the class (only known names count: the
        // class has options of its own, like `twocolumn`).
        let class = regex(r"\\documentclass\s*\[([^\]]*)\]");
        if let Some(c) = class.captures(&preamble)
            && let Some(code) = options(&c[1]).filter_map(crate::lang::from_tex_name).last()
        {
            return DetectedLanguage::Supported(code);
        }
    }
    DetectedLanguage::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn documentclass_ignora_comentarios() {
        assert!(has_documentclass("\\documentclass{article}\n"));
        assert!(has_documentclass(
            "% preámbulo\n  \\documentclass[12pt]{report}"
        ));
        assert!(!has_documentclass(
            "% \\documentclass{article}\n\\section{x}"
        ));
        assert!(has_documentclass(
            "50\\% de descuento \\documentclass{article}"
        ));
        assert!(!has_documentclass("\\input{capitulo}"));
    }

    #[test]
    fn candidatos_con_main_primero_y_sin_carpetas_ocultas() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let doc = "\\documentclass{article}\n\\begin{document}\\end{document}\n";
        fs::create_dir_all(root.join("capitulos")).unwrap();
        fs::create_dir_all(root.join(".jutex/build")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("anexo.tex"), doc).unwrap();
        fs::write(root.join("main.tex"), doc).unwrap();
        fs::write(root.join("capitulos/borrador.tex"), doc).unwrap();
        fs::write(root.join("capitulos/intro.tex"), "\\section{Intro}").unwrap();
        fs::write(root.join(".jutex/build/main.tex"), doc).unwrap();
        fs::write(root.join(".git/x.tex"), doc).unwrap();
        fs::write(root.join("notas.txt"), doc).unwrap();

        assert_eq!(
            main_file_candidates(root),
            ["main.tex", "anexo.tex", "capitulos/borrador.tex"]
        );
    }

    #[test]
    fn sin_candidatos_en_carpeta_sin_documentos() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.tex"), "\\section{x}").unwrap();
        assert!(main_file_candidates(tmp.path()).is_empty());
    }

    #[test]
    fn detecta_el_compilador() {
        assert_eq!(
            detect_compiler("% !TEX program = lualatex\n\\usepackage{fontspec}"),
            Compiler::Lualatex
        );
        assert_eq!(
            detect_compiler("%!TEX TS-program = xelatex\n"),
            Compiler::Xelatex
        );
        assert_eq!(
            detect_compiler("% !TeX program=pdflatex\n\\usepackage{fontspec}"),
            Compiler::Pdflatex
        );
        assert_eq!(
            detect_compiler("\\usepackage{fontspec}\n\\usepackage[spanish]{babel}"),
            Compiler::Xelatex
        );
        assert_eq!(
            detect_compiler("\\usepackage{polyglossia}"),
            Compiler::Xelatex
        );
        assert_eq!(detect_compiler("\\usepackage{luacode}"), Compiler::Lualatex);
        assert_eq!(
            detect_compiler("\\usepackage[utf8]{inputenc}"),
            Compiler::Pdflatex
        );
        assert_eq!(
            detect_compiler("% \\usepackage{fontspec}\n"),
            Compiler::Pdflatex
        );
    }
    #[test]
    fn idioma_del_preambulo() {
        use super::DetectedLanguage::*;
        assert_eq!(
            detect_language("\\usepackage[english,spanish]{babel}"),
            Supported("es")
        );
        assert_eq!(
            detect_language("\\usepackage[main=english,spanish]{babel}"),
            Supported("en")
        );
        assert_eq!(
            detect_language("%\\usepackage[french]{babel}\n\\usepackage[ngerman]{babel}"),
            Supported("de")
        );
        assert_eq!(
            detect_language(
                "\\usepackage{polyglossia}\n\\setdefaultlanguage[variant=brazilian]{portuguese}"
            ),
            Supported("pt")
        );
        assert_eq!(
            detect_language("\\setmainlanguage{italian}"),
            Supported("it")
        );
        assert_eq!(
            detect_language("\\babelprovide[main,import]{french}"),
            Supported("fr")
        );
        assert_eq!(detect_language("\\usepackage[russian]{babel}"), Unsupported);
        assert_eq!(
            detect_language("\\usepackage[T1]{fontenc}\n\\usepackage{babel}"),
            Unknown
        );
        assert_eq!(
            detect_language("\\documentclass[french,a4paper]{article}\n\\usepackage{babel}"),
            Supported("fr")
        );
        assert_eq!(detect_language("\\documentclass{article}"), Unknown);
        // Only the preamble: a babel after \begin{document} is not the document's.
        assert_eq!(
            detect_language("\\begin{document}\n\\usepackage[french]{babel}"),
            Unknown
        );
        // Options with spaces and line breaks.
        assert_eq!(
            detect_language("\\usepackage[\n  spanish,\n  es-tabla\n]{babel}"),
            Supported("es")
        );
    }
}

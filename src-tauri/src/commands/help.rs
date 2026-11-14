//! Links of the «Ayuda» menu, opened in the system browser. Only these fixed addresses:
//! the webview never sends a URL.

use crate::error::{AppError, AppResult};
use tauri::AppHandle;

/// The LaTeX manual for `locale`: its wikibook (the English one if there is none).
fn latex_docs(locale: &str) -> &'static str {
    crate::lang::get(locale)
        .or_else(|| crate::lang::get("en"))
        .map(|l| l.wikibook.as_str())
        .unwrap_or("https://en.wikibooks.org/wiki/LaTeX")
}

/// Opens a help page in the browser (a network use the user asked for).
#[tauri::command]
pub fn open_help_link(app: AppHandle, topic: String, locale: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let url = match topic.as_str() {
        "latex" => latex_docs(&locale),
        _ => return Err(AppError::Forbidden(format!("enlace de ayuda «{topic}»"))),
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_segun_el_idioma() {
        assert!(latex_docs("es").starts_with("https://es.wikibooks.org/"));
        assert!(latex_docs("en").starts_with("https://en.wikibooks.org/"));
        assert_eq!(latex_docs("pt"), "https://pt.wikibooks.org/wiki/Latex");
        assert_eq!(
            latex_docs("de"),
            "https://de.wikibooks.org/wiki/LaTeX-Kompendium"
        );
        // No Italian wikibook: the English one.
        assert_eq!(latex_docs("it"), latex_docs("en"));
        assert_eq!(latex_docs("xx"), latex_docs("en"));
        assert_eq!(latex_docs("fr"), "https://fr.wikibooks.org/wiki/LaTeX");
    }
}

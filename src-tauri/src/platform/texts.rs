//! Texts the backend shows itself, outside the webview: the macOS menu bar and the Windows
//! notice that asks to close LibreTeX. Here, not behind their `cfg`, so they are tested on
//! every system.

/// The macOS menu's texts in `locale` (English if JuTeX does not have it): Edit, Window,
/// Quit, Hide, Hide Others, Show All.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn menu(locale: &str) -> [&'static str; 6] {
    match locale {
        "es" => [
            "Edición",
            "Ventana",
            "Salir de JuTeX",
            "Ocultar JuTeX",
            "Ocultar otros",
            "Mostrar todo",
        ],
        "pt" => [
            "Editar",
            "Janela",
            "Encerrar JuTeX",
            "Ocultar JuTeX",
            "Ocultar Outros",
            "Mostrar Tudo",
        ],
        "fr" => [
            "Édition",
            "Fenêtre",
            "Quitter JuTeX",
            "Masquer JuTeX",
            "Masquer les autres",
            "Tout afficher",
        ],
        "de" => [
            "Bearbeiten",
            "Fenster",
            "JuTeX beenden",
            "JuTeX ausblenden",
            "Andere ausblenden",
            "Alle einblenden",
        ],
        "it" => [
            "Modifica",
            "Finestra",
            "Esci da JuTeX",
            "Nascondi JuTeX",
            "Nascondi altre",
            "Mostra tutte",
        ],
        _ => [
            "Edit",
            "Window",
            "Quit JuTeX",
            "Hide JuTeX",
            "Hide Others",
            "Show All",
        ],
    }
}

/// Windows: the notice that asks to close LibreTeX before moving its folders, in `locale`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn close_libretex(locale: &str) -> &'static str {
    match locale {
        "es" => {
            "LibreTeX está abierto.\n\nCiérralo para que JuTeX traiga tus proyectos, ajustes y \
             pestañas, y pulsa Reintentar.\n\nSi cancelas, JuTeX no se abre ahora; lo hará la \
             próxima vez."
        }
        "pt" => {
            "O LibreTeX está aberto.\n\nFeche-o para que o JuTeX traga seus projetos, \
             configurações e abas, e clique em Repetir.\n\nSe você cancelar, o JuTeX não abre \
             agora; abrirá na próxima vez."
        }
        "fr" => {
            "LibreTeX est ouvert.\n\nFermez-le pour que JuTeX récupère vos projets, réglages et \
             onglets, puis cliquez sur Réessayer.\n\nSi vous annulez, JuTeX ne s’ouvre pas \
             maintenant ; il le fera la prochaine fois."
        }
        "de" => {
            "LibreTeX ist geöffnet.\n\nSchließen Sie es, damit JuTeX Ihre Projekte, \
             Einstellungen und Tabs übernehmen kann, und klicken Sie auf Wiederholen.\n\nWenn \
             Sie abbrechen, öffnet sich JuTeX jetzt nicht, sondern beim nächsten Mal."
        }
        "it" => {
            "LibreTeX è aperto.\n\nChiudilo perché JuTeX possa portare qui i tuoi progetti, le \
             impostazioni e le schede, poi fai clic su Riprova.\n\nSe annulli, JuTeX non si apre \
             ora; lo farà la prossima volta."
        }
        _ => {
            "LibreTeX is open.\n\nClose it so JuTeX can bring over your projects, settings and \
             tabs, then click Retry.\n\nIf you cancel, JuTeX does not open now; it will next time."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_en_cada_idioma() {
        assert_eq!(menu("de")[0], "Bearbeiten");
        assert_eq!(menu("es")[2], "Salir de JuTeX");
        assert_eq!(menu("pt")[1], "Janela");
        assert_eq!(menu("xx"), menu("en"));
        for l in crate::lang::all() {
            assert!(menu(&l.code).iter().all(|t| !t.is_empty()), "{}", l.code);
        }
    }

    #[test]
    fn aviso_de_libretex_en_cada_idioma() {
        assert!(close_libretex("fr").starts_with("LibreTeX est ouvert"));
        assert!(close_libretex("es").starts_with("LibreTeX está abierto"));
        assert_eq!(close_libretex("xx"), close_libretex("en"));
    }
}

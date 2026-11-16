// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// The WebKitGTK settings below only apply on Linux, but their tests run everywhere.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

fn main() {
    #[cfg(target_os = "linux")]
    configure_webkit_rendering();

    jutex_lib::run()
}

const DISABLE_DMABUF: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
const NV_EXPLICIT_SYNC: &str = "__NV_DISABLE_EXPLICIT_SYNC";

/// What the system looks like, for choosing how WebKitGTK renders.
#[derive(Debug, Clone, Copy, Default)]
struct System {
    wayland: bool,
    nvidia: bool,
    /// `JUTEX_SAFE_RENDERER=1`: the user asks for the old, software-composited path.
    safe: bool,
    /// The user already chose `WEBKIT_DISABLE_DMABUF_RENDERER`.
    user_set_dmabuf: bool,
}

/// Environment variables to set (only those the user has not set).
///
/// WebKitGTK on Wayland with NVIDIA drivers: its DMA-BUF renderer crashes the Wayland
/// connection ("Error 71 (Protocol error)") because of the driver's explicit sync. Turning
/// the whole renderer off (`WEBKIT_DISABLE_DMABUF_RENDERER=1`) avoids the crash but leaves
/// the page without GPU compositing, and scrolling stutters on large screens. Instead, keep
/// the GPU renderer and disable only the driver's explicit sync. `JUTEX_SAFE_RENDERER=1`
/// brings back the old behavior in case a system still fails.
fn renderer_vars(system: System) -> Vec<(&'static str, &'static str)> {
    if system.safe {
        vec![(DISABLE_DMABUF, "1")]
    } else if system.wayland && system.nvidia && !system.user_set_dmabuf {
        vec![(NV_EXPLICIT_SYNC, "1")]
    } else {
        Vec::new()
    }
}

/// `JUTEX_SAFE_RENDERER=1` (or `LIBRETEX_SAFE_RENDERER=1`, its name before 2.0.0).
fn safe_renderer_requested(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> bool {
    ["JUTEX_SAFE_RENDERER", "LIBRETEX_SAFE_RENDERER"]
        .iter()
        .any(|name| var(name).is_some_and(|v| v == "1"))
}

const GTK_IM_MODULE: &str = "GTK_IM_MODULE";

/// How keys reach GTK, for choosing its input module.
#[derive(Debug, Clone, Copy, Default)]
struct InputSetup {
    wayland: bool,
    /// The user already chose `GTK_IM_MODULE`.
    user_set_gtk: bool,
    /// An input method (IBus, fcitx, KWin's virtual keyboard…) is in use.
    input_method: bool,
    /// `JUTEX_SYSTEM_IM=1`: the user asks for the system's input module.
    system_im: bool,
}

/// GTK's input module. On Wayland, GTK 3 hands every key to the compositor's text-input
/// protocol and waits for it; with several keys pressed at once they arrived late in
/// WebKitGTK (pedido del usuario). GTK's own simple module still handles dead keys and the
/// Compose key, and keys go straight to the page. Not with an input method (it needs the
/// compositor's protocol) nor if the user chose the module.
fn input_vars(setup: InputSetup) -> Vec<(&'static str, &'static str)> {
    if setup.wayland && !setup.user_set_gtk && !setup.input_method && !setup.system_im {
        vec![(GTK_IM_MODULE, "gtk-im-context-simple")]
    } else {
        Vec::new()
    }
}

/// An input method announced in the environment (`QT_IM_MODULE`, `XMODIFIERS=@im=…`).
fn input_method_in_env(var: impl Fn(&str) -> Option<std::ffi::OsString>) -> bool {
    let set = |name: &str| var(name).is_some_and(|v| !v.is_empty());
    let xmodifiers = var("XMODIFIERS")
        .map(|v| v.to_string_lossy().into_owned())
        .unwrap_or_default();
    set("QT_IM_MODULE")
        || set("SDL_IM_MODULE")
        || (xmodifiers.contains("@im=") && !xmodifiers.contains("@im=none"))
}

/// KWin (Plasma on Wayland) configured with an input method (`kwinrc`, `[Wayland]`).
fn kwin_has_input_method(kwinrc: &str) -> bool {
    let mut in_wayland = false;
    kwinrc.lines().any(|line| {
        let line = line.trim();
        if line.starts_with('[') {
            in_wayland = line == "[Wayland]";
            return false;
        }
        in_wayland
            && line.split_once('=').is_some_and(|(key, value)| {
                key.trim().starts_with("InputMethod") && !value.trim().is_empty()
            })
    })
}

/// A running input method program, by its process name.
fn is_input_method_process(comm: &str) -> bool {
    matches!(
        comm.trim(),
        "fcitx5" | "fcitx" | "ibus-daemon" | "maliit-keyboard" | "maliit-server"
    )
}

#[cfg(target_os = "linux")]
fn input_method_running() -> bool {
    let Ok(dir) = std::fs::read_dir("/proc") else {
        return false;
    };
    dir.flatten().any(|entry| {
        std::fs::read_to_string(entry.path().join("comm"))
            .is_ok_and(|comm| is_input_method_process(&comm))
    })
}

#[cfg(target_os = "linux")]
fn configure_input_module(wayland: bool) {
    use std::env::var_os;

    let kwinrc = directories::BaseDirs::new()
        .and_then(|dirs| std::fs::read_to_string(dirs.config_dir().join("kwinrc")).ok())
        .unwrap_or_default();
    let setup = InputSetup {
        wayland,
        user_set_gtk: var_os(GTK_IM_MODULE).is_some(),
        input_method: input_method_in_env(|name| var_os(name))
            || kwin_has_input_method(&kwinrc)
            || (wayland && input_method_running()),
        system_im: var_os("JUTEX_SYSTEM_IM").is_some_and(|v| v == "1"),
    };
    for (name, value) in input_vars(setup) {
        // SAFETY: as in `configure_webkit_rendering`, still single-threaded.
        unsafe { std::env::set_var(name, value) };
    }
}

#[cfg(target_os = "linux")]
fn configure_webkit_rendering() {
    use std::env::var_os;

    let system = System {
        wayland: var_os("WAYLAND_DISPLAY").is_some()
            || var_os("XDG_SESSION_TYPE").is_some_and(|v| v == "wayland"),
        nvidia: std::path::Path::new("/proc/driver/nvidia/version").exists(),
        safe: safe_renderer_requested(|name| var_os(name)),
        user_set_dmabuf: var_os(DISABLE_DMABUF).is_some(),
    };
    for (name, value) in renderer_vars(system) {
        if var_os(name).is_none() {
            // SAFETY: since Rust 2024 `set_var` is unsafe because modifying the
            // environment is a data race if other threads read it concurrently. This runs
            // first thing in `main`, before Tauri/GTK or any other code could spawn
            // threads, so the process is still single-threaded.
            unsafe { std::env::set_var(name, value) };
        }
    }
    configure_input_module(system.wayland);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_modo_seguro_acepta_el_nombre_viejo() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, v)| std::ffi::OsString::from(v))
            }
        };
        assert!(safe_renderer_requested(env(&[(
            "JUTEX_SAFE_RENDERER",
            "1"
        )])));
        assert!(safe_renderer_requested(env(&[(
            "LIBRETEX_SAFE_RENDERER",
            "1"
        )])));
        assert!(!safe_renderer_requested(env(&[(
            "JUTEX_SAFE_RENDERER",
            "0"
        )])));
        assert!(!safe_renderer_requested(env(&[])));
    }

    #[test]
    fn wayland_con_nvidia_usa_la_gpu_sin_sync_explicita() {
        let system = System {
            wayland: true,
            nvidia: true,
            ..Default::default()
        };
        assert_eq!(renderer_vars(system), [(NV_EXPLICIT_SYNC, "1")]);
    }

    #[test]
    fn otros_sistemas_no_se_tocan() {
        let x11_nvidia = System {
            nvidia: true,
            ..Default::default()
        };
        assert!(renderer_vars(x11_nvidia).is_empty());
        let wayland_amd = System {
            wayland: true,
            ..Default::default()
        };
        assert!(renderer_vars(wayland_amd).is_empty());
    }

    #[test]
    fn en_wayland_sin_metodo_de_entrada_usa_el_modulo_simple_de_gtk() {
        let plain = InputSetup {
            wayland: true,
            ..Default::default()
        };
        assert_eq!(
            input_vars(plain),
            [(GTK_IM_MODULE, "gtk-im-context-simple")]
        );
    }

    #[test]
    fn con_un_metodo_de_entrada_o_en_x11_no_toca_nada() {
        let wayland = InputSetup {
            wayland: true,
            ..Default::default()
        };
        for setup in [
            InputSetup {
                wayland: false,
                ..wayland
            },
            InputSetup {
                user_set_gtk: true,
                ..wayland
            },
            InputSetup {
                input_method: true,
                ..wayland
            },
            InputSetup {
                system_im: true,
                ..wayland
            },
        ] {
            assert!(input_vars(setup).is_empty(), "{setup:?}");
        }
    }

    #[test]
    fn reconoce_un_metodo_de_entrada() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, v)| std::ffi::OsString::from(v))
            }
        };
        assert!(!input_method_in_env(env(&[])));
        assert!(!input_method_in_env(env(&[("XMODIFIERS", "@im=none")])));
        assert!(input_method_in_env(env(&[("XMODIFIERS", "@im=fcitx")])));
        assert!(input_method_in_env(env(&[("QT_IM_MODULE", "ibus")])));
        assert!(!kwin_has_input_method(
            "[Wayland]\nVirtualKeyboardEnabled=false\n"
        ));
        assert!(kwin_has_input_method(
            "[Wayland]\nInputMethod[$e]=/usr/share/applications/fcitx5-wayland-launcher.desktop\n"
        ));
        assert!(!kwin_has_input_method("[Wayland]\nInputMethod=\n"));
        assert!(is_input_method_process("fcitx5"));
        assert!(is_input_method_process("ibus-daemon"));
        assert!(!is_input_method_process("kwin_wayland"));
    }

    #[test]
    fn respeta_al_usuario_y_el_modo_seguro() {
        let chosen = System {
            wayland: true,
            nvidia: true,
            user_set_dmabuf: true,
            ..Default::default()
        };
        assert!(renderer_vars(chosen).is_empty());
        let safe = System {
            wayland: true,
            nvidia: true,
            safe: true,
            ..Default::default()
        };
        assert_eq!(renderer_vars(safe), [(DISABLE_DMABUF, "1")]);
    }
}

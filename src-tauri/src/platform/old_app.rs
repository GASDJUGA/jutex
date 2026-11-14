//! Windows: LibreTeX still open while JuTeX moves its folders (2.0.0). Windows does not let a
//! folder be renamed while a file inside is open, and LibreTeX's webview keeps its storage
//! open (the cookies even for reading): the move would fail, JuTeX would start with new empty
//! folders and, since those then exist, never move the old ones. So before moving anything
//! JuTeX asks to close LibreTeX, or does not open (it tries again next time).

use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IDRETRY, MB_ICONINFORMATION, MB_RETRYCANCEL, MB_SETFOREGROUND, MessageBoxW,
};

/// LibreTeX's program (`mainBinaryName` before 2.0.0).
const OLD_EXE: &str = "libretex.exe";

/// Whether `name` (a process's image name) is `exe`, ignoring case as Windows does.
pub fn same_exe(name: &str, exe: &str) -> bool {
    name.eq_ignore_ascii_case(exe)
}

/// Whether a process with this image name is running.
fn is_running(exe: &str) -> bool {
    // SAFETY: a snapshot of the system's processes, walked with a correctly sized entry and
    // closed here.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more && !found {
            let len = entry
                .szExeFile
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(entry.szExeFile.len());
            found = same_exe(&String::from_utf16_lossy(&entry.szExeFile[..len]), exe);
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
        found
    }
}

/// Asks to close LibreTeX; whether the user chose to try again.
fn ask_to_close() -> bool {
    // In the language of Windows' interface (JuTeX's settings may not exist yet).
    let text = super::texts::close_libretex(super::locale::system_locale());
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (text, title) = (wide(text), wide("JuTeX"));
    // SAFETY: both strings end in 0 and outlive the call; no owner window yet.
    let answer = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_RETRYCANCEL | MB_ICONINFORMATION | MB_SETFOREGROUND,
        )
    };
    answer == IDRETRY
}

/// Waits until LibreTeX is closed, asking the user; `false` if they cancelled.
pub fn wait_until_closed() -> bool {
    while is_running(OLD_EXE) {
        if !ask_to_close() {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_programa_se_reconoce_sin_mayusculas() {
        assert!(same_exe("LibreTeX.exe", OLD_EXE));
        assert!(same_exe("libretex.exe", OLD_EXE));
        assert!(!same_exe("jutex.exe", OLD_EXE));
        assert!(!same_exe("libretex.exe.bak", OLD_EXE));
    }

    #[test]
    fn este_proceso_esta_corriendo() {
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy();
        assert!(is_running(&name));
        assert!(!is_running("no-existe-ningun-programa-asi.exe"));
    }
}

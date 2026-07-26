use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::System::DataExchange::{
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, RegisterClipboardFormatW,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
use windows::core::HSTRING;

/// Clipboard formats that mean "do not record this".
///
/// Windows password managers register these alongside the payload. They are the
/// direct equivalent of the macOS nspasteboard markers, and honoring them is what
/// keeps copied passwords out of the history.
const EXCLUDE_FORMATS: &[&str] = &[
    "ExcludeClipboardContentFromMonitorProcessing",
    "CanIncludeInClipboardHistory",
    "CanUploadToCloudClipboard",
];

const CF_HTML: &str = "HTML Format";
const CF_RTF: &str = "Rich Text Format";

/// Monotonic counter bumped by the OS on every clipboard write.
///
/// `GetClipboardSequenceNumber` needs no `OpenClipboard` call, so polling it never
/// contends with the app that owns the clipboard — the failure mode that makes
/// naive clipboard managers break paste in other applications.
pub fn sequence() -> i64 {
    unsafe { GetClipboardSequenceNumber() as i64 }
}

fn format_available(name: &str) -> bool {
    unsafe {
        let id = RegisterClipboardFormatW(&HSTRING::from(name));
        if id == 0 {
            return false;
        }
        IsClipboardFormatAvailable(id).is_ok()
    }
}

pub fn is_concealed() -> bool {
    // `CanIncludeInClipboardHistory` / `CanUploadToCloudClipboard` carry a DWORD
    // payload of 0 to opt out. Presence alone is the conservative read: an app that
    // bothers to register them is expressing an opinion about being recorded, and
    // erring toward not storing a secret is the right default.
    EXCLUDE_FORMATS.iter().any(|f| format_available(f))
}

pub fn has_html() -> bool {
    format_available(CF_HTML)
}

pub fn has_rtf() -> bool {
    format_available(CF_RTF)
}

/// Executable name of the foreground window's process (e.g. `chrome.exe`).
///
/// Best-effort, same caveat as macOS: read after the copy, so a fast app switch can
/// mislabel. It is a label, not a security control.
pub fn frontmost_app() -> Option<String> {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.is_invalid() {
            return None;
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

        let mut buffer = [0u16; MAX_PATH as usize];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(handle);
        result.ok()?;

        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_owned)
    }
}

//! Registra a faixa como AppBar do Windows (SHAppBarMessage), reservando espaço
//! na borda inferior — outras janelas não a sobrepõem e maximizadas param acima.
//!
//! Fase 1: register (ABM_NEW → ABM_QUERYPOS → ABM_SETPOS) + unregister (ABM_REMOVE).
//! Fase 4 (TODO): tratar a mensagem de callback (ABN_POSCHANGED / ABN_FULLSCREENAPP)
//! e WM_DPICHANGED / WM_DISPLAYCHANGE p/ reposicionar via subclass do WndProc.

use tauri::WebviewWindow;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SetWindowPos, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE,
    SWP_SHOWWINDOW,
};

// Mensagem de callback que o shell usa p/ notificar a AppBar (WM_USER + 1).
const APPBAR_CALLBACK: u32 = 0x0400 + 1;

fn hwnd_of(window: &WebviewWindow) -> Result<HWND, String> {
    // `hwnd()` devolve o HWND da versão do crate `windows` que o Tauri usa, que
    // pode não ser a nossa. Reconstruímos pelo ponteiro cru p/ desacoplar versões.
    let raw = window.hwnd().map_err(|e| e.to_string())?;
    Ok(HWND(raw.0 as _))
}

fn base_data(hwnd: HWND) -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: APPBAR_CALLBACK,
        uEdge: ABE_BOTTOM,
        rc: RECT::default(),
        lParam: LPARAM(0),
    }
}

/// Registra a faixa e a posiciona na borda inferior.
/// `height_logical` em px lógicos; a escala DPI é aplicada aqui.
pub fn register(window: &WebviewWindow, height_logical: i32) -> Result<(), String> {
    let hwnd = hwnd_of(window)?;
    unsafe {
        let mut abd = base_data(hwnd);

        // 1. registra a AppBar
        if SHAppBarMessage(ABM_NEW, &mut abd) == 0 {
            return Err("ABM_NEW falhou".into());
        }

        // 2. DPI da janela → altura física
        let dpi = GetDpiForWindow(hwnd);
        let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
        let height_px = (height_logical as f32 * scale).round() as i32;

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        // 3. retângulo proposto: full-width na base
        abd.uEdge = ABE_BOTTOM;
        abd.rc = RECT {
            left: 0,
            top: screen_h - height_px,
            right: screen_w,
            bottom: screen_h,
        };

        // 4. o SO ajusta rc (ex.: descontando a taskbar)
        SHAppBarMessage(ABM_QUERYPOS, &mut abd);
        // reforça a altura a partir da borda inferior ajustada
        abd.rc.top = abd.rc.bottom - height_px;

        // 5. commit da posição
        SHAppBarMessage(ABM_SETPOS, &mut abd);

        // 6. move a janela p/ o rc final
        let r = abd.rc;
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Remove o registro de AppBar, liberando o espaço reservado (chamar no exit / hide).
pub fn unregister(window: &WebviewWindow) -> Result<(), String> {
    let hwnd = hwnd_of(window)?;
    unsafe {
        let mut abd = base_data(hwnd);
        SHAppBarMessage(ABM_REMOVE, &mut abd);
    }
    Ok(())
}

//! Registra a faixa como AppBar do Windows (SHAppBarMessage), reservando espaço
//! na borda escolhida (topo/base/esquerda/direita) do monitor escolhido.

use crate::model::Edge;
use tauri::WebviewWindow;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, APPBARDATA,
};
use tauri::{PhysicalPosition, PhysicalSize};

const APPBAR_CALLBACK: u32 = 0x0400 + 1;

/// Retângulo do monitor em pixels físicos (coords de tela virtual).
#[derive(Clone, Copy)]
pub struct DockRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

fn hwnd_of(window: &WebviewWindow) -> Result<HWND, String> {
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

fn edge_const(edge: Edge) -> u32 {
    match edge {
        Edge::Top => ABE_TOP,
        Edge::Bottom => ABE_BOTTOM,
        Edge::Left => ABE_LEFT,
        Edge::Right => ABE_RIGHT,
    }
}

/// Calcula o retângulo fino da faixa dentro do monitor, conforme a borda.
fn strip_rect(edge: Edge, mon: DockRect, thickness: i32) -> RECT {
    match edge {
        Edge::Bottom => RECT {
            left: mon.left,
            top: mon.bottom - thickness,
            right: mon.right,
            bottom: mon.bottom,
        },
        Edge::Top => RECT {
            left: mon.left,
            top: mon.top,
            right: mon.right,
            bottom: mon.top + thickness,
        },
        Edge::Left => RECT {
            left: mon.left,
            top: mon.top,
            right: mon.left + thickness,
            bottom: mon.bottom,
        },
        Edge::Right => RECT {
            left: mon.right - thickness,
            top: mon.top,
            right: mon.right,
            bottom: mon.bottom,
        },
    }
}

// Re-fixa a dimensão fina a partir da borda externa (após o SO ajustar no QUERYPOS).
fn repin_thickness(edge: Edge, rc: &mut RECT, thickness: i32) {
    match edge {
        Edge::Bottom => rc.top = rc.bottom - thickness,
        Edge::Top => rc.bottom = rc.top + thickness,
        Edge::Left => rc.right = rc.left + thickness,
        Edge::Right => rc.left = rc.right - thickness,
    }
}

/// Registra e posiciona a faixa. `thickness` em px físicos.
pub fn register(
    window: &WebviewWindow,
    edge: Edge,
    mon: DockRect,
    thickness: i32,
) -> Result<(), String> {
    let hwnd = hwnd_of(window)?;
    unsafe {
        let mut abd = base_data(hwnd);
        if SHAppBarMessage(ABM_NEW, &mut abd) == 0 {
            return Err("ABM_NEW falhou".into());
        }

        abd.uEdge = edge_const(edge);
        abd.rc = strip_rect(edge, mon, thickness);

        SHAppBarMessage(ABM_QUERYPOS, &mut abd);
        repin_thickness(edge, &mut abd.rc, thickness);

        SHAppBarMessage(ABM_SETPOS, &mut abd);

        let r = abd.rc;
        eprintln!(
            "[dock] rc final: ({},{}) {}x{}",
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top
        );
        // Posiciona/dimensiona via Tauri (trata DPI corretamente p/ o webview).
        window
            .set_position(PhysicalPosition::new(r.left, r.top))
            .map_err(|e| e.to_string())?;
        window
            .set_size(PhysicalSize::new(
                (r.right - r.left) as u32,
                (r.bottom - r.top) as u32,
            ))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Remove o registro de AppBar, liberando o espaço reservado.
pub fn unregister(window: &WebviewWindow) -> Result<(), String> {
    let hwnd = hwnd_of(window)?;
    unsafe {
        let mut abd = base_data(hwnd);
        SHAppBarMessage(ABM_REMOVE, &mut abd);
    }
    Ok(())
}

/// Re-doca no monitor `index` e borda `edge` (unregister + register), usando os
/// monitores que o Tauri enxerga. `thickness_logical` em px lógicos.
pub fn redock(
    window: &WebviewWindow,
    edge: Edge,
    monitor_index: usize,
    thickness_logical: u32,
) -> Result<(), String> {
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    for (i, m) in monitors.iter().enumerate() {
        let p = m.position();
        let s = m.size();
        eprintln!(
            "[dock] monitor[{i}] pos=({},{}) size={}x{} scale={}",
            p.x,
            p.y,
            s.width,
            s.height,
            m.scale_factor()
        );
    }
    let mon = monitors
        .get(monitor_index)
        .or_else(|| monitors.first())
        .ok_or("nenhum monitor disponível")?;

    let p = mon.position();
    let s = mon.size();
    eprintln!(
        "[dock] escolhido monitor[{monitor_index}] edge={edge:?} pos=({},{}) size={}x{}",
        p.x, p.y, s.width, s.height
    );
    let rect = DockRect {
        left: p.x,
        top: p.y,
        right: p.x + s.width as i32,
        bottom: p.y + s.height as i32,
    };
    let thickness = (thickness_logical as f64 * mon.scale_factor()).round() as i32;

    let _ = unregister(window);
    register(window, edge, rect, thickness)
}

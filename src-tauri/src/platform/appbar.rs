//! Registers the strip as a Windows AppBar (SHAppBarMessage), reserving space on the
//! chosen edge (top/bottom/left/right) of the chosen monitor.

use crate::model::Edge;
use tauri::WebviewWindow;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE,
    ABM_SETPOS, APPBARDATA,
};
use tauri::{PhysicalPosition, PhysicalSize};

const APPBAR_CALLBACK: u32 = 0x0400 + 1;

/// Monitor rectangle in physical pixels (virtual-screen coordinates).
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

/// Computes the thin strip rectangle within the monitor for the given edge.
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

// Re-pin the thin dimension from the outer edge (after the OS adjusts it in QUERYPOS).
fn repin_thickness(edge: Edge, rc: &mut RECT, thickness: i32) {
    match edge {
        Edge::Bottom => rc.top = rc.bottom - thickness,
        Edge::Top => rc.bottom = rc.top + thickness,
        Edge::Left => rc.right = rc.left + thickness,
        Edge::Right => rc.left = rc.right - thickness,
    }
}

/// Registers and positions the strip. `thickness` in physical pixels.
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
            return Err("ABM_NEW failed".into());
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
        // Position/size via Tauri (handles DPI correctly for the webview).
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

/// Unregisters the AppBar, freeing the reserved space.
pub fn unregister(window: &WebviewWindow) -> Result<(), String> {
    let hwnd = hwnd_of(window)?;
    unsafe {
        let mut abd = base_data(hwnd);
        SHAppBarMessage(ABM_REMOVE, &mut abd);
    }
    Ok(())
}

/// Re-docks on monitor `index` and edge `edge` (unregister + register), using the
/// monitors Tauri reports. `thickness_logical` in logical pixels.
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
        .ok_or("no monitor available")?;

    let p = mon.position();
    let s = mon.size();
    eprintln!(
        "[dock] chosen monitor[{monitor_index}] edge={edge:?} pos=({},{}) size={}x{}",
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

# Implementation plan / architecture — Agenda Strip

A thin calendar strip docked to a screen edge, showing the day as a timeline.
Windows-only. Stack: Tauri v2 (Rust) + Svelte + plain CSS. AppBar via `SHAppBarMessage`.
Default time window: −60 min .. +480 min. Default thickness: 24 logical px.

## Fixed decisions

- Providers behind a single `CalendarSource` trait (`list_events(from, to) -> NormalizedEvent[]`).
- Order: ICS by URL/file → Google (OAuth PKCE) → Microsoft Graph (reserved).
- SQLite cache; OAuth tokens **only** in the Windows Credential Manager (`keyring`).
- Sync: a Rust timer every N min (default 3) → writes SQLite → emits a Tauri event.
  The frontend never talks to external APIs directly.
- Three windows: `strip` (AppBar), `detail` (popover, closes on blur), `settings` (popup).

## Status

- **Core — done:** AppBar docking on all four edges + monitor selection (vertical on the
  sides); now-marker; lane layout + `+N` overflow badge; clock.
- **ICS — done:** provider + SQLite cache + sync engine + detail popover. RRULE expanded
  within the window (`rrule` crate).
- **Google — done:** OAuth PKCE + 127.0.0.1 loopback; refresh token in the keyring; reads
  the whole `calendarList` (owned + shared), per-calendar color. Auto-reconnect when the
  Testing-mode token expires (reuses stored client id/secret; no env needed).
- **Packaging — done:** autostart on login, single-instance guard, msi/nsis bundles.
- **Remaining polish:** tray icon (show/hide, sync now, quit); a full settings panel
  (time window, sync interval, per-account color); a Microsoft Graph provider
  (`AccountKind::Graph` + trait are reserved); `syncToken` incremental sync for Google.

## Data model

`NormalizedEvent { id, account_id, source_id, title, start, end, all_day, description?,
location?, meeting_link?, html_link?, attendees[], color }` — mirrored in `src/lib/types.ts`.

SQLite tables: `accounts`, `events` (attendees stored as inline JSON), `config`, `meta`.
Indexes on `events(start_utc, end_utc)` and `events(account_id)`.

### Tauri contract

- Backend → front (emit): `calendar://events-updated { events, generated_at }`,
  `sync://status`, `dock://changed { AppConfig }`, `detail://event`, `settings://open`.
- Front → backend (invoke): `get_events`, `get_config`, `set_config`, `list_accounts`,
  `add_ics_account`, `remove_account`, `set_account_color`, `trigger_sync`,
  `add_google_account`, `open_in_browser`, `open_detail`, `close_detail`, `get_monitors`,
  `set_dock`, `open_settings`. (Some account/config commands are the API surface for the
  planned settings/account UI and aren't called by the current UI yet.)

## AppBar (`src-tauri/src/platform/appbar.rs`)

Isolated behind `#[cfg(windows)]`. Sequence per (edge, monitor):
`ABM_NEW` → `ABM_QUERYPOS` (OS adjusts the rect) → re-pin the thin dimension → `ABM_SETPOS`.
The window is then positioned/sized via **Tauri** (`PhysicalPosition`/`PhysicalSize`) rather
than a raw `SetWindowPos`, so the webview's DPI stays consistent (a raw physical
`SetWindowPos` caused the content to overflow on a 150%-scaled monitor). `ABM_REMOVE` on
exit. Monitors are enumerated via Tauri (`available_monitors`). Future: handle
`ABN_POSCHANGED` / `ABN_FULLSCREENAPP` / `WM_DPICHANGE` via a WndProc subclass.

## Lane algorithm (`src/lib/layout.ts`)

Time is the main axis; overlapping events stack on the cross axis. Sort by (start, end),
group into transitive-overlap clusters, greedy first-fit per cluster; the cluster's
`laneCount` sets the row size. Cap `MAX_LANES = 3` (thin strip); events beyond that are
hidden and surfaced by a `+N` overflow badge.

## Notable risks / mitigations

- Logical (Tauri) vs physical (Win32) DPI mismatch → size/position via Tauri; the AppBar
  `rc` is the source of truth.
- `windows` crate version ≠ Tauri's → HWND reconstructed from the raw pointer
  (`HWND(raw.0 as _)`).
- Google Testing-mode refresh tokens expire after ~7 days → detected on `invalid_grant`
  and auto-reconnected.
- Multi-instance (autostart + manual) → `tauri-plugin-single-instance`.

## Key crates

`tauri` (+ plugins `opener`, `autostart`, `single-instance`), `rusqlite` (bundled),
`chrono` + `chrono-tz`, `icalendar`, `rrule`, `reqwest`, `keyring`, `tiny_http`, `sha2`,
`base64`, `getrandom`, `url`, `webbrowser`, `uuid`, `async-trait`, `tokio`, `windows`.

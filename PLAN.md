# Plano de Implementação — Agenda Timeline Strip

Faixa fina de agenda docada na borda **inferior** do desktop (acima da taskbar),
timeline **horizontal** (tempo corre esquerda→direita). Windows 11, 1 monitor.

Stack: Tauri v2 (Rust) + Svelte + CSS puro. AppBar via `SHAppBarMessage`.
Janela de tempo: −60min .. +480min. Altura: 32px lógicos.

## Decisões fixas

- Providers atrás da trait `CalendarSource` (`list_events(from, to) -> NormalizedEvent[]`).
- Ordem: ICS por URL → Google (OAuth PKCE + syncToken) → Microsoft Graph (stub).
- Cache em SQLite; tokens OAuth **só** no Windows Credential Manager (`keyring`).
- Sync: timer Rust a cada N min (default 3) → grava SQLite → emite evento Tauri.
- Duas janelas: `strip` (AppBar) e `detail` (popover, fecha no blur).

## Fases

- **Fase 1 — prova de valor (feita):** Tauri + AppBar + render de mocks + marcador de agora + lanes.
- **Fase 2:** provider ICS + SQLite + sync engine + popover de detalhe.
- **Fase 3:** Google OAuth (PKCE + loopback 127.0.0.1 + keyring + syncToken).
- **Fase 4:** polimento — tray, autostart, configurações, robustez multi-monitor/DPI.

## Modelo de dados

`NormalizedEvent { id, account_id, source_id, title, start, end, all_day, description?,
location?, meeting_link?, html_link?, attendees[], color }` — espelhado em `src/lib/types.ts`.

SQLite (Fase 2): tabelas `accounts`, `events` (attendees como JSON inline), `config`, `meta`.
Índices em `events(start_utc, end_utc)` e `events(account_id)`.

### Contrato Tauri

- Backend→front (emit): `calendar://events-updated { events, generated_at }`, `sync://status`.
- Front→backend (invoke): `get_events`, `get_event`, `trigger_sync`, `list/add/remove/set_color`
  de contas, `get/set_config`, `open_in_browser`, `open_detail`, `close_detail`, `set_strip_visible`.

## AppBar (`src-tauri/src/platform/appbar.rs`)

Isolado em `#[cfg(windows)]`. Sequência:
`ABM_NEW` (registra) → `ABM_QUERYPOS` (SO ajusta rc) → `ABM_SETPOS` (commit) → `SetWindowPos`.
`ABM_REMOVE` no exit/hide. DPI: `GetDpiForWindow` → `scale = dpi/96` → altura física.
Fase 4 (TODO): tratar `ABN_POSCHANGED` / `ABN_FULLSCREENAPP` / `WM_DPICHANGED` via subclass do WndProc.

## Algoritmo de lanes (`src/lib/layout.ts`)

Eixo X = tempo; sobrepostos empilham em Y. Ordena por (start, end), agrupa em clusters de
overlap transitivo, greedy first-fit por cluster; `laneCount` do cluster define a altura de linha.
Cap `MAX_LANES = 3` (faixa fina); excedentes ocultos (badge "+N" = evolução futura).

## Riscos principais

- Descasamento DPI lógico(Tauri)×físico(Win32) → `rc` do AppBar é fonte de verdade; tratar `WM_DPICHANGED`.
- Versão do crate `windows` ≠ a do Tauri → HWND reconstruído pelo ponteiro cru (`HWND(raw.0 as _)`).
- RRULE do ICS (Fase 2) → crate `rrule`, expandir só na janela.
- Refresh token Google em modo Testing expira em 7 dias → detectar e re-autenticar.
- Sobreposição ilegível a 32px → cap de lanes + detalhe no clique.

## Crates (Fases 2–4)

`rusqlite` (bundled), `chrono`+`chrono-tz`, `ical`, `rrule`, `reqwest` (rustls), `keyring`,
`oauth2`, `tiny_http`, `url`, `uuid`, `thiserror`, `tracing`, `tauri-plugin-autostart`,
`tauri-plugin-opener`, `windows`.

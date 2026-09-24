# Agenda Strip

A thin, always-visible calendar timeline docked to a screen edge on Windows.
It shows your day as a horizontal (or vertical) strip: each event is a colored
block placed and sized by its time, with a marker for "now". Aesthetically
inspired by taskbar widgets like Toolbar Hero — but the content is just your
calendar.

Built with **Tauri v2 (Rust)** + **Svelte** (plain CSS, no heavy UI framework).
The strip registers as a Windows **AppBar** (`SHAppBarMessage`) so the OS reserves
its space and other windows don't cover it.

> **Windows-only.** It relies on the Win32 AppBar API, so it must be built and run
> on native Windows (PowerShell), not WSL.

## Features

- **Docked strip** on any screen edge (top / bottom / left / right) and any monitor —
  pick both from a small popup by clicking the clock. Left/right render vertically.
- **Google Calendar** via OAuth 2.0 (PKCE, loopback redirect). Reads **all** calendars
  visible in the account (owned **and** shared), each with its own color.
- **ICS** calendars by secret URL or local file (no OAuth) — recurring events (RRULE)
  expanded within the window.
- **Detail popover** on click: title, time, location, attendees, meeting link, and an
  "Open in browser" button. Closes on blur.
- **Now marker**, overlapping events laid out in **lanes** with a **`+N`** overflow badge,
  and a **clock** at the far end.
- **Secure tokens**: OAuth refresh tokens are stored in the **Windows Credential Manager**
  (via `keyring`), never in a file. Event cache + config live in a local SQLite DB.
- **Autostart** on login, **single-instance**, low resource usage (periodic sync, not streaming).

## Prerequisites (Windows)

1. **Rust** (`rustup`, toolchain `stable-msvc`) — https://rustup.rs
2. **Node.js** 20+ (with npm)
3. **WebView2** runtime — already present on Windows 11
4. **Visual Studio C++ Build Tools** (workload "Desktop development with C++",
   incl. the Windows 10/11 SDK — the linker needs `dbghelp.lib`)

## Google Cloud OAuth setup (one time)

To read Google calendars, the app needs an OAuth client. This is a one-time, ~5-minute
setup in the Google Cloud Console; afterwards it's just a normal "Sign in with Google"
prompt.

1. Go to https://console.cloud.google.com and create (or select) a project.
2. **APIs & Services → Library** → search **Google Calendar API** → **Enable**.
3. **APIs & Services → OAuth consent screen** → User type **External** →
   **Publishing status: Testing** → under **Audience → Test users**, add your Google
   account's email.
4. **APIs & Services → Credentials → + Create credentials → OAuth client ID** →
   Application type **Desktop app** → **Create**. Copy the **Client ID** and
   **Client secret**. (Desktop clients allow the `127.0.0.1` loopback redirect the app
   uses — no redirect URI to register.)

> **Testing-mode note:** refresh tokens issued in Testing mode expire after ~7 days.
> The app handles this automatically: when a sync sees an expired/revoked token, it
> reopens the Google login (reusing the stored client id/secret) and refreshes it.

Your login covers every calendar visible in that Google account, including ones shared
with you — you don't need to own them.

## Configuration (environment variables)

Set these in the shell before running (or before the first launch of the installed app):

| Variable | Purpose |
|---|---|
| `AGENDA_GOOGLE_CLIENT_ID` | Google OAuth client id (triggers the one-time login on first run) |
| `AGENDA_GOOGLE_CLIENT_SECRET` | Google OAuth client secret |
| `AGENDA_ICS_URL` | Optional: seed an ICS account from a secret iCal URL **or** a local file path |

After the first successful login, the Google account (with its client id/secret) and the
refresh token are persisted, so the env vars are only needed once.

## Run (dev)

In PowerShell, at the repo root:

```powershell
$env:AGENDA_GOOGLE_CLIENT_ID="....apps.googleusercontent.com"
$env:AGENDA_GOOGLE_CLIENT_SECRET="GOCSPX-..."
npm install
npm run tauri dev
```

On first run the browser opens for the Google consent screen (in Testing mode, click
**Advanced → Continue**). The strip then docks and fills with your real events.

To try it without Google, point `AGENDA_ICS_URL` at the bundled fixture:

```powershell
$env:AGENDA_ICS_URL="$PWD\tests\fixtures\sample.ics"
npm run tauri dev
```

## Build & install

```powershell
npm run tauri build
```

Produces installers under `src-tauri/target/release/bundle/` (`.msi` and NSIS `.exe`).
Install one and launch it once so autostart registers a stable path. The installed app
reuses the same data directory, so your Google login carries over.

## Usage

- **Click the clock** (far end of the strip) → popup to choose the **monitor** and
  **edge**; changes apply live and persist.
- **Click an event** → detail popover; click outside to close.

## Tests

```powershell
npm test          # vitest: time-window math + lane layout (pure logic, no Tauri)
npm run check     # svelte-check (type checking)
```

## Architecture

- `src/` — Svelte frontend (three windows: `strip`, `detail`, `settings`).
  - `src/lib/` — `time.ts` (window/position math), `layout.ts` (lane assignment +
    overflow), `types.ts`, `ipc.ts` (Tauri command/event wrappers), `mocks.ts`.
  - `src/strip/` — `Strip.svelte`, `EventBlock.svelte`, `NowMarker.svelte`.
  - `src/detail/`, `src/settings/` — popover windows.
- `src-tauri/src/` — Rust backend.
  - `platform/appbar.rs` — Win32 AppBar (`#[cfg(windows)]`, isolated).
  - `providers/` — `CalendarSource` trait + `ics.rs`, `google.rs` (Microsoft `graph.rs`
    is reserved for the future).
  - `oauth/` — PKCE flow + loopback server; `tokens.rs` (Credential Manager).
  - `db/` — SQLite (accounts, events cache, config).
  - `sync.rs` — periodic sync engine; `commands.rs` — Tauri commands.

### Data locations

- SQLite DB + `crash.log`: `%APPDATA%\com.agendastrip\`
- OAuth refresh tokens: **Windows Credential Manager** (service `agenda-strip`)

## Security

The app talks only to your calendar providers (Google / the ICS URLs you configure) —
no telemetry, no third-party services. Refresh tokens live in the OS credential store.
The Google `client_secret` for a Desktop OAuth client is not confidential (per Google's
installed-app model) and is stored in the local SQLite config for the single local user.

## Roadmap / status

Core is done (docking on all edges, Google multi-calendar, ICS, detail popover, autostart,
auto-reconnect). See [PLAN.md](PLAN.md) for architecture details and the remaining polish
(tray icon, a full settings panel, per-account color UI, Microsoft Graph provider).

## License

[MIT](LICENSE).

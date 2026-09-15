# Agenda Strip

Faixa fina de agenda docada na borda inferior do desktop (Windows AppBar), mostrando o dia como timeline horizontal. Tauri v2 (Rust) + Svelte.

Ver **PLAN.md** para a arquitetura e o roadmap por fases.

## ⚠️ Ambiente

Este app é **Windows-only** (usa Win32 `SHAppBarMessage`). Build e execução têm que
ser no **Windows nativo** (PowerShell), não no WSL — no WSL o AppBar não existe.

## Pré-requisitos (Windows)

1. **Rust** (rustup): https://rustup.rs → `rustup-init.exe` → toolchain `stable-msvc`.
2. **Node.js** 20+ (com npm).
3. **WebView2** — já vem no Windows 11.
4. **Build tools do C++** (Visual Studio Build Tools, workload "Desktop C++").

## Rodar (dev)

No PowerShell, na raiz do projeto:

```powershell
npm install
npm run tauri dev
```

Isso sobe o Vite (porta 1420) e compila o Rust. A faixa aparece docada acima da taskbar
com os eventos mockados, o marcador de "agora" e a sobreposição em lanes.

> Nota: `npm install` deve rodar no **Windows** — os binários nativos (esbuild/rollup,
> tauri-cli) são específicos da plataforma. Não instale as deps pelo WSL.

## Testes

```powershell
npm test          # vitest: time.ts + layout.ts (lógica pura, sem Tauri)
```

## Build (bundle)

```powershell
npm run tauri build
```

Antes de bundlar, gere os ícones (precisa de um PNG grande):

```powershell
npm run tauri icon caminho/para/icon.png
```

## Estrutura

- `src/` — frontend Svelte (strip + detail popover).
  - `src/lib/` — `time.ts` (janela/posição), `layout.ts` (lanes), `types.ts`, `ipc.ts`, `mocks.ts`.
  - `src/strip/` — `Strip.svelte`, `EventBlock.svelte`, `NowMarker.svelte`.
  - `src/detail/` — `Detail.svelte` (Fase 2).
- `src-tauri/` — backend Rust.
  - `src/platform/appbar.rs` — Win32 AppBar isolado (`#[cfg(windows)]`).
  - `src/model.rs` — `NormalizedEvent` + mocks.
  - `app.manifest` — DPI awareness per-monitor v2.

## Status

**Fase 1** (prova de valor): faixa docada + render de mocks + marcador de agora + lanes. ✅
Fases 2–4 (ICS/SQLite/sync/popover, Google OAuth, polimento) — ver PLAN.md.

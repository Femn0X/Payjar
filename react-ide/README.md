# Pro IDE - React + Vite + Tauri (scaffold)

This repository is a minimal scaffold for a desktop IDE built with React + Vite on the frontend and Tauri (Rust) as the native shell.

What is included
- Vite + React app with a sidebar and Monaco editor (via @monaco-editor/react)
- Tauri `src-tauri` stub with `Cargo.toml`, `main.rs` and `tauri.conf.json`

Quick start

1) Install prerequisites
- Node.js (v18+ recommended)
- Rust toolchain (stable) and `cargo` (for Tauri)
- Tauri prerequisites (platform-specific) — see https://tauri.app/v1/guides/getting-started/prerequisites

2) From project root:

```bash
cd /home/lucawinecker/index/react-app
npm install
# To run the frontend in the browser (fast feedback):
npm run dev

# To run the Tauri desktop app (requires Rust & Tauri CLI):
# install tauri cli if not present: cargo install tauri-cli
npm run tauri:dev
```

Notes
- This is a scaffold. Many production features are missing (project tree management, file I/O, terminals, plugins, settings, packaging details).
- Next steps: wire filesystem access via Tauri commands, add file explorer with read/write, support terminals via pty, add multiple panes, and autosave.


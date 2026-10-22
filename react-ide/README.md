# Payjar Studio

Payjar Studio is the browser IDE inside \`react-ide/\`, built with React, Vite and Monaco Editor. The existing Tauri shell remains available for future desktop integration.

## Run locally

Requirements: Node.js 18 or newer and npm.

\`\`\`bash
cd react-ide
npm install
npm run dev
\`\`\`

Open the local URL printed by Vite (usually http://localhost:5173).

To build the browser app:

\`\`\`bash
npm run build
npm run preview
\`\`\`

The Tauri development command remains \`npm run tauri:dev\` and requires the Rust toolchain plus the platform-specific Tauri prerequisites.

## Included features

- Monaco editor with Payjar syntax highlighting and cursor position
- Multiple workspace files, tabs, file filtering, create and delete actions
- In-browser execution using the TypeScript Payjar lexer, parser and interpreter
- Terminal output and visible runtime errors
- Browser-local autosave through localStorage
- Import source files, export an individual file, and import/export a JSON workspace backup
- Responsive dark interface

## Workspace and runtime notes

- The browser IDE stores workspace text in the current browser profile. Export a workspace JSON backup to move it to another browser or device.
- Importing source files copies their text into the browser workspace; it does not connect to or modify a GitHub repository or local disk.
- Execution supports the syntax implemented by \`react-ide/payjar.ts\`. It is not a full operating-system terminal, and it cannot run arbitrary Rust or shell commands in the browser.
- \`readln(...)\` input is requested through the browser prompt dialog.
- Browser storage can be cleared by browser settings. Keep backups of important work.
- The Tauri shell and native filesystem integration are not wired into this web workspace yet.

## Project layout

- \`src/App.tsx\` — workspace UI and browser file management
- \`src/components/Editor.tsx\` — Monaco editor and Payjar execution panel
- \`src/syntax.ts\` — Monaco language tokenization and editor configuration
- \`payjar.ts\` — TypeScript lexer, parser and interpreter
- \`src-tauri/\` — existing desktop shell scaffold

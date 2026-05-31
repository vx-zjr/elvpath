# ElvPath

ElvPath is a Rust + Tauri desktop app for planning and deploying developer environments on Windows, macOS, and mainstream Linux distributions.

## Current milestone

- Tauri 2 + React/Vite GUI.
- Declarative catalog in `src-tauri/catalog/catalog.json`.
- Rust catalog validation, detection, installation-plan generation, dry-run execution wrappers, and Tauri commands.
- English and Simplified Chinese UI.
- Optional official source, proxy, or China mirror network profile.
- Seed catalog entries for languages, editors, containers, databases, DevOps, cloud, mobile, and AI/data tooling.

## Commands

```powershell
npm install
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

Generate the frontend contract file from the Rust type export helper:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --bin export_types
```

## Safety model

Installation plans show source, URL, checksum/signature expectation, permission requirements, commands, and configuration changes before execution. Live elevated installation should stay behind explicit user confirmation; the current milestone defaults to dry-run behavior from the GUI.


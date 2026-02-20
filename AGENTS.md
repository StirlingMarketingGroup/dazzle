# Repository Guidelines

## Project Structure & Module Organization

- Frontend: `src/` (React + TypeScript). Components in `src/components/` (PascalCase `.tsx`), state in `src/store/` (Zustand), types in `src/types/`.
- Styles: `src/index.css` with Tailwind; config in `tailwind.config.js` and `postcss.config.js`.
- Desktop backend: `src-tauri/` (Rust, Tauri). App entry `src-tauri/src/main.rs`, commands in `src-tauri/src/commands.rs`, config in `src-tauri/tauri.conf.json` and `src-tauri/capabilities/`.
- Build output: `dist/`. Do not commit artifacts.

## Build, Test, and Development Commands

- `npm run dev` — Start Vite dev server for the web UI.
- `npm run build` — Production build of the web assets to `dist/`.
- `npm run tauri dev` — Run the full desktop app (Tauri + frontend) in dev.
- `npm run tauri build` — Build a desktop binary via Tauri.
- Rust-only: `cd src-tauri && cargo build` (or `cargo check` for fast checks).

## Coding Style & Naming Conventions

- TypeScript strict mode is enabled; keep types precise. Prefer `@/` path alias over long relatives.
- Components: PascalCase (`PrinterList.tsx`). Hooks: `useX.ts`. Store files: `useSomethingStore.ts`. Types live in `src/types/index.ts`.
- Indentation: 2 spaces; favor functional components and hooks. Use Tailwind utility classes for styling.

## Commit & Pull Request Guidelines

- Use concise, typed prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`.
- PRs should include: clear description, linked issues, platform(s) tested (Windows/macOS/Linux).

## Development Process & Auto-Restart Behavior

- This project automatically restarts when Rust code changes are detected in `src-tauri/`.
- The frontend also has hot reload capabilities for React/TypeScript changes.
- **For agents**: DO NOT manually kill running processes or search for PIDs to restart the application.

## Security & Configuration Tips

- Respect Tauri capabilities; only request permissions you use (`src-tauri/capabilities/`).
- Avoid hardcoded user paths; use Tauri plugins (`@tauri-apps/plugin-os`) and Rust helpers.
- Never commit secrets; configuration belongs in environment or Tauri config.

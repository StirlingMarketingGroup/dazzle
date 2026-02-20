# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Primary Development Workflow

- `npm run tauri dev` - Start the full Tauri app with hot reload (frontend + backend)
- `npm run tauri build` - Build production desktop app for current platform
- `npm run dev` - Start Vite dev server for frontend-only development
- `npm run build` - Build frontend assets to `dist/`
- `cd src-tauri && cargo build` - Build Rust backend only
- `cd src-tauri && cargo check` - Fast compile check for Rust code

### Platform-Specific Notes

- macOS: Requires Xcode command line tools
- All platforms: Requires Rust 1.77+ and Node.js 18+

## Development Server Management

**IMPORTANT: DO NOT kill running development servers (`npm run tauri dev`)**

- The dev server has excellent hot-reload capabilities for both frontend and Rust changes
- Changes to TypeScript/React files reload instantly
- Changes to Rust files trigger automatic recompilation and app restart
- If you believe a manual restart is needed, ask the developer first

## Code Quality Verification

**ALWAYS run test builds to catch ALL errors and warnings before completing work:**

1. **Frontend Build**: `npm run build`
2. **Backend Build**: `cd src-tauri && cargo build`
3. **Full Production Build**: `npm run tauri build` (when appropriate)

## Refactoring Philosophy

**Be eager to rewrite and refactor. This is encouraged and expected.**

- **Slight awkwardness?** Consider refactoring.
- **Code feels patched together?** Rewrite it cleanly.
- **Adding a feature requires contortions?** Step back and redesign first.
- Delete code aggressively—less code is better code.

## Architecture Overview

### Technology Stack

- **Frontend**: React 19 + TypeScript + Tailwind CSS + Vite
- **Backend**: Rust + Tauri 2.0 + Tokio for async operations
- **State Management**: Zustand (`src/store/`)
- **HTTP Server**: Axum (for receiving ZPL print jobs)
- **Icons**: Phosphor React

### Key Concepts

- **Print Server**: HTTP server on configurable port (default 9100) accepts ZPL via POST
- **Printer Discovery**: Platform-specific printer enumeration
- **Raw Printing**: `lp -o raw` on Mac/Linux, raw spool API on Windows
- **System Tray**: Always-available tray icon with quick printer selection

### Key Backend Modules

- `commands.rs` - Tauri commands (printer listing, printing, config)
- `lib.rs` - App setup and plugin registration

### Key Frontend Architecture

- `App.tsx` - Main layout
- `store/` - Zustand state management
- `components/` - UI components

## File Structure Notes

### Frontend (`src/`)

- `components/` - UI components (PascalCase .tsx files)
- `hooks/` - Custom React hooks
- `store/` - Zustand state management
- `types/` - TypeScript type definitions
- `utils/` - Utility functions

### Backend (`src-tauri/src/`)

- `main.rs` - App entry point
- `lib.rs` - Library configuration and module declarations
- `commands.rs` - API commands (printer ops, config)

### Configuration Files

- `src-tauri/tauri.conf.json` - Tauri app configuration
- `package.json` - Frontend dependencies and scripts
- `src-tauri/Cargo.toml` - Rust dependencies
- `tailwind.config.js` - Tailwind CSS configuration

## Coding Conventions

### TypeScript/React

- Strict TypeScript mode enabled
- 2-space indentation
- PascalCase for components, camelCase for functions/variables
- Functional components with hooks
- Use `@/` path alias for imports

### Rust

- Follow Rust naming conventions (snake_case)
- Use `#[tauri::command]` for exposed functions
- Handle errors with proper Result types
- Use async/await for I/O operations

### Commit Messages

- Use prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`
- Keep first line under 50 characters
- Reference issues when applicable

# Dazzle

> _A group of zebras is called a "dazzle"_

A lightweight, open-source Tauri desktop app for printing ZPL directly to thermal label printers. Cross-platform (Mac, Windows, Linux) and free alternative to QZ Tray.

## Features

- Printer discovery and selection
- Raw ZPL printing to thermal/label printers
- HTTP server for receiving print jobs from web apps
- System tray with status indicator
- Auto-start on login (optional)
- Print queue/history view
- Configurable port

## Supported Printers

- Zebra printers (ZPL native)
- Bixolon SRP-770III (ZPL emulation)
- Any thermal printer with ZPL support

## API

```typescript
// From any web app / browser
await fetch('http://localhost:9100/print', {
  method: 'POST',
  headers: { 'Content-Type': 'text/plain' },
  body: zplContent,
});
```

## Tech Stack

- **Tauri 2.0** — Rust backend, tiny binary (~5MB)
- **React 19** — UI
- **Axum** — HTTP server for print jobs
- **Platform printing:**
  - Mac/Linux: `lp -d <printer> -o raw`
  - Windows: Raw spool API

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://www.rust-lang.org/tools/install) 1.77+
- Platform-specific Tauri dependencies ([see docs](https://v2.tauri.app/start/prerequisites/))

### Setup

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

## Related

- [Marlin](https://github.com/StirlingMarketingGroup/marlin) — Our Tauri file browser (same pattern)
- [go-zpl](https://github.com/StirlingMarketingGroup/go-zpl) — Our ZPL library

## License

MIT

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
- Configurable port (default: 29100)

## Supported Printers

- Zebra printers (ZPL native)
- Bixolon SRP-770III (ZPL emulation)
- Any thermal printer with ZPL support

## Client Library

Install the TypeScript client for easy integration:

```bash
npm install dazzle-zpl
```

```typescript
import { Dazzle } from 'dazzle-zpl';

const dazzle = new Dazzle();

// Check if Dazzle is running
if (await dazzle.isRunning()) {
  // Print a label
  await dazzle.print('^XA^FO50,50^A0N,50,50^FDHello World^FS^XZ');

  // List available printers
  const printers = await dazzle.printers();

  // Print to a specific printer
  await dazzle.print(zpl, { printer: 'Zebra_ZD420' });
}
```

Or use the HTTP API directly:

```typescript
await fetch('http://localhost:29100/print', {
  method: 'POST',
  body: zplContent,
});
```

### HTTP Endpoints

| Method | Path                  | Description                |
| ------ | --------------------- | -------------------------- |
| POST   | `/print`              | Send ZPL (body = raw text) |
| POST   | `/print?printer=NAME` | Send to a specific printer |
| GET    | `/printers`           | List available printers    |
| GET    | `/status`             | Health check / version     |

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

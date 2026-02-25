export interface DazzleOptions {
  /** Hostname of the Dazzle server. @default "localhost" */
  host?: string;
  /** Port of the Dazzle server. @default 29100 */
  port?: number;
  /** Protocol to use. @default "http" */
  protocol?: 'http' | 'https';
}

export interface Printer {
  name: string;
  is_default: boolean;
}

export interface PrintResult {
  job_id: string;
}

export interface ServerStatus {
  status: string;
  version: string;
}

export interface PrintOptions {
  /** Override the default printer for this job. */
  printer?: string;
}

export interface WatchOptions {
  /** Polling interval in milliseconds. @default 5000 */
  interval?: number;
}

/**
 * Base64-encode a Uint8Array for binary-safe transmission.
 * Uses btoa with a latin1 intermediate string.
 */
function toBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

export class Dazzle {
  private baseUrl: string;
  private watchers = new Set<(running: boolean) => void>();
  private pollTimer: ReturnType<typeof setInterval> | null = null;
  private pollInterval = 0;
  private lastStatus: boolean | null = null;

  constructor(options?: DazzleOptions) {
    const host = options?.host ?? 'localhost';
    const port = options?.port ?? 29100;
    const protocol = options?.protocol ?? 'http';
    this.baseUrl = `${protocol}://${host}:${port}`;
  }

  /**
   * Watch for server status changes. The callback fires immediately with
   * the current status, then again whenever the status changes.
   *
   * Returns an unwatch function. When all watchers unsubscribe, polling stops.
   *
   * ```ts
   * const unwatch = dazzle.watch((running) => {
   *   banner.classList.toggle('d-none', running);
   * });
   * // later: unwatch();
   * ```
   */
  watch(callback: (running: boolean) => void, options?: WatchOptions): () => void {
    const interval = options?.interval ?? 5000;
    this.watchers.add(callback);

    // Fire immediately with current known status, then poll
    this.isRunning().then((running) => {
      if (this.watchers.has(callback)) {
        callback(running);
        this.lastStatus = running;
      }
    });

    // Start or restart polling at the shortest requested interval
    if (this.pollTimer === null || interval < this.pollInterval) {
      this.startPolling(interval);
    }

    return () => {
      this.watchers.delete(callback);
      if (this.watchers.size === 0) {
        this.stopPolling();
      }
    };
  }

  private startPolling(interval: number): void {
    this.stopPolling();
    this.pollInterval = interval;
    this.pollTimer = setInterval(async () => {
      const running = await this.isRunning();
      if (running !== this.lastStatus) {
        this.lastStatus = running;
        for (const cb of this.watchers) {
          cb(running);
        }
      }
    }, interval);
  }

  private stopPolling(): void {
    if (this.pollTimer !== null) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
      this.pollInterval = 0;
      this.lastStatus = null;
    }
  }

  /**
   * Check if the Dazzle server is reachable.
   *
   * Uses `no-cors` mode to avoid noisy CORS console errors when
   * the server is not running.
   */
  async isRunning(): Promise<boolean> {
    try {
      // An opaque response (type "opaque") means the server responded.
      // A network error (TypeError) means it's unreachable.
      const res = await fetch(`${this.baseUrl}/status`, { mode: 'no-cors' });
      return res.type === 'opaque' || res.ok;
    } catch {
      return false;
    }
  }

  /** Get server status and version. */
  async status(): Promise<ServerStatus> {
    const res = await fetch(`${this.baseUrl}/status`);
    if (!res.ok) throw new DazzleError(`Server error: ${res.status}`, res.status);
    return res.json();
  }

  /** List available printers on the host machine. */
  async printers(): Promise<Printer[]> {
    const res = await fetch(`${this.baseUrl}/printers`);
    if (!res.ok) throw new DazzleError(`Server error: ${res.status}`, res.status);
    return res.json();
  }

  /**
   * Send ZPL to be printed.
   *
   * Accepts a `string` (ASCII ZPL), `Uint8Array`, or `ArrayBuffer`.
   * Binary data is automatically base64-encoded for safe transmission.
   * Strings are also base64-encoded by default to avoid any encoding issues.
   */
  async print(
    zpl: string | Uint8Array | ArrayBuffer,
    options?: PrintOptions
  ): Promise<PrintResult> {
    const url = new URL(`${this.baseUrl}/print`);
    url.searchParams.set('encoding', 'base64');
    if (options?.printer) {
      url.searchParams.set('printer', options.printer);
    }

    let bytes: Uint8Array;
    if (typeof zpl === 'string') {
      // Encode string as latin1 bytes to preserve ZPL binary data
      bytes = new Uint8Array(zpl.length);
      for (let i = 0; i < zpl.length; i++) {
        bytes[i] = zpl.charCodeAt(i) & 0xff;
      }
    } else if (zpl instanceof ArrayBuffer) {
      bytes = new Uint8Array(zpl);
    } else {
      bytes = zpl;
    }

    const res = await fetch(url, {
      method: 'POST',
      body: toBase64(bytes),
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      throw new DazzleError(body || `Print failed: ${res.status}`, res.status);
    }

    return res.json();
  }

  /**
   * Fetch a ZPL file from a URL and print it.
   *
   * Fetches as binary (ArrayBuffer) to preserve image data,
   * then base64-encodes and sends to the print server.
   */
  async printURL(url: string, options?: PrintOptions): Promise<PrintResult> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new DazzleError(`Failed to fetch ZPL from ${url}: ${response.status}`, response.status);
    }
    const buffer = await response.arrayBuffer();
    return this.print(buffer, options);
  }

  /**
   * Print multiple ZPL payloads sequentially to preserve print order.
   *
   * Each item can be a `string`, `Uint8Array`, or `ArrayBuffer`.
   */
  async printAll(
    items: (string | Uint8Array | ArrayBuffer)[],
    options?: PrintOptions
  ): Promise<PrintResult[]> {
    const results: PrintResult[] = [];
    for (const item of items) {
      results.push(await this.print(item, options));
    }
    return results;
  }

  /**
   * Fetch and print multiple ZPL files sequentially to preserve print order.
   */
  async printURLs(urls: string[], options?: PrintOptions): Promise<PrintResult[]> {
    const results: PrintResult[] = [];
    for (const url of urls) {
      results.push(await this.printURL(url, options));
    }
    return results;
  }
}

export class DazzleError extends Error {
  constructor(
    message: string,
    public readonly statusCode: number
  ) {
    super(message);
    this.name = 'DazzleError';
  }
}

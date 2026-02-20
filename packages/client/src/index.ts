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

export class Dazzle {
  private baseUrl: string;

  constructor(options?: DazzleOptions) {
    const host = options?.host ?? 'localhost';
    const port = options?.port ?? 29100;
    const protocol = options?.protocol ?? 'http';
    this.baseUrl = `${protocol}://${host}:${port}`;
  }

  /** Check if the Dazzle server is reachable. */
  async isRunning(): Promise<boolean> {
    try {
      await this.status();
      return true;
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
   * Uses the server's configured default printer unless
   * `options.printer` is specified.
   */
  async print(zpl: string, options?: PrintOptions): Promise<PrintResult> {
    const url = new URL(`${this.baseUrl}/print`);
    if (options?.printer) {
      url.searchParams.set('printer', options.printer);
    }

    const res = await fetch(url, { method: 'POST', body: zpl });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      throw new DazzleError(body || `Print failed: ${res.status}`, res.status);
    }

    return res.json();
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

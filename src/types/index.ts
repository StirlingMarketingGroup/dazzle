export interface Printer {
  name: string;
  is_default: boolean;
}

export interface PrintJob {
  id: string;
  printer: string;
  timestamp: number;
  status: 'printing' | 'completed' | 'failed';
  zpl_preview?: string;
  error?: string;
}

export interface AppConfig {
  port: number;
  selected_printer: string | null;
}

export type ServerStatus =
  | { state: 'starting' }
  | { state: 'serving' }
  | { state: 'standby' }
  | { state: 'failed'; error: string };

export type Autostart = 'off' | 'user' | 'all_users';

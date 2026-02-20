export interface Printer {
  name: string;
  is_default: boolean;
}

export interface PrintJob {
  id: string;
  printer: string;
  timestamp: number;
  status: 'pending' | 'printing' | 'completed' | 'failed';
  zpl_preview?: string;
  error?: string;
}

export interface AppConfig {
  port: number;
  selected_printer: string | null;
  auto_start: boolean;
}

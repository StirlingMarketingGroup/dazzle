import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Printer, PrintJob, AppConfig } from '@/types';

interface AppStore {
  printers: Printer[];
  config: AppConfig;
  printJobs: PrintJob[];
  serverRunning: boolean;
  serverError: string | null;
  loading: boolean;
  initError: string | null;

  init: () => Promise<void>;
  refreshPrinters: () => Promise<void>;
  updateConfig: (config: AppConfig) => Promise<void>;
  restartServer: () => Promise<void>;
}

let listeners: UnlistenFn[] = [];

export const useAppStore = create<AppStore>((set) => ({
  printers: [],
  config: { port: 29100, selected_printer: null },
  printJobs: [],
  serverRunning: false,
  serverError: null,
  loading: true,
  initError: null,

  init: async () => {
    // Clean up previous listeners (HMR safety)
    for (const unlisten of listeners) unlisten();
    listeners = [];

    try {
      const [printers, loadedConfig, printJobs, serverRunning] = await Promise.all([
        invoke<Printer[]>('list_printers'),
        invoke<AppConfig>('get_config'),
        invoke<PrintJob[]>('get_print_jobs'),
        invoke<boolean>('get_server_running'),
      ]);

      // Auto-select the system default printer if none is configured
      let config = loadedConfig;
      if (!config.selected_printer && printers.length > 0) {
        const defaultPrinter = printers.find((p) => p.is_default) ?? printers[0];
        config = { ...config, selected_printer: defaultPrinter.name };
        invoke('set_config', { newConfig: config }).catch(() => {});
      }

      set({ printers, config, printJobs, serverRunning, loading: false });
    } catch (e) {
      set({ loading: false, initError: String(e) });
    }

    listeners.push(
      await listen<PrintJob>('print-job', (event) => {
        set((state) => {
          const jobs = [...state.printJobs];
          const idx = jobs.findIndex((j) => j.id === event.payload.id);
          if (idx >= 0) {
            jobs[idx] = event.payload;
          } else {
            jobs.unshift(event.payload);
          }
          return { printJobs: jobs.slice(0, 100) };
        });
      })
    );

    listeners.push(
      await listen<boolean>('server-status', (event) => {
        set({
          serverRunning: event.payload,
          ...(event.payload ? { serverError: null } : {}),
        });
      })
    );

    listeners.push(
      await listen<string>('server-error', (event) => {
        set({ serverError: event.payload, serverRunning: false });
      })
    );
  },

  refreshPrinters: async () => {
    const printers = await invoke<Printer[]>('list_printers');
    set({ printers });
  },

  updateConfig: async (config: AppConfig) => {
    await invoke('set_config', { newConfig: config });
    set({ config });
  },

  restartServer: async () => {
    set({ serverError: null });
    try {
      await invoke('restart_server');
    } catch (e) {
      set({ serverError: String(e), serverRunning: false });
    }
  },
}));

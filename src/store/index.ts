import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Printer, PrintJob, AppConfig } from '@/types';

interface AppStore {
  printers: Printer[];
  config: AppConfig;
  printJobs: PrintJob[];
  serverRunning: boolean;
  loading: boolean;

  init: () => Promise<void>;
  refreshPrinters: () => Promise<void>;
  updateConfig: (config: AppConfig) => Promise<void>;
}

export const useAppStore = create<AppStore>((set) => ({
  printers: [],
  config: { port: 29100, selected_printer: null, auto_start: false },
  printJobs: [],
  serverRunning: false,
  loading: true,

  init: async () => {
    const [printers, config, printJobs] = await Promise.all([
      invoke<Printer[]>('list_printers'),
      invoke<AppConfig>('get_config'),
      invoke<PrintJob[]>('get_print_jobs'),
    ]);

    set({ printers, config, printJobs, loading: false });

    listen<PrintJob>('print-job', (event) => {
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
    });

    listen<boolean>('server-status', (event) => {
      set({ serverRunning: event.payload });
    });
  },

  refreshPrinters: async () => {
    const printers = await invoke<Printer[]>('list_printers');
    set({ printers });
  },

  updateConfig: async (config: AppConfig) => {
    await invoke('set_config', { newConfig: config });
    set({ config });
  },
}));

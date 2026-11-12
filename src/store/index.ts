import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Printer, PrintJob, AppConfig, ServerStatus, Autostart, PrinterChoice } from '@/types';

interface AppStore {
  printers: Printer[];
  config: AppConfig;
  printJobs: PrintJob[];
  serverStatus: ServerStatus;
  loading: boolean;
  initError: string | null;
  autostart: Autostart;
  printerChoice: PrinterChoice | null;

  init: () => Promise<void>;
  refresh: () => Promise<void>;
  updateConfig: (config: AppConfig) => Promise<void>;
  restartServer: () => Promise<void>;
  setAutostart: (enabled: boolean) => Promise<void>;
}

let listeners: UnlistenFn[] = [];

export const useAppStore = create<AppStore>((set) => ({
  printers: [],
  config: { port: 29100, selected_printer: null },
  printJobs: [],
  serverStatus: { state: 'starting' },
  loading: true,
  initError: null,
  autostart: 'off',
  printerChoice: null,

  init: async () => {
    // Clean up previous listeners (HMR safety)
    for (const unlisten of listeners) unlisten();
    listeners = [];

    // Register before the snapshot. The backend emits a status only when it
    // changes, so a change between the snapshot and a later listener would never show.
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
      await listen<ServerStatus>('server-status', (event) => {
        set({ serverStatus: event.payload });
      })
    );

    try {
      const [printers, config, printJobs, serverStatus, autostart, printerChoice] =
        await Promise.all([
          invoke<Printer[]>('list_printers'),
          invoke<AppConfig>('get_config'),
          invoke<PrintJob[]>('get_print_jobs'),
          invoke<ServerStatus>('get_server_status'),
          invoke<Autostart>('get_autostart'),
          invoke<PrinterChoice | null>('get_printer_choice'),
        ]);

      set({
        printers,
        config,
        printJobs,
        serverStatus,
        autostart,
        printerChoice,
        loading: false,
      });
    } catch (e) {
      set({ loading: false, initError: String(e) });
    }
  },

  // Re-reads what changes outside Dazzle: the printers Windows has, and the administrator's settings.
  refresh: async () => {
    const [printers, printerChoice, autostart] = await Promise.all([
      invoke<Printer[]>('list_printers'),
      invoke<PrinterChoice | null>('get_printer_choice'),
      invoke<Autostart>('get_autostart'),
    ]);
    set({ printers, printerChoice, autostart });
  },

  updateConfig: async (config: AppConfig) => {
    await invoke('set_config', { newConfig: config });
    set({ config });
    set({ printerChoice: await invoke<PrinterChoice | null>('get_printer_choice') });
  },

  restartServer: async () => {
    await invoke('restart_server');
  },

  setAutostart: async (enabled: boolean) => {
    await invoke('set_autostart', { enabled });
    set({ autostart: await invoke<Autostart>('get_autostart') });
  },
}));

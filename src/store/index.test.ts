import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from './index';
import type { Printer, AppConfig, PrintJob } from '@/types';

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

function resetStore() {
  useAppStore.setState({
    printers: [],
    config: { port: 29100, selected_printer: null },
    printJobs: [],
    serverRunning: false,
    serverError: null,
    loading: true,
    initError: null,
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe('useAppStore', () => {
  describe('initial state', () => {
    it('has correct defaults', () => {
      const state = useAppStore.getState();
      expect(state.printers).toEqual([]);
      expect(state.config).toEqual({
        port: 29100,
        selected_printer: null,
      });
      expect(state.printJobs).toEqual([]);
      expect(state.serverRunning).toBe(false);
      expect(state.serverError).toBeNull();
      expect(state.loading).toBe(true);
      expect(state.initError).toBeNull();
    });
  });

  describe('init', () => {
    const printers: Printer[] = [
      { name: 'Zebra ZD420', is_default: true },
      { name: 'Brother QL-800', is_default: false },
    ];

    const config: AppConfig = {
      port: 9100,
      selected_printer: 'Zebra ZD420',
    };

    const printJobs: PrintJob[] = [
      {
        id: 'abc123',
        printer: 'Zebra ZD420',
        timestamp: 1700000000,
        status: 'completed',
        zpl_preview: '^XA^FO50,50^FDHello^FS^XZ',
      },
    ];

    it('loads printers, config, jobs, and server status', async () => {
      mockInvoke
        .mockResolvedValueOnce(printers) // list_printers
        .mockResolvedValueOnce(config) // get_config
        .mockResolvedValueOnce(printJobs) // get_print_jobs
        .mockResolvedValueOnce(true); // get_server_running
      mockListen.mockResolvedValue(() => {});

      await useAppStore.getState().init();

      const state = useAppStore.getState();
      expect(state.loading).toBe(false);
      expect(state.initError).toBeNull();
      expect(state.printers).toEqual(printers);
      expect(state.config).toEqual(config);
      expect(state.printJobs).toEqual(printJobs);
      expect(state.serverRunning).toBe(true);
    });

    it('auto-selects default printer when none configured', async () => {
      const noSelectionConfig: AppConfig = {
        port: 29100,
        selected_printer: null,
      };

      mockInvoke
        .mockResolvedValueOnce(printers) // list_printers
        .mockResolvedValueOnce(noSelectionConfig) // get_config
        .mockResolvedValueOnce([]) // get_print_jobs
        .mockResolvedValueOnce(false) // get_server_running
        .mockResolvedValue(undefined); // set_config (fire-and-forget)
      mockListen.mockResolvedValue(() => {});

      await useAppStore.getState().init();

      const state = useAppStore.getState();
      expect(state.config.selected_printer).toBe('Zebra ZD420');
    });

    it('auto-selects first printer when no default exists', async () => {
      const printersNoDefault: Printer[] = [
        { name: 'Printer A', is_default: false },
        { name: 'Printer B', is_default: false },
      ];

      mockInvoke
        .mockResolvedValueOnce(printersNoDefault)
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce(false)
        .mockResolvedValue(undefined);
      mockListen.mockResolvedValue(() => {});

      await useAppStore.getState().init();

      expect(useAppStore.getState().config.selected_printer).toBe('Printer A');
    });

    it('sets initError on failure', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('backend down'));
      mockListen.mockResolvedValue(() => {});

      await useAppStore.getState().init();

      const state = useAppStore.getState();
      expect(state.loading).toBe(false);
      expect(state.initError).toContain('backend down');
    });

    it('registers event listeners for print-job, server-status, server-error', async () => {
      mockInvoke
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce(false);
      mockListen.mockResolvedValue(() => {});

      await useAppStore.getState().init();

      const listenedEvents = mockListen.mock.calls.map((call) => call[0]);
      expect(listenedEvents).toContain('print-job');
      expect(listenedEvents).toContain('server-status');
      expect(listenedEvents).toContain('server-error');
    });
  });

  describe('refreshPrinters', () => {
    it('updates the printer list', async () => {
      const newPrinters: Printer[] = [{ name: 'New Printer', is_default: true }];
      mockInvoke.mockResolvedValueOnce(newPrinters);

      await useAppStore.getState().refreshPrinters();

      expect(useAppStore.getState().printers).toEqual(newPrinters);
      expect(mockInvoke).toHaveBeenCalledWith('list_printers');
    });
  });

  describe('updateConfig', () => {
    it('saves config via invoke and updates state', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      const newConfig: AppConfig = {
        port: 8080,
        selected_printer: 'Test Printer',
      };

      await useAppStore.getState().updateConfig(newConfig);

      expect(mockInvoke).toHaveBeenCalledWith('set_config', { newConfig });
      expect(useAppStore.getState().config).toEqual(newConfig);
    });
  });

  describe('restartServer', () => {
    it('clears error and invokes restart_server', async () => {
      useAppStore.setState({ serverError: 'old error' });
      mockInvoke.mockResolvedValueOnce(undefined);

      await useAppStore.getState().restartServer();

      expect(useAppStore.getState().serverError).toBeNull();
      expect(mockInvoke).toHaveBeenCalledWith('restart_server');
    });

    it('sets error on failure', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('port in use'));

      await useAppStore.getState().restartServer();

      const state = useAppStore.getState();
      expect(state.serverError).toContain('port in use');
      expect(state.serverRunning).toBe(false);
    });
  });

  describe('print-job event handler', () => {
    it('adds new jobs to the front of the list', async () => {
      mockInvoke
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce(false);

      // Capture the print-job event handler
      let printJobHandler: ((event: { payload: PrintJob }) => void) | undefined;
      mockListen.mockImplementation(async (event: string, handler: unknown) => {
        if (event === 'print-job') {
          printJobHandler = handler as (event: { payload: PrintJob }) => void;
        }
        return () => {};
      });

      await useAppStore.getState().init();

      expect(printJobHandler).toBeDefined();

      const newJob: PrintJob = {
        id: 'job1',
        printer: 'Zebra',
        timestamp: 1700000000,
        status: 'printing',
      };

      printJobHandler!({ payload: newJob });

      expect(useAppStore.getState().printJobs[0]).toEqual(newJob);
    });

    it('updates existing jobs by id', async () => {
      const existingJob: PrintJob = {
        id: 'job1',
        printer: 'Zebra',
        timestamp: 1700000000,
        status: 'printing',
      };

      mockInvoke
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce([existingJob])
        .mockResolvedValueOnce(false);

      let printJobHandler: ((event: { payload: PrintJob }) => void) | undefined;
      mockListen.mockImplementation(async (event: string, handler: unknown) => {
        if (event === 'print-job') {
          printJobHandler = handler as (event: { payload: PrintJob }) => void;
        }
        return () => {};
      });

      await useAppStore.getState().init();

      const updatedJob: PrintJob = {
        ...existingJob,
        status: 'completed',
      };

      printJobHandler!({ payload: updatedJob });

      const jobs = useAppStore.getState().printJobs;
      expect(jobs).toHaveLength(1);
      expect(jobs[0].status).toBe('completed');
    });

    it('caps job list at 100 entries', async () => {
      // Pre-fill with 100 jobs
      const existingJobs: PrintJob[] = Array.from({ length: 100 }, (_, i) => ({
        id: `old-${i}`,
        printer: 'Zebra',
        timestamp: 1700000000 + i,
        status: 'completed' as const,
      }));

      mockInvoke
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce(existingJobs)
        .mockResolvedValueOnce(false);

      let printJobHandler: ((event: { payload: PrintJob }) => void) | undefined;
      mockListen.mockImplementation(async (event: string, handler: unknown) => {
        if (event === 'print-job') {
          printJobHandler = handler as (event: { payload: PrintJob }) => void;
        }
        return () => {};
      });

      await useAppStore.getState().init();

      printJobHandler!({
        payload: {
          id: 'new-job',
          printer: 'Zebra',
          timestamp: 1800000000,
          status: 'printing',
        },
      });

      const jobs = useAppStore.getState().printJobs;
      expect(jobs).toHaveLength(100);
      expect(jobs[0].id).toBe('new-job');
    });
  });
});

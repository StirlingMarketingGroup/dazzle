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
    serverStatus: { state: 'starting' },
    loading: true,
    initError: null,
    autostart: 'off',
    policyPrinter: null,
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
      expect(state.serverStatus).toEqual({ state: 'starting' });
      expect(state.autostart).toBe('off');
      expect(state.policyPrinter).toBeNull();
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

    it('loads printers, config, jobs, server status, and autostart', async () => {
      mockInvoke
        .mockResolvedValueOnce(printers) // list_printers
        .mockResolvedValueOnce(config) // get_config
        .mockResolvedValueOnce(printJobs) // get_print_jobs
        .mockResolvedValueOnce({ state: 'serving' }) // get_server_status
        .mockResolvedValueOnce('user') // get_autostart
        .mockResolvedValueOnce('Zebra ZD420'); // get_policy_printer
      mockListen.mockResolvedValue(() => {});

      await useAppStore.getState().init();

      const state = useAppStore.getState();
      expect(state.loading).toBe(false);
      expect(state.initError).toBeNull();
      expect(state.printers).toEqual(printers);
      expect(state.config).toEqual(config);
      expect(state.printJobs).toEqual(printJobs);
      expect(state.serverStatus).toEqual({ state: 'serving' });
      expect(state.autostart).toBe('user');
      expect(state.policyPrinter).toBe('Zebra ZD420');
      expect(mockInvoke).toHaveBeenCalledWith('get_server_status');
      expect(mockInvoke).toHaveBeenCalledWith('get_policy_printer');
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
        .mockResolvedValueOnce({ state: 'starting' }) // get_server_status
        .mockResolvedValueOnce('off') // get_autostart
        .mockResolvedValueOnce(null) // get_policy_printer
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
        .mockResolvedValueOnce({ state: 'starting' })
        .mockResolvedValueOnce('off')
        .mockResolvedValueOnce(null)
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

    it('registers event listeners before loading the snapshot', async () => {
      mockInvoke
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ state: 'starting' })
        .mockResolvedValueOnce('off')
        .mockResolvedValueOnce(null);
      mockListen.mockResolvedValue(() => {});

      await useAppStore.getState().init();

      const listenedEvents = mockListen.mock.calls.map((call) => call[0]);
      expect(listenedEvents).toContain('print-job');
      expect(listenedEvents).toContain('server-status');
      expect(mockListen.mock.invocationCallOrder[0]).toBeLessThan(
        mockInvoke.mock.invocationCallOrder[0]
      );
    });

    it('stores server-status payloads', async () => {
      mockInvoke
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ state: 'starting' })
        .mockResolvedValueOnce('off')
        .mockResolvedValueOnce(null);

      let statusHandler: ((event: { payload: { state: 'standby' } }) => void) | undefined;
      mockListen.mockImplementation(async (event: string, handler: unknown) => {
        if (event === 'server-status') {
          statusHandler = handler as (event: { payload: { state: 'standby' } }) => void;
        }
        return () => {};
      });

      await useAppStore.getState().init();

      statusHandler!({ payload: { state: 'standby' } });

      expect(useAppStore.getState().serverStatus).toEqual({ state: 'standby' });
    });
  });

  describe('refresh', () => {
    it('updates the printer list, the policy printer and launch at login', async () => {
      const newPrinters: Printer[] = [{ name: 'New Printer', is_default: true }];
      mockInvoke
        .mockResolvedValueOnce(newPrinters)
        .mockResolvedValueOnce('Front Desk')
        .mockResolvedValueOnce('all_users');

      await useAppStore.getState().refresh();

      expect(useAppStore.getState().printers).toEqual(newPrinters);
      expect(useAppStore.getState().policyPrinter).toBe('Front Desk');
      expect(useAppStore.getState().autostart).toBe('all_users');
      expect(mockInvoke).toHaveBeenCalledWith('list_printers');
      expect(mockInvoke).toHaveBeenCalledWith('get_policy_printer');
      expect(mockInvoke).toHaveBeenCalledWith('get_autostart');
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
    it('invokes restart_server', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await useAppStore.getState().restartServer();

      expect(mockInvoke).toHaveBeenCalledWith('restart_server');
    });
  });

  describe('setAutostart', () => {
    it('enables autostart via invoke and updates state', async () => {
      mockInvoke.mockResolvedValueOnce(undefined).mockResolvedValueOnce('user');

      await useAppStore.getState().setAutostart(true);

      expect(mockInvoke).toHaveBeenCalledWith('set_autostart', { enabled: true });
      expect(useAppStore.getState().autostart).toBe('user');
    });

    it('disables autostart via invoke and updates state', async () => {
      useAppStore.setState({ autostart: 'user' });
      mockInvoke.mockResolvedValueOnce(undefined).mockResolvedValueOnce('off');

      await useAppStore.getState().setAutostart(false);

      expect(mockInvoke).toHaveBeenCalledWith('set_autostart', { enabled: false });
      expect(useAppStore.getState().autostart).toBe('off');
    });
  });

  describe('print-job event handler', () => {
    it('adds new jobs to the front of the list', async () => {
      mockInvoke
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ port: 29100, selected_printer: null })
        .mockResolvedValueOnce([])
        .mockResolvedValueOnce({ state: 'starting' })
        .mockResolvedValueOnce('off')
        .mockResolvedValueOnce(null);

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
        .mockResolvedValueOnce({ state: 'starting' })
        .mockResolvedValueOnce('off')
        .mockResolvedValueOnce(null);

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
        .mockResolvedValueOnce({ state: 'starting' })
        .mockResolvedValueOnce('off')
        .mockResolvedValueOnce(null);

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

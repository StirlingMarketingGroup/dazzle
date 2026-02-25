import { useEffect, useState } from 'react';
import { ArrowClockwise, Play, WarningCircle } from 'phosphor-react';
import { useAppStore } from '@/store';

export default function ServerStatus() {
  const serverRunning = useAppStore((s) => s.serverRunning);
  const serverError = useAppStore((s) => s.serverError);
  const config = useAppStore((s) => s.config);
  const updateConfig = useAppStore((s) => s.updateConfig);
  const restartServer = useAppStore((s) => s.restartServer);
  const [port, setPort] = useState(config.port.toString());

  // Sync port input when config changes externally
  useEffect(() => {
    setPort(config.port.toString());
  }, [config.port]);
  const [saving, setSaving] = useState(false);
  const [restarting, setRestarting] = useState(false);

  const applyPort = async () => {
    const newPort = parseInt(port, 10);
    if (!newPort || newPort === config.port || newPort < 1 || newPort > 65535) return;
    setSaving(true);
    try {
      await updateConfig({ ...config, port: newPort });
    } finally {
      setSaving(false);
    }
  };

  const handleRestart = async () => {
    setRestarting(true);
    try {
      await restartServer();
    } finally {
      setRestarting(false);
    }
  };

  return (
    <div className="px-4 py-3 border-b">
      <div className="flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${serverRunning ? 'bg-app-green' : 'bg-app-red'}`} />
        <span className="text-sm">{serverRunning ? 'Server running' : 'Server stopped'}</span>
        <button
          onClick={handleRestart}
          disabled={restarting}
          className="ml-auto flex items-center gap-1.5 px-2.5 py-1 text-xs rounded bg-app-dark hover:bg-app-lighter transition-colors disabled:opacity-50"
          title={serverRunning ? 'Restart server' : 'Start server'}
        >
          {serverRunning ? (
            <ArrowClockwise size={14} className={restarting ? 'animate-spin' : ''} />
          ) : (
            <Play size={14} weight="fill" />
          )}
          {restarting ? 'Starting…' : serverRunning ? 'Restart' : 'Start'}
        </button>
      </div>

      {serverError && (
        <div className="flex items-start gap-2 mt-2 px-2 py-1.5 rounded bg-red-950/40 text-red-400 text-xs">
          <WarningCircle size={14} className="shrink-0 mt-0.5" weight="fill" />
          <span className="break-all">{serverError}</span>
        </div>
      )}

      <p className="text-xs text-app-muted mt-2">
        Local print server that receives ZPL from your browser and sends it to your thermal printer.
        {' '}
        <a
          href="https://github.com/StirlingMarketingGroup/dazzle"
          target="_blank"
          rel="noopener noreferrer"
          className="text-app-accent hover:underline"
        >
          GitHub
        </a>
      </p>

      <div className="flex items-center gap-3 mt-2">
        <div className="flex items-center gap-2">
          <label className="text-xs text-app-muted">Port</label>
          <input
            className="input-field w-20 text-sm tabular-nums"
            value={port}
            onChange={(e) => setPort(e.target.value.replace(/\D/g, ''))}
            onBlur={applyPort}
            onKeyDown={(e) => e.key === 'Enter' && applyPort()}
            maxLength={5}
          />
          {saving && <span className="text-xs text-app-muted">Restarting…</span>}
        </div>
        <label className="ml-auto flex items-center gap-1.5 text-xs text-app-muted cursor-pointer select-none">
          <input
            type="checkbox"
            checked={config.auto_start}
            onChange={(e) => updateConfig({ ...config, auto_start: e.target.checked })}
            className="accent-app-accent"
          />
          Launch at login
        </label>
      </div>
    </div>
  );
}

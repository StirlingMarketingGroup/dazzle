import { useState } from 'react';
import { useAppStore } from '@/store';

export default function ServerStatus() {
  const serverRunning = useAppStore((s) => s.serverRunning);
  const config = useAppStore((s) => s.config);
  const updateConfig = useAppStore((s) => s.updateConfig);
  const [port, setPort] = useState(config.port.toString());
  const [saving, setSaving] = useState(false);

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

  return (
    <div className="px-4 py-3 border-b">
      <div className="flex items-center gap-2">
        <div className={`w-2 h-2 rounded-full ${serverRunning ? 'bg-app-green' : 'bg-app-red'}`} />
        <span className="text-sm">{serverRunning ? 'Server running' : 'Server stopped'}</span>
      </div>
      <div className="flex items-center gap-2 mt-2">
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
    </div>
  );
}

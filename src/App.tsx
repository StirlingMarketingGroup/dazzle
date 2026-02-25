import { useEffect, useState } from 'react';
import { platform } from '@tauri-apps/plugin-os';
import { useAppStore } from '@/store';
import TitleBar from '@/components/TitleBar';
import ServerStatus from '@/components/ServerStatus';
import PrinterSelect from '@/components/PrinterSelect';
import PrintLog from '@/components/PrintLog';

export default function App() {
  const init = useAppStore((s) => s.init);
  const loading = useAppStore((s) => s.loading);
  const initError = useAppStore((s) => s.initError);
  const [isMac, setIsMac] = useState(false);

  useEffect(() => {
    setIsMac(platform() === 'macos');
    init();
  }, [init]);

  if (loading) {
    return (
      <div className="h-screen bg-app-darker flex items-center justify-center">
        <span className="text-sm text-app-muted">Loading…</span>
      </div>
    );
  }

  if (initError) {
    return (
      <div className="h-screen bg-app-darker flex flex-col items-center justify-center gap-3 p-8">
        <span className="text-sm text-red-400">Failed to initialize</span>
        <span className="text-xs text-app-muted text-center break-all">{initError}</span>
        <button
          onClick={() => init()}
          className="mt-2 px-3 py-1.5 text-xs rounded bg-app-dark hover:bg-app-lighter transition-colors"
        >
          Retry
        </button>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-app-darker">
      {isMac && <TitleBar />}
      <div className="flex-1 overflow-y-auto flex flex-col">
        <ServerStatus />
        <PrinterSelect />
        <PrintLog />
      </div>
    </div>
  );
}

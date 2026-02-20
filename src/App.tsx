import { useEffect } from 'react';
import { useAppStore } from '@/store';
import TitleBar from '@/components/TitleBar';
import ServerStatus from '@/components/ServerStatus';
import PrinterSelect from '@/components/PrinterSelect';
import PrintLog from '@/components/PrintLog';

export default function App() {
  const init = useAppStore((s) => s.init);
  const loading = useAppStore((s) => s.loading);

  useEffect(() => {
    init();
  }, [init]);

  if (loading) {
    return (
      <div className="h-screen bg-app-darker flex items-center justify-center">
        <span className="text-sm text-app-muted">Loading…</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-app-darker">
      <TitleBar />
      <div className="flex-1 overflow-y-auto flex flex-col">
        <ServerStatus />
        <PrinterSelect />
        <PrintLog />
      </div>
    </div>
  );
}

import { ArrowClockwise, Printer } from 'phosphor-react';
import { useAppStore } from '@/store';

export default function PrinterSelect() {
  const printers = useAppStore((s) => s.printers);
  const config = useAppStore((s) => s.config);
  const updateConfig = useAppStore((s) => s.updateConfig);
  const refreshPrinters = useAppStore((s) => s.refreshPrinters);

  const selectPrinter = (name: string) => {
    updateConfig({ ...config, selected_printer: name });
  };

  return (
    <div className="px-4 py-3 border-b">
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-xs font-semibold tracking-widest text-app-muted uppercase">Printers</h2>
        <button className="btn-icon" onClick={refreshPrinters} title="Refresh printer list">
          <ArrowClockwise size={14} />
        </button>
      </div>

      {printers.length === 0 ? (
        <p className="text-sm text-app-muted py-2">No printers found</p>
      ) : (
        <div className="space-y-0.5">
          {printers.map((p) => {
            const selected = config.selected_printer === p.name;
            return (
              <button
                key={p.name}
                onClick={() => selectPrinter(p.name)}
                className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors text-left ${
                  selected ? 'bg-app-accent/15 text-app-accent' : 'hover:bg-app-light text-app-text'
                }`}
              >
                <Printer size={16} weight={selected ? 'fill' : 'regular'} />
                <span className="truncate">{p.name}</span>
                {p.is_default && (
                  <span className="text-[10px] text-app-muted ml-auto shrink-0">default</span>
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

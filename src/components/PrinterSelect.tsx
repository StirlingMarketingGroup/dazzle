import { ArrowClockwise, MagicWand, Printer } from 'phosphor-react';
import { useAppStore } from '@/store';

export default function PrinterSelect() {
  const printers = useAppStore((s) => s.printers);
  const config = useAppStore((s) => s.config);
  const printerChoice = useAppStore((s) => s.printerChoice);
  const updateConfig = useAppStore((s) => s.updateConfig);
  const refresh = useAppStore((s) => s.refresh);
  const policyChoice = printerChoice?.source === 'policy' ? printerChoice : null;
  const automatic = printerChoice?.source === 'automatic';

  const selectPrinter = (name: string) => {
    updateConfig({ ...config, selected_printer: name });
  };

  return (
    <div className="px-4 py-3 border-b">
      <div className="flex items-center justify-between mb-2">
        <h2 className="text-xs font-semibold tracking-widest text-app-muted uppercase">Printers</h2>
        <button className="btn-icon" onClick={refresh} title="Refresh printer list">
          <ArrowClockwise size={14} />
        </button>
      </div>

      {policyChoice &&
        (printers.some((printer) => printer.name === policyChoice.name) ? (
          <p className="text-xs text-app-muted mb-2">
            Your administrator chose this PC&apos;s printer.
          </p>
        ) : (
          <p className="text-xs text-red-400 mb-2">
            Your administrator chose the printer &quot;{policyChoice.name}&quot;, but it is not
            installed for this user.
          </p>
        ))}

      {printers.length === 0 ? (
        <p className="text-sm text-app-muted py-2">No printers found</p>
      ) : (
        <div className="space-y-0.5">
          {policyChoice === null && (
            <button
              onClick={() => updateConfig({ ...config, selected_printer: null })}
              title={automatic ? printerChoice.name : undefined}
              className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors text-left ${
                automatic
                  ? 'bg-app-accent/15 text-app-accent'
                  : 'enabled:hover:bg-app-light text-app-text'
              }`}
            >
              <MagicWand size={16} weight={automatic ? 'fill' : 'regular'} className="shrink-0" />
              <span className="shrink-0">Automatic</span>
              {automatic && (
                <span className="text-[10px] text-app-muted ml-auto truncate">
                  {printerChoice.name}
                </span>
              )}
            </button>
          )}
          {printers.map((p) => {
            const selected = !automatic && printerChoice?.name === p.name;
            return (
              <button
                key={p.name}
                onClick={() => selectPrinter(p.name)}
                disabled={policyChoice !== null}
                className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors text-left ${
                  selected
                    ? 'bg-app-accent/15 text-app-accent'
                    : 'enabled:hover:bg-app-light text-app-text'
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

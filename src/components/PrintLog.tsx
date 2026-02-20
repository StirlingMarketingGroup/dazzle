import { CheckCircle, XCircle, CircleNotch } from 'phosphor-react';
import { useAppStore } from '@/store';

export default function PrintLog() {
  const printJobs = useAppStore((s) => s.printJobs);

  return (
    <div className="px-4 py-3 flex-1 min-h-0">
      <h2 className="text-xs font-semibold tracking-widest text-app-muted uppercase mb-2">
        Recent Jobs
      </h2>

      {printJobs.length === 0 ? (
        <p className="text-sm text-app-muted py-2">
          No print jobs yet. Send ZPL to the server to get started.
        </p>
      ) : (
        <div className="space-y-0.5 overflow-y-auto">
          {printJobs.map((job) => (
            <div
              key={job.id}
              className="flex items-center gap-2.5 px-3 py-2 rounded-md bg-app-gray text-sm"
              title={job.error ?? job.zpl_preview ?? undefined}
            >
              <StatusIcon status={job.status} />
              <span className="truncate flex-1">{job.printer}</span>
              <span className="text-[11px] text-app-muted tabular-nums shrink-0">
                {new Date(job.timestamp * 1000).toLocaleTimeString()}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function StatusIcon({ status }: { status: string }) {
  switch (status) {
    case 'completed':
      return <CheckCircle size={16} weight="fill" className="text-app-green shrink-0" />;
    case 'failed':
      return <XCircle size={16} weight="fill" className="text-app-red shrink-0" />;
    default:
      return <CircleNotch size={16} className="text-app-yellow shrink-0 animate-spin" />;
  }
}

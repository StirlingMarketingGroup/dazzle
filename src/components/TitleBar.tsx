export default function TitleBar() {
  return (
    <div
      data-tauri-drag-region
      className="h-10 flex items-center pl-24 pr-4 bg-app-darker border-b shrink-0"
    >
      <span
        data-tauri-drag-region
        className="text-xs font-semibold tracking-widest text-app-muted uppercase"
      >
        Dazzle
      </span>
    </div>
  );
}

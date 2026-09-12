import { useLayoutEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { Search } from "lucide-react";
import type { DashboardFile } from "../types/file";
import { FileCard } from "./FileCard";
import { previewKey, useMediaPreviews } from "../hooks/useMediaPreviews";

interface FileGridProps { files:DashboardFile[]; selectable:boolean; selectedIds:Set<string>; onToggleSelection:(id:string)=>void; onOpen:(index:number)=>void; }

export function FileGrid({ files, selectable, selectedIds, onToggleSelection, onOpen }:FileGridProps) {
  const gridRef = useRef<HTMLDivElement>(null);
  const updateRef = useRef<() => void>(() => {});
  const [layout, setLayout] = useState({ columns: 1, height: 210, gap: 16, first: 0, last: 0 });
  useLayoutEffect(() => {
    const grid = gridRef.current;
    const pane = grid?.closest<HTMLElement>(".results-pane");
    if (!grid || !pane) return;
    let frame = 0;
    const update = () => {
      const style = getComputedStyle(grid);
      const columns = Math.max(1, style.gridTemplateColumns.split(" ").length);
      const height = parseFloat(style.getPropertyValue("--file-row-height"));
      const gap = parseFloat(style.rowGap) || 0;
      const rows = Math.ceil(files.length / columns);
      const top = pane.getBoundingClientRect().top + pane.clientTop - grid.getBoundingClientRect().top;
      const first = Math.min(Math.max(0, rows - 1), Math.max(0, Math.floor(top / (height + gap))));
      const last = Math.min(rows, Math.max(first + 1, Math.ceil((top + pane.clientHeight) / (height + gap))));
      setLayout(previous => previous.columns === columns && previous.height === height && previous.gap === gap && previous.first === first && previous.last === last
        ? previous : { columns, height, gap, first, last });
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(() => { frame = 0; update(); });
    };
    updateRef.current = update;
    update();
    const observer = new ResizeObserver(schedule);
    observer.observe(grid);
    observer.observe(pane);
    // Toolbar/warning height changes can move the grid without resizing it.
    for (const child of pane.children) observer.observe(child);
    pane.addEventListener("scroll", schedule, { passive: true });
    window.addEventListener("resize", schedule);
    return () => {
      observer.disconnect();
      pane.removeEventListener("scroll", schedule);
      window.removeEventListener("resize", schedule);
      cancelAnimationFrame(frame);
    };
  }, [files]);
  const visibleStart = Math.min(files.length, layout.first * layout.columns);
  const visibleEnd = Math.min(files.length, Math.max(layout.last, 1) * layout.columns);
  const previews = useMediaPreviews(files, visibleStart, visibleEnd);
  if(!files.length)return <div className="empty-state"><Search/><strong>No files found</strong><span>Try a different search term.</span></div>;
  const rows = Math.ceil(files.length / layout.columns);
  const start = visibleStart;
  const end = visibleEnd;
  return <div ref={gridRef} className="file-grid virtual-file-grid"
    style={{ height: rows * (layout.height + layout.gap) - layout.gap, gridAutoRows: layout.height }}
    onKeyDown={event => {
      const item = (event.target as HTMLElement).closest<HTMLElement>("[data-file-index]");
      if (!item) return;
      const index = Number(item.dataset.fileIndex);
      const delta = event.key === "Tab" ? (event.shiftKey ? -1 : 1)
        : event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1
        : event.key === "ArrowDown" ? layout.columns : event.key === "ArrowUp" ? -layout.columns : 0;
      const target = index + delta;
      if (!delta || target < 0 || target >= files.length) return;
      event.preventDefault();
      const grid = gridRef.current!;
      const pane = grid.closest<HTMLElement>(".results-pane")!;
      const rowTop = grid.getBoundingClientRect().top - pane.getBoundingClientRect().top - pane.clientTop + Math.floor(target / layout.columns) * (layout.height + layout.gap);
      if (rowTop < 0) pane.scrollTop += rowTop;
      else if (rowTop + layout.height > pane.clientHeight) pane.scrollTop += rowTop + layout.height - pane.clientHeight;
      flushSync(() => updateRef.current());
      grid.querySelector<HTMLElement>(`[data-file-index="${target}"] [role="button"]`)?.focus({ preventScroll: true });
    }}>
    {files.slice(start, end).map((file, offset) => {
      const index = start + offset;
      return <div key={file.id} data-file-index={index} style={{ gridRow: Math.floor(index / layout.columns) + 1, gridColumn: index % layout.columns + 1, minWidth: 0 }}>
        <FileCard file={file} previewUrl={previews.get(previewKey(file))} selectable={selectable} selected={selectedIds.has(String(file.id))} onToggleSelection={() => onToggleSelection(String(file.id))} onOpen={() => onOpen(index)}/>
      </div>;
    })}
  </div>;
}

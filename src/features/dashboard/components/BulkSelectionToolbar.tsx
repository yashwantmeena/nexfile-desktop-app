import { useEffect, useRef } from "react";
import { Check, MoreHorizontal, Star, Trash2, X } from "lucide-react";

interface BulkSelectionToolbarProps {
  count:number;
  busy:boolean;
  error:string;
  onDelete:()=>void;
  onFavorite:()=>void;
  favoriteLabel:string;
  onClear:()=>void;
}

export function BulkSelectionToolbar({ count, busy, error, onDelete, onFavorite, favoriteLabel, onClear }:BulkSelectionToolbarProps) {
  const menuRef = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    const closeOnOutsideClick = (event:PointerEvent) => {
      const menu = menuRef.current;
      if (menu?.open && event.target instanceof Node && !menu.contains(event.target)) menu.open = false;
    };
    document.addEventListener("pointerdown", closeOnOutsideClick);
    return () => document.removeEventListener("pointerdown", closeOnOutsideClick);
  }, []);
  return <div className="bulk-selection-toolbar" aria-label="Bulk file actions">
    <div className="bulk-selection-summary"><span className="bulk-selection-check"><Check /></span><strong>{count} {count === 1 ? "file" : "files"} selected</strong></div>
    <details ref={menuRef} className="bulk-action-menu">
      <summary aria-label="More bulk actions" title="More bulk actions"><MoreHorizontal /></summary>
      <div className="bulk-action-options" role="menu">
        <button type="button" role="menuitem" className="bulk-delete-button" disabled={busy} onClick={onDelete}><Trash2 />{busy ? "Moving…" : "Move to Trash"}</button>
        <button type="button" role="menuitem" className="bulk-favorite-button" disabled={busy} onClick={onFavorite}><Star />{favoriteLabel}</button>
        <button type="button" role="menuitem" className="bulk-clear-menu-button" disabled={busy} onClick={onClear}><X />Clear selection</button>
      </div>
    </details>
    {error && <span className="bulk-selection-error" role="alert">{error}</span>}
  </div>;
}

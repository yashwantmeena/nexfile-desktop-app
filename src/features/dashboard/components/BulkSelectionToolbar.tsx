import { useEffect, useRef, useState } from "react";
import { Check, ChevronLeft, FolderPlus, MoreHorizontal, Star, Tag, Trash2, X } from "lucide-react";
import type { Collection } from "@/features/collections/api/collections";

interface BulkSelectionToolbarProps {
  count:number;
  busy:boolean;
  error:string;
  onDelete:()=>void;
  onFavorite:()=>void;
  favoriteLabel:string;
  collections:Collection[];
  onAddToCollection:(collectionName:string)=>void;
  onAddTag:(tag:string)=>void;
  onClear:()=>void;
}

export function BulkSelectionToolbar({ count, busy, error, onDelete, onFavorite, favoriteLabel, collections, onAddToCollection, onAddTag, onClear }:BulkSelectionToolbarProps) {
  const menuRef = useRef<HTMLDetailsElement>(null);
  const [panel, setPanel] = useState<"actions"|"collection"|"tag">("actions");
  const [tag, setTag] = useState("");
  useEffect(() => {
    const closeOnOutsideClick = (event:PointerEvent) => {
      const menu = menuRef.current;
      if (menu?.open && event.target instanceof Node && !menu.contains(event.target)) menu.open = false;
    };
    document.addEventListener("pointerdown", closeOnOutsideClick);
    return () => document.removeEventListener("pointerdown", closeOnOutsideClick);
  }, []);
  const closeMenu = () => {
    if (menuRef.current) menuRef.current.open = false;
    setPanel("actions");
    setTag("");
  };
  const submitTag = () => {
    const value = tag.trim();
    if (!value || busy) return;
    closeMenu();
    onAddTag(value);
  };
  return <div className="bulk-selection-toolbar" aria-label="Bulk file actions">
    <div className="bulk-selection-summary"><span className="bulk-selection-check"><Check /></span><strong>{count} {count === 1 ? "file" : "files"} selected</strong></div>
    <details ref={menuRef} className="bulk-action-menu" onToggle={()=>{if(!menuRef.current?.open){setPanel("actions");setTag("");}}}>
      <summary aria-label="More bulk actions" title="More bulk actions"><MoreHorizontal /></summary>
      <div className="bulk-action-options" role="menu">
        {panel === "collection" ? <>
          <button type="button" role="menuitem" className="bulk-panel-back" disabled={busy} onClick={()=>setPanel("actions")}><ChevronLeft />Choose collection</button>
          <div className="bulk-collection-options">
            {collections.map(collection=><button type="button" role="menuitem" key={collection.id} disabled={busy} title={collection.name} onClick={()=>{closeMenu();onAddToCollection(collection.name);}}><FolderPlus /><span>{collection.name}</span></button>)}
          </div>
        </> : panel === "tag" ? <>
          <button type="button" role="menuitem" className="bulk-panel-back" disabled={busy} onClick={()=>setPanel("actions")}><ChevronLeft />Add tag</button>
          <form className="bulk-tag-form" onSubmit={event=>{event.preventDefault();submitTag();}}>
            <input autoFocus value={tag} maxLength={80} disabled={busy} aria-label="Tag to add" placeholder="Enter one tag" onChange={event=>setTag(event.target.value)} />
            <button type="submit" disabled={busy || !tag.trim()}><Tag />Add tag</button>
          </form>
        </> : <>
          <button type="button" role="menuitem" className="bulk-delete-button" disabled={busy} onClick={onDelete}><Trash2 />{busy ? "Moving…" : "Move to Trash"}</button>
          <button type="button" role="menuitem" className="bulk-favorite-button" disabled={busy} onClick={onFavorite}><Star />{favoriteLabel}</button>
          <button type="button" role="menuitem" className="bulk-collection-button" disabled={busy || !collections.length} onClick={()=>setPanel("collection")}><FolderPlus />{collections.length ? "Add to collection" : "No collections"}</button>
          <button type="button" role="menuitem" className="bulk-tag-button" disabled={busy} onClick={()=>setPanel("tag")}><Tag />Add tag</button>
          <button type="button" role="menuitem" className="bulk-clear-menu-button" disabled={busy} onClick={onClear}><X />Clear selection</button>
        </>}
      </div>
    </details>
    {error && <span className="bulk-selection-error" role="alert">{error}</span>}
  </div>;
}

import { useEffect, useState } from "react";
import { Popover } from "radix-ui";
import { Check, FileStack, FolderOpen, Plus, Search, X } from "lucide-react";
import type { Collection } from "@/features/collections/api/collections";
import type { ImportSelection } from "@/features/import/services/import_service";

interface ImportPreviewDialogProps {
  selection: ImportSelection;
  count: number | null;
  counting: boolean;
  error?: string;
  collections: Collection[];
  busy: boolean;
  onClose: () => void;
  onConfirm: (collectionIds: string[]) => void;
}

function displayName(path: string, folder: boolean) {
  const normalized = path.replace(/[\\/]$/, "");
  const name = normalized.split(/[\\/]/).pop();
  return name || (folder ? "Selected folder" : "Selected files");
}

export function ImportPreviewDialog({ selection, count, counting, error, collections, busy, onClose, onConfirm }: ImportPreviewDialogProps) {
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [collectionMenuOpen, setCollectionMenuOpen] = useState(false);
  const [collectionQuery, setCollectionQuery] = useState("");
  useEffect(() => setSelectedIds([]), [selection]);
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => { if (event.key === "Escape" && !busy) onClose(); };
    document.addEventListener("keydown", closeOnEscape);
    return () => document.removeEventListener("keydown", closeOnEscape);
  }, [busy, onClose]);
  const countLabel = count === null ? "Counting files…" : `${count.toLocaleString()} ${count === 1 ? "file" : "files"}`;
  const selectionLabel = selection.folder ? "Folder import" : selection.paths.length === 1 ? "File import" : "File selection";
  const canConfirm = !counting && !error && count !== null && count > 0 && !busy;
  const collectionMatches = collections.filter(collection => collection.name.toLowerCase().includes(collectionQuery.trim().toLowerCase()));
  const selectedCollections = selectedIds.map(id => collections.find(collection => collection.id === id)).filter((collection): collection is Collection => !!collection);

  return <div className="import-preview-backdrop" role="dialog" aria-modal="true" aria-labelledby="import-preview-title" onMouseDown={event => { if (event.target === event.currentTarget && !busy) onClose(); }}>
    <section className="import-preview-dialog">
      <button type="button" className="preview-close import-preview-close" aria-label="Close import preview" disabled={busy} onClick={onClose}><X /></button>
      <div className="import-preview-heading">
        <div className="import-preview-icon"><FileStack /></div>
        <div><p>Review import</p><h2 id="import-preview-title">Ready to import</h2></div>
      </div>
      <div className="import-preview-summary">
        <span className="import-preview-summary-icon">{selection.folder ? <FolderOpen /> : <FileStack />}</span>
        <div><strong>{displayName(selection.paths[0], selection.folder)}</strong><small>{selectionLabel}</small></div>
        <div className="import-preview-count"><strong>{countLabel}</strong><small>{selection.folder ? "in this folder" : "selected"}</small></div>
      </div>
      <section className="import-collection-picker" aria-labelledby="import-collection-title">
        <div className="import-section-heading"><div><h3 id="import-collection-title">Add to collections</h3><p>Optional · applied to every imported file</p></div></div>
        {collections.length ? <Popover.Root open={collectionMenuOpen} onOpenChange={value => { setCollectionMenuOpen(value); if (!value) setCollectionQuery(""); }}>
          <div className="import-selected-collections" aria-label="Selected collections">
            <div className="import-collection-chip-list">{selectedCollections.map(collection => <span className="import-collection-chip" key={collection.id}><span>{collection.name}</span><button type="button" disabled={busy} aria-label={`Remove ${collection.name}`} onClick={() => setSelectedIds(current => current.filter(id => id !== collection.id))}><X /></button></span>)}</div>
            <Popover.Trigger asChild><button type="button" className="import-collection-add" disabled={busy} aria-label="Add collection" title="Add collection"><Plus /></button></Popover.Trigger>
          </div>
          <Popover.Portal><Popover.Content className="import-collection-menu" align="start" sideOffset={7} collisionPadding={12} aria-label="Select collections">
            <div className="import-collection-search"><Search /><input autoFocus aria-label="Search collections" placeholder="Search collections…" value={collectionQuery} onChange={event => setCollectionQuery(event.target.value)} /></div>
            <div className="import-collection-options">
              {collectionMatches.map(collection => {
                const selected = selectedIds.includes(collection.id);
                return <button type="button" key={collection.id} aria-pressed={selected} className={selected ? "selected" : ""} onClick={() => setSelectedIds(current => selected ? current.filter(id => id !== collection.id) : [...current, collection.id])}><span className="import-checkbox">{selected && <Check />}</span><span className="import-collection-name"><FolderOpen />{collection.name}</span></button>;
              })}
              {!collectionMatches.length && <p>{collections.length ? "No collections match your search." : "No collections yet."}</p>}
            </div>
          </Popover.Content></Popover.Portal>
        </Popover.Root> : <p className="import-no-collections">No collections yet. Cancel this preview to create one from the Collections sidebar.</p>}
      </section>
      {error && <p className="import-preview-error" role="alert">{error}</p>}
      <footer className="import-preview-footer"><button type="button" disabled={busy} onClick={onClose}>Cancel</button><button type="button" className="import-preview-confirm" disabled={!canConfirm} onClick={() => onConfirm(selectedIds)}>{busy ? "Queuing…" : count === null ? "Preparing…" : `Import ${count.toLocaleString()} ${count === 1 ? "file" : "files"}`}</button></footer>
    </section>
  </div>;
}

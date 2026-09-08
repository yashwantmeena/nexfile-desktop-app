import { createContext, useCallback, useEffect, useContext, useRef, useState, type ReactNode } from "react";
import { createCollection, listCollections, updateCollection, deleteCollection, type Collection } from "@/features/collections/api/collections";
import { FolderPlus, X } from "lucide-react";

const CollectionsContext = createContext<{
  collections: Collection[];
  selected: Collection | undefined;
  select: (id: string | null) => void;
  openCreate: () => void;
  openEdit: (id: string) => void;
  openDelete: (id: string) => void;
  loadError: string;
  refresh: () => void;
}>(null!);
export const useCollections = () => useContext(CollectionsContext);
const errorMessage = (error: unknown) => typeof error === "object" && error !== null && "message" in error && typeof error.message === "string" ? error.message : "Unable to save collections. Please try again.";

export function CollectionsProvider({ children }: { children: ReactNode }) {
  const [collections, setCollections] = useState<Collection[]>([]);
  const [selectedId, select] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<Collection | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [loadError, setLoadError] = useState("");
  const generation = useRef(0);
  const deleteDialog = useRef<HTMLDialogElement>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const refresh = useCallback(async () => {
    const request = ++generation.current;
    try {
      const items = await listCollections();
      if (request !== generation.current) return;
      setCollections(items); setLoadError("");
    } catch (error) { if (request === generation.current) setLoadError(errorMessage(error)); }
  }, []);
  useEffect(() => {
    void refresh();
    const onFocus = () => { if (!dialog.current?.open && !deleteDialog.current?.open) void refresh(); };
    window.addEventListener("focus", onFocus);
    return () => { generation.current++; window.removeEventListener("focus", onFocus); };
  }, [refresh]);
  const apply = (items: Collection[]) => { generation.current++; setCollections(items); };
  const duplicate = collections.some(item => item.id !== editingId && item.name.toLowerCase() === name.trim().toLowerCase());
  return <CollectionsContext.Provider value={{ collections, selected: collections.find(item => item.id === selectedId), select, loadError, refresh: () => { void refresh(); },
    openCreate: () => { setEditingId(null); setName(""); setError(""); void refresh(); dialog.current?.showModal(); },
    openEdit: id => { const item = collections.find(item => item.id === id); if (item) { setEditingId(id); setName(item.name); setError(""); dialog.current?.showModal(); } },
    openDelete: id => { const item = collections.find(item => item.id === id); if (item) { setDeleting(item); setError(""); deleteDialog.current?.showModal(); } } }}>
    {children}
    <dialog ref={dialog} className="collection-dialog" aria-labelledby="collection-dialog-title" onCancel={event => { if (saving) event.preventDefault(); }}>
      <form onSubmit={async event => {
        event.preventDefault();
        if (saving || !name.trim() || duplicate) return;
        setSaving(true); setError("");
        try {
          apply(await (editingId ? updateCollection(editingId, name.trim()) : createCollection(name.trim())));
          dialog.current?.close();
        } catch (error) { setError(errorMessage(error)); } finally { setSaving(false); }
      }}>
        <button type="button" className="collection-close" disabled={saving} aria-label="Close" onClick={() => dialog.current?.close()}><X size={18}/></button>
        <div className="collection-symbol"><FolderPlus size={24}/></div>
        <h2 id="collection-dialog-title">{editingId ? "Rename collection" : "Create collection"}</h2>
        <p>Create a collection here, then choose it when importing files.</p>
        <label htmlFor="collection-name">Collection name</label>
        <input id="collection-name" autoFocus disabled={saving} maxLength={60} placeholder="e.g. Brand assets" value={name} onChange={event => setName(event.target.value)} aria-invalid={duplicate}/>
        {duplicate && <small role="alert">A collection with this name already exists.</small>}
        {error && <small role="alert">{error}</small>}
        <footer><button type="button" disabled={saving} onClick={() => dialog.current?.close()}>Cancel</button><button type="submit" disabled={saving || !name.trim() || duplicate}>{saving ? "Saving…" : editingId ? "Save changes" : "Create collection"}</button></footer>
      </form>
    </dialog>
    <dialog ref={deleteDialog} className="collection-dialog" aria-labelledby="delete-collection-title" onCancel={event => { if (saving) event.preventDefault(); }}>
      <h2 id="delete-collection-title">Delete collection?</h2>
      <p>Delete “{deleting?.name}”? This removes it from future import choices. Files and existing drive collections stay unchanged.</p>
      {error && <small role="alert">{error}</small>}
      <footer><button type="button" disabled={saving} autoFocus onClick={() => deleteDialog.current?.close()}>Cancel</button><button type="button" disabled={saving} className="delete-collection-confirm" onClick={async () => {
        if (!deleting || saving) return;
        setSaving(true); setError("");
        try {
          apply(await deleteCollection(deleting.id));
          if (selectedId === deleting.id) select(null);
          deleteDialog.current?.close();
        } catch (error) { setError(errorMessage(error)); } finally { setSaving(false); }
      }}>{saving ? "Deleting…" : "Delete collection"}</button></footer>
    </dialog>
  </CollectionsContext.Provider>;
}






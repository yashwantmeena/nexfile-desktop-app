import { useEffect, useState } from "react";
import { Popover } from "radix-ui";
import { Check, FolderOpen, Plus, Search, X } from "lucide-react";
import type { Collection } from "@/features/collections/api/collections";

interface CollectionPickerProps {
  collections: Collection[];
  selectedIds: string[];
  onChange: (ids: string[]) => void;
  disabled?: boolean;
  label?: string;
  addLabel?: string;
}

export function CollectionPicker({ collections, selectedIds, onChange, disabled = false, label = "Select collections", addLabel }: CollectionPickerProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const matches = collections.filter(collection => collection.name.toLowerCase().includes(query.trim().toLowerCase()));
  const selected = selectedIds
    .map(id => collections.find(collection => collection.id === id))
    .filter((collection): collection is Collection => !!collection);

  useEffect(() => {
    if (!open) setQuery("");
  }, [open]);

  const toggle = (id: string) => onChange(selectedIds.includes(id) ? selectedIds.filter(value => value !== id) : [...selectedIds, id]);
  return <Popover.Root open={open} onOpenChange={setOpen}>
    <div className="import-selected-collections" aria-label="Selected collections">
      <div className="import-collection-chip-list">
        {selected.map(collection => <span className="import-collection-chip" key={collection.id}><span>{collection.name}</span><button type="button" disabled={disabled} aria-label={`Remove ${collection.name}`} onClick={() => onChange(selectedIds.filter(id => id !== collection.id))}><X /></button></span>)}
      </div>
      <Popover.Trigger asChild><button type="button" className={`import-collection-add ${addLabel ? "with-label" : ""}`} disabled={disabled} aria-label="Add collection" title="Add collection"><Plus />{addLabel&&<span>{addLabel}</span>}</button></Popover.Trigger>
    </div>
    <Popover.Portal><Popover.Content className="import-collection-menu" align="start" sideOffset={7} collisionPadding={12} aria-label={label}>
      <div className="import-collection-search"><Search /><input autoFocus aria-label="Search collections" placeholder="Search collections…" value={query} onChange={event => setQuery(event.target.value)} /></div>
      <div className="import-collection-options">
        {matches.map(collection => {
          const isSelected = selectedIds.includes(collection.id);
          return <button type="button" key={collection.id} aria-pressed={isSelected} className={isSelected ? "selected" : ""} onClick={() => toggle(collection.id)}><span className="import-checkbox">{isSelected && <Check />}</span><span className="import-collection-name"><FolderOpen />{collection.name}</span></button>;
        })}
        {!matches.length && <p>{collections.length ? "No collections match your search." : "No collections yet."}</p>}
      </div>
    </Popover.Content></Popover.Portal>
  </Popover.Root>;
}

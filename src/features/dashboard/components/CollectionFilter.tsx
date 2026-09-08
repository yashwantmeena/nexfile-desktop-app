import { useState } from "react";
import { Popover } from "radix-ui";
import { Check, ChevronDown, FolderOpen, Search } from "lucide-react";
import { useCollections } from "@/components/layout/CollectionsProvider";

export function CollectionFilter({ onSelect }: { onSelect: (id: string | null) => void }) {
  const { collections, selected } = useCollections();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const matches = collections.filter(item => item.name.toLowerCase().includes(query.trim().toLowerCase()));
  const choose = (id: string | null) => { onSelect(id); setOpen(false); };
  return <Popover.Root open={open} onOpenChange={value => { setOpen(value); setQuery(""); }}>
    <Popover.Trigger asChild>
      <button type="button" className={`collection-filter-trigger${selected ? " active" : ""}`} aria-label={`Filter by collection: ${selected?.name ?? "All collections"}`}>
        <FolderOpen size={16}/><span>{selected?.name ?? "All collections"}</span><ChevronDown size={14}/>
      </button>
    </Popover.Trigger>
    <Popover.Portal>
      <Popover.Content className="collection-filter-menu" align="end" sideOffset={8} collisionPadding={12} aria-label="Filter by collection">
        <div className="collection-filter-search"><Search size={16}/><input aria-label="Search collections" placeholder="Search collections…" value={query} onChange={event => setQuery(event.target.value)}/></div>
        <div className="collection-filter-options">
          <button type="button" aria-pressed={!selected} onClick={() => choose(null)}><FolderOpen size={16}/><span>All collections</span>{!selected && <Check size={15}/>}</button>
          {matches.map(item => <button type="button" key={item.id} aria-pressed={selected?.id === item.id} onClick={() => choose(item.id)}><FolderOpen size={16}/><span>{item.name}</span>{selected?.id === item.id && <Check size={15}/>}</button>)}
          {!matches.length && <p>{collections.length ? "No collections match your search." : "No collections yet. Create one beside Import."}</p>}
        </div>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>;
}

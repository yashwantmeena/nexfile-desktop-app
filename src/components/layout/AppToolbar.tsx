import { useEffect, useRef, useState } from "react";
import { Check, ChevronDown, FileText, FileUp, FolderUp, Plus, Sparkles, Tag } from "lucide-react";
import { useCollections } from "./CollectionsProvider";
import { getImportErrorMessage, selectAndImportFiles, selectAndImportFolder } from "@/features/import/services/import_service";
import { TagSearchInput } from "./TagSearchInput";

interface AppToolbarProps {
  query: string;
  onQueryChange: (value: string) => void;
  onQuerySubmit: (value: string) => void;
  searchMode: SearchMode;
  onSearchModeChange: (value: SearchMode) => void;
}

export type SearchMode = "name" | "tags";

const searchModes = [
  { value: "name", label: "Name", description: "Match file names", icon: FileText, disabled: false },
  { value: "tags", label: "Tags", description: "Search assigned tags", icon: Tag, disabled: false },
  { value: "caption", label: "Caption", description: "Vector search", icon: Sparkles, disabled: true },
] as const;

export function AppToolbar({ query, onQueryChange, onQuerySubmit, searchMode, onSearchModeChange }: AppToolbarProps) {
  const { openCreate, collections } = useCollections();
  const [importCollections, setImportCollections] = useState<string[]>([]);
  const [searchMenuOpen, setSearchMenuOpen] = useState(false);
  const [importMenuOpen, setImportMenuOpen] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [importError, setImportError] = useState<string>();
  const searchModeRef = useRef<HTMLDivElement>(null);
  const importMenuRef = useRef<HTMLDivElement>(null);
  const placeholder = searchMode === "tags" ? "Search files by tag..." : "Search files by name...";

  useEffect(() => {
    const closeOnOutsideClick = (event: PointerEvent) => {
      if (!searchModeRef.current?.contains(event.target as Node)) setSearchMenuOpen(false);
      if (!importMenuRef.current?.contains(event.target as Node)) setImportMenuOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setSearchMenuOpen(false);
        setImportMenuOpen(false);
      }
    };
    document.addEventListener("pointerdown", closeOnOutsideClick);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsideClick);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, []);

  const startImport = async (selectAndImport: () => Promise<unknown>) => {
    setImportMenuOpen(false);
    setImportError(undefined);
    setIsImporting(true);

    try {
      await selectAndImport();
    } catch (error) {
      setImportError(getImportErrorMessage(error));
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <header className="nf-toolbar">
      <div className="global-search">
        <div className={`search-mode${searchMenuOpen ? " open" : ""}`} ref={searchModeRef}>
          <button className="search-mode-trigger" type="button" aria-haspopup="listbox" aria-expanded={searchMenuOpen} onClick={() => { setSearchMenuOpen((open) => !open); }}>
            <span>{searchMode === "name" ? "Name" : "Tags"}</span>
            <ChevronDown />
          </button>
          {searchMenuOpen && <div className="search-mode-menu" role="listbox" aria-label="Search using">
            <p>Search using</p>
            {searchModes.map(({ value, label, description, icon: Icon, disabled }) => (
              <button
                key={value}
                type="button"
                role="option"
                aria-selected={searchMode === value}
                aria-disabled={disabled || undefined}
                className={searchMode === value ? "selected" : ""}
                disabled={disabled}
                onClick={() => {
                  if (value !== "caption") onSearchModeChange(value);
                  setSearchMenuOpen(false);
                }}
              >
                <span className="mode-icon"><Icon /></span>
                <span className="mode-copy"><strong>{label}</strong><small>{description}</small></span>
                {disabled ? <em>Soon</em> : searchMode === value && <Check className="mode-check" />}
              </button>
            ))}
          </div>}
        </div>
        <span className="search-divider" />
        <button className="search-submit" type="button" aria-label="Search files" title="Search" onClick={() => onQuerySubmit(query)}><SearchIcon /></button>
        <TagSearchInput query={query} enabled={searchMode === "tags" && !searchMenuOpen} onChange={onQueryChange} onSubmit={onQuerySubmit} placeholder={placeholder} />
      </div>
      <button className="create-collection-button" type="button" onClick={openCreate} title="Create collection"><Plus size={17}/><span>Create collection</span></button>
      <div className={`import-menu${importMenuOpen ? " open" : ""}`} ref={importMenuRef}>
        <button className="import-files-button" type="button" aria-haspopup="dialog" aria-expanded={importMenuOpen} disabled={isImporting} onClick={() => setImportMenuOpen((open) => !open)}>
          <FileUp />
          <span>{isImporting ? "Queuing..." : "Import"}</span>
          <ChevronDown className="import-chevron" />
        </button>
        {importMenuOpen && <div className="import-options" role="dialog" aria-label="Import options">
          <fieldset className="import-collections"><legend>Add to collections</legend>{collections.length ? collections.map(item => <label key={item.id}><input type="checkbox" checked={importCollections.includes(item.id)} onChange={event => setImportCollections(current => event.target.checked ? [...current, item.id] : current.filter(id => id !== item.id))}/><span>{item.name}</span></label>) : <p>Create a collection to organize this import.</p>}<small>Optional · applied to every imported file</small></fieldset>
          <button type="button" onClick={() => void startImport(() => selectAndImportFiles(importCollections.filter(id => collections.some(item => item.id === id))))}>
            <FileUp />
            <span><strong>Import files</strong><small>Select one or more files</small></span>
          </button>
          <button type="button" onClick={() => void startImport(() => selectAndImportFolder(importCollections.filter(id => collections.some(item => item.id === id))))}>
            <FolderUp />
            <span><strong>Import folder</strong><small>Select a folder and its contents</small></span>
          </button>
        </div>}
        {importError && <p className="import-error" role="alert">{importError}</p>}
      </div>
    </header>
  );
}

function SearchIcon() {
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="11" cy="11" r="7" /><path d="m20 20-4-4" /></svg>;
}



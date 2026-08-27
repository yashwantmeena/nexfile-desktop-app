import { useEffect, useRef, useState } from "react";
import { CalendarDays, Check, ChevronDown, FileText, FileUp, FolderUp, SlidersHorizontal, Sparkles, Tag } from "lucide-react";
import { getImportErrorMessage, selectAndImportFiles, selectAndImportFolder } from "@/features/import/services/import_service";

export type DateFilter = "any" | "today" | "7days" | "30days" | "year";

interface AppToolbarProps {
  query: string;
  dateFilter: DateFilter;
  onQueryChange: (value: string) => void;
  onDateFilterChange: (value: DateFilter) => void;
}

type SearchMode = "name" | "tags";

const searchModes = [
  { value: "name", label: "Name", description: "Match file names", icon: FileText, disabled: false },
  { value: "tags", label: "Tags", description: "Search assigned tags", icon: Tag, disabled: false },
  { value: "caption", label: "Caption", description: "Vector search", icon: Sparkles, disabled: true },
] as const;

const dateFilters: { value:DateFilter; label:string }[] = [
  { value: "any", label: "Any time" },
  { value: "today", label: "Today" },
  { value: "7days", label: "Last 7 days" },
  { value: "30days", label: "Last 30 days" },
  { value: "year", label: "This year" },
];

export function AppToolbar({ query, dateFilter, onQueryChange, onDateFilterChange }: AppToolbarProps) {
  const [searchMode, setSearchMode] = useState<SearchMode>("tags");
  const [searchMenuOpen, setSearchMenuOpen] = useState(false);
  const [filterMenuOpen, setFilterMenuOpen] = useState(false);
  const [importMenuOpen, setImportMenuOpen] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [importError, setImportError] = useState<string>();
  const searchModeRef = useRef<HTMLDivElement>(null);
  const searchFilterRef = useRef<HTMLDivElement>(null);
  const importMenuRef = useRef<HTMLDivElement>(null);
  const placeholder = searchMode === "tags" ? "Search files by tag..." : "Search files by name...";

  useEffect(() => {
    const closeOnOutsideClick = (event: PointerEvent) => {
      if (!searchModeRef.current?.contains(event.target as Node)) setSearchMenuOpen(false);
      if (!searchFilterRef.current?.contains(event.target as Node)) setFilterMenuOpen(false);
      if (!importMenuRef.current?.contains(event.target as Node)) setImportMenuOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setSearchMenuOpen(false);
        setFilterMenuOpen(false);
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
          <button className="search-mode-trigger" type="button" aria-haspopup="listbox" aria-expanded={searchMenuOpen} onClick={() => { setFilterMenuOpen(false); setSearchMenuOpen((open) => !open); }}>
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
                  if (value !== "caption") setSearchMode(value);
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
        <SearchIcon />
        <input aria-label="Search files" value={query} onChange={(event) => onQueryChange(event.target.value)} placeholder={placeholder} />
        <div className={`search-filter-wrap${filterMenuOpen ? " open" : ""}`} ref={searchFilterRef}>
          <button className="search-filter-button" type="button" aria-label="Open search filters" aria-haspopup="dialog" aria-expanded={filterMenuOpen} title="Search filters" onClick={() => { setSearchMenuOpen(false); setFilterMenuOpen((open) => !open); }}><SlidersHorizontal /></button>
          {filterMenuOpen && <div className="search-filters-menu" role="dialog" aria-label="Search filters">
            <header className="filter-menu-header"><span><SlidersHorizontal /></span><div><strong>Search filters</strong><small>Narrow your results</small></div></header>
            <div className="filter-menu-content">
              <section className="filter-group">
                <header><span><CalendarDays /></span><div><strong>Date modified</strong><small>Choose when files were updated</small></div></header>
                <div className="filter-option-grid" role="radiogroup" aria-label="Date modified">
                  {dateFilters.map(({ value, label }) => <button key={value} type="button" role="radio" aria-checked={dateFilter === value} className={dateFilter === value ? "selected" : ""} onClick={() => onDateFilterChange(value)}><span className="date-radio"><i /></span>{label}{dateFilter === value && <Check />}</button>)}
                </div>
              </section>
            </div>
            <footer className="filter-menu-footer"><button type="button" className="filter-clear" disabled={dateFilter === "any"} onClick={() => onDateFilterChange("any")}>Clear</button><button type="button" className="filter-done" onClick={() => setFilterMenuOpen(false)}>Done</button></footer>
          </div>}
        </div>
      </div>
      <div className={`import-menu${importMenuOpen ? " open" : ""}`} ref={importMenuRef}>
        <button className="import-files-button" type="button" aria-haspopup="menu" aria-expanded={importMenuOpen} disabled={isImporting} onClick={() => setImportMenuOpen((open) => !open)}>
          <FileUp />
          <span>{isImporting ? "Queuing..." : "Import"}</span>
          <ChevronDown className="import-chevron" />
        </button>
        {importMenuOpen && <div className="import-options" role="menu">
          <button type="button" role="menuitem" onClick={() => void startImport(selectAndImportFiles)}>
            <FileUp />
            <span><strong>Import files</strong><small>Select one or more files</small></span>
          </button>
          <button type="button" role="menuitem" onClick={() => void startImport(selectAndImportFolder)}>
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

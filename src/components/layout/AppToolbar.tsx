import { ChevronDown, Filter, Grid2X2, LayoutGrid, List } from "lucide-react";

interface AppToolbarProps {
  query: string;
  gridMode: boolean;
  onQueryChange: (value: string) => void;
  onGridModeChange: (value: boolean) => void;
}

export function AppToolbar({ query, gridMode, onQueryChange, onGridModeChange }: AppToolbarProps) {
  return (
    <header className="nf-toolbar">
      <label className="global-search">
        <SearchIcon />
        <input value={query} onChange={(event) => onQueryChange(event.target.value)} placeholder="Search files, tags, content..." />
        <kbd>⌘K</kbd>
      </label>
      <div className="view-switch">
        <button className={gridMode ? "active" : ""} onClick={() => onGridModeChange(true)} aria-label="Grid view"><LayoutGrid /></button>
        <button className={gridMode ? "" : "active"} onClick={() => onGridModeChange(false)} aria-label="List view"><List /></button>
        <button aria-label="Compact grid"><Grid2X2 /></button>
      </div>
      <button className="sort-button">Sort: Newest <ChevronDown /></button>
      <button className="filter-button"><Filter />Filters</button>
    </header>
  );
}

function SearchIcon() {
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><circle cx="11" cy="11" r="7" /><path d="m20 20-4-4" /></svg>;
}

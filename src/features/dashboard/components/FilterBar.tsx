import type { ReactNode } from "react";
import { ListFilter, RotateCcw, X } from "lucide-react";

interface FilterBarProps {
  tags:string[];
  onTagsChange:(tags:string[])=>void;
  onReset:()=>void;
  selectAllLabel?:string;
  selectAllDisabled?:boolean;
  selectAllPressed?:boolean;
  onSelectAll?:()=>void;
  selectionActions?:ReactNode;
}

export function FilterBar({ tags, onTagsChange, onReset, selectAllLabel, selectAllDisabled = false, selectAllPressed = false, onSelectAll, selectionActions }:FilterBarProps) {
  const appliedCount=tags.length;
  return (
    <section className="match-row" aria-label="Active filters">
      <div className="filter-heading">
        <span><ListFilter /></span>
        <div><strong>Active filters</strong><small>{appliedCount} applied</small></div>
      </div>
      <div className="filter-scroll">
        {tags.map(tag=><span className="filter-tag" key={tag}>#{tag}<button aria-label={`Remove ${tag} filter`} onClick={()=>onTagsChange(tags.filter(item=>item!==tag))}><X /></button></span>)}
      </div>
      <div className="filter-actions">
        <button className="clear-all" disabled={!appliedCount} onClick={onReset}><RotateCcw />Reset</button>
        {onSelectAll && selectAllLabel && <button type="button" className="select-all-button" disabled={selectAllDisabled} aria-pressed={selectAllPressed} onClick={onSelectAll}>{selectAllLabel}</button>}
        {selectionActions}
      </div>
    </section>
  );
}

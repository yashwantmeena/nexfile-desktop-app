import { CalendarDays, ListFilter, RotateCcw, X } from "lucide-react";

interface FilterBarProps { tags:string[]; dateFilterLabel?:string; onTagsChange:(tags:string[])=>void; onClearDateFilter:()=>void; onReset:()=>void; }

export function FilterBar({ tags, dateFilterLabel, onTagsChange, onClearDateFilter, onReset }:FilterBarProps) {
  const appliedCount=tags.length+(dateFilterLabel?1:0);
  return (
    <section className="match-row" aria-label="Active filters">
      <div className="filter-heading">
        <span><ListFilter /></span>
        <div><strong>Active filters</strong><small>{appliedCount} applied</small></div>
      </div>
      <div className="filter-scroll">
        {tags.map(tag=><span className="filter-tag" key={tag}>#{tag}<button aria-label={`Remove ${tag} filter`} onClick={()=>onTagsChange(tags.filter(item=>item!==tag))}><X /></button></span>)}
        {dateFilterLabel&&<span className="filter-tag date-filter-tag"><CalendarDays />{dateFilterLabel}<button aria-label="Remove date filter" onClick={onClearDateFilter}><X /></button></span>}
      </div>
      <button className="clear-all" disabled={!appliedCount} onClick={onReset}><RotateCcw />Reset</button>
    </section>
  );
}

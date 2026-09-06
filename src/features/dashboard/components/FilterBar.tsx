import { ListFilter, RotateCcw, X } from "lucide-react";

interface FilterBarProps { tags:string[]; onTagsChange:(tags:string[])=>void; onReset:()=>void; }

export function FilterBar({ tags, onTagsChange, onReset }:FilterBarProps) {
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
      <button className="clear-all" disabled={!appliedCount} onClick={onReset}><RotateCcw />Reset</button>
    </section>
  );
}

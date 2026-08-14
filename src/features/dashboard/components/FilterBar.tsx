import { ChevronDown, Plus, X } from "lucide-react";

interface FilterBarProps { tags:string[]; onTagsChange:(tags:string[])=>void; }

export function FilterBar({ tags, onTagsChange }:FilterBarProps) {
  return <div className="match-row"><span>Match:</span><button>All tags <ChevronDown /></button>{tags.map(tag=><span className="filter-tag" key={tag}>{tag}<button onClick={()=>onTagsChange(tags.filter(item=>item!==tag))}><X /></button></span>)}<button className="add-tag"><Plus /> Add tag</button><button className="clear-all" onClick={()=>onTagsChange([])}>Clear all</button></div>;
}

import { Archive, File, Image, Music, Video } from "lucide-react";
import type { FileCategory } from "../types/file";

const icons:Record<string,typeof File>={All:File,Images:Image,PDFs:File,Documents:File,Videos:Video,Audio:Music,Archives:Archive};
interface CategoryFiltersProps { categories:FileCategory[]; activeCategory:string; onCategoryChange:(value:string)=>void; }

export function CategoryFilters({ categories, activeCategory, onCategoryChange }:CategoryFiltersProps) {
  return <div className="category-row">{categories.map(({label,count})=>{const Icon=icons[label];return <button key={label} className={activeCategory===label?"active":""} onClick={()=>onCategoryChange(label)}>{Icon&&<Icon />}<span>{label}</span><small>{count}</small></button>})}</div>;
}

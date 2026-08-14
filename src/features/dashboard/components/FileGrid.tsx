import { Search } from "lucide-react";
import type { DashboardFile } from "../types/file";
import { FileCard } from "./FileCard";

interface FileGridProps { files:DashboardFile[]; gridMode:boolean; selectedId:number; favorites:number[]; onSelect:(id:number)=>void; onFavorite:(id:number)=>void; }

export function FileGrid({ files,gridMode,selectedId,favorites,onSelect,onFavorite }:FileGridProps) {
  if(!files.length)return <div className="empty-state"><Search/><strong>No files found</strong><span>Try a different search term.</span></div>;
  return <div className={gridMode?"file-grid":"file-grid list-mode"}>{files.map(file=><FileCard key={file.id} file={file} selected={selectedId===file.id} favorite={favorites.includes(file.id)} onSelect={()=>onSelect(file.id)} onFavorite={()=>onFavorite(file.id)}/>)}</div>;
}

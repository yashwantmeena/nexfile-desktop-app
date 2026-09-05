import { Search } from "lucide-react";
import type { DashboardFile } from "../types/file";
import { FileCard } from "./FileCard";

interface FileGridProps { files:DashboardFile[]; onOpen:(index:number)=>void; }

export function FileGrid({ files,onOpen }:FileGridProps) {
  if(!files.length)return <div className="empty-state"><Search/><strong>No files found</strong><span>Try a different search term.</span></div>;
  return <div className="file-grid">{files.map((file,index)=><FileCard key={file.id} file={file} onOpen={()=>onOpen(index)}/>)}</div>;
}

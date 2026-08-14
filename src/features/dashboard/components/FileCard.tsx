import { Archive, File, FileCode2, FileText, MoreHorizontal, Music, Play, Star } from "lucide-react";
import type { DashboardFile } from "../types/file";

interface FileCardProps { file:DashboardFile; selected:boolean; favorite:boolean; onSelect:()=>void; onFavorite:()=>void; }

function FileArtwork({ file }: { file:DashboardFile }) {
  if(file.image) return <div className="photo-art" style={{backgroundImage:`url(${file.image})`}}><span className={`type-pill ${file.kind.toLowerCase()}`}>{file.kind}</span>{file.duration&&<><button className="play"><Play /></button><b className="duration">{file.duration}</b></>}</div>;
  if(file.kind==="FIG") return <div className="fig-art"><div className="fig-screen"><i/><i/><i/><i/></div><span className="type-pill fig">FIG</span></div>;
  const Icon=file.kind==="ZIP"?Archive:file.kind==="MP3"?Music:file.kind==="TS"?FileCode2:file.kind==="MD"?FileText:File;
  return <div className={`document-art art-${file.kind.toLowerCase()}`}><div className="paper"><Icon/><span/><span/><span/></div><span className={`type-pill ${file.kind.toLowerCase()}`}>{file.kind}</span></div>;
}

export function FileCard({ file, selected, favorite, onSelect, onFavorite }:FileCardProps) {
  return <article className={`file-card${selected?" selected":""}`} onClick={onSelect}><div className="file-card-top"><button className="select-dot" aria-label="Select file"><i/></button><button className="file-menu" aria-label="File menu"><MoreHorizontal /></button>{file.image&&<button className={`card-star${favorite?" favorite":""}`} onClick={(event)=>{event.stopPropagation();onFavorite();}}><Star /></button>}</div><FileArtwork file={file}/><div className="file-copy"><strong>{file.name}</strong><span>{file.path}</span><small>{file.time}<em>{file.time}</em></small></div></article>;
}

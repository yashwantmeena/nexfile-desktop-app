import { useEffect, useId, useRef, useState } from "react";
import { Archive, Check, ChevronLeft, ChevronRight, Clock3, Copy, ExternalLink, File, FileCode2, FileText, Film, FolderOpen, Heart, Mic2, MoreHorizontal, Pencil, Star, Tag, Wrench, X } from "lucide-react";
import type { DashboardFile } from "../types/file";
import { OriginalMediaPreview } from "./OriginalMediaPreview";

interface FilePreviewModalProps { files:DashboardFile[]; index:number; onIndexChange:(index:number)=>void; onClose:()=>void; onApplyFilter:(value:string)=>void; }

function formatSize(bytes?:number){if(bytes===undefined)return "Unknown";if(bytes<1024)return `${bytes} B`;const units=["KB","MB","GB","TB"];let value=bytes/1024,unit=0;while(value>=1024&&unit<units.length-1){value/=1024;unit++;}return `${value>=10?value.toFixed(0):value.toFixed(1)} ${units[unit]}`;}
function readable(value:string){return value.replace(/[_-]/g," ");}

export function FilePreviewModal({files,index,onIndexChange,onClose,onApplyFilter}:FilePreviewModalProps){
  const file=files[index];
  const [infoOpen,setInfoOpen]=useState(false);
  const infoId=useId();
  const infoToggleRef=useRef<HTMLButtonElement>(null);
  const closeInfo=()=>{setInfoOpen(false);infoToggleRef.current?.focus();};
  const [favorite,setFavorite]=useState(false);
  const [copied,setCopied]=useState(false);
  const [displayName,setDisplayName]=useState(file.name);
  const [draftName,setDraftName]=useState(file.name);
  const [editingName,setEditingName]=useState(false);
  useEffect(()=>{setFavorite(false);setDisplayName(file.name);setDraftName(file.name);setEditingName(false);},[file.id,file.name]);
  useEffect(()=>{const key=(event:KeyboardEvent)=>{if(event.key==="Escape")onClose();if(event.key==="ArrowLeft"&&index>0)onIndexChange(index-1);if(event.key==="ArrowRight"&&index<files.length-1)onIndexChange(index+1);};window.addEventListener("keydown",key);return()=>window.removeEventListener("keydown",key);},[files.length,index,onClose,onIndexChange]);
  const copyPath=async()=>{try{await navigator.clipboard.writeText(file.path);setCopied(true);window.setTimeout(()=>setCopied(false),1400);}catch{setCopied(false);}};
  const type=file.fileType??file.kind.toLowerCase();
  const fallbackIcon=type==="video"?Film:type==="audio"?Mic2:file.kind==="ZIP"?Archive:file.kind==="TS"?FileCode2:file.kind==="MD"?FileText:File;
  const FallbackIcon=fallbackIcon;
  return <div className="file-preview-backdrop" role="dialog" aria-modal="true" aria-label={`Preview ${file.name}`} onMouseDown={event=>{if(event.target===event.currentTarget)onClose();}}>
    <section className={`file-preview-modal ${infoOpen?"preview-info-open":"preview-info-collapsed"}`}>
      <button className="preview-close" aria-label="Close preview" onClick={onClose}><X/></button>
      <div className="preview-stage">
        <div className="preview-canvas">
          <div className="preview-top-actions">
            <button aria-label="Favorite" aria-pressed={favorite} className={favorite?"active":""} onClick={()=>setFavorite(value=>!value)}><Heart/></button>
            <button ref={infoToggleRef} aria-label={infoOpen?"Hide file information":"Show file information"} title={infoOpen?"Hide file information":"Show file information"} aria-controls={infoId} aria-expanded={infoOpen} onClick={()=>setInfoOpen(value=>!value)}><MoreHorizontal/></button>
          </div>
          {file.image?<OriginalMediaPreview key={`${file.id}:${file.image}`} src={file.image} name={file.name} type={type==="video"?"video":"image"}/>:<div className="preview-fallback"><FallbackIcon/><strong>{file.name}</strong></div>}
          <button className="preview-nav preview-previous" disabled={index===0} aria-label="Previous file" onClick={()=>onIndexChange(index-1)}><ChevronLeft/></button>
          <button className="preview-nav preview-next" disabled={index===files.length-1} aria-label="Next file" onClick={()=>onIndexChange(index+1)}><ChevronRight/></button>
        </div>
      </div>
      {infoOpen && <aside id={infoId} className="preview-details" aria-label="File information">
        <div className="preview-info-heading"><strong>File information</strong><button aria-label="Collapse file information" onClick={closeInfo}><X/></button></div>
        <header><span className="preview-file-icon"><File/></span><div className="preview-name-block">{editingName?<div className="preview-name-editor"><input value={draftName} autoFocus aria-label="File name" onChange={event=>setDraftName(event.target.value)} onKeyDown={event=>{if(event.key==="Enter"){setDisplayName(draftName.trim()||file.name);setEditingName(false);}if(event.key==="Escape"){setDraftName(displayName);setEditingName(false);}}}/><button aria-label="Save file name" onClick={()=>{setDisplayName(draftName.trim()||file.name);setEditingName(false);}}><Check/></button></div>:<div className="preview-name-row"><h2 title={displayName}>{displayName}</h2><button aria-label="Edit file name" onClick={()=>setEditingName(true)}><Pencil/></button></div>}<p>{file.kind} <i/> {formatSize(file.sizeBytes)}</p></div><button aria-label="Favorite" className={favorite?"active":""} onClick={()=>setFavorite(value=>!value)}><Star/></button></header>
        <div className="preview-info-grid">
          <section className="preview-info-card"><h3><File/>File information</h3><dl><div><dt>Type</dt><dd>{readable(type)}</dd></div><div><dt>Format</dt><dd>{file.kind}</dd></div><div><dt>Size</dt><dd>{formatSize(file.sizeBytes)}</dd></div></dl></section>
          <section className="preview-info-card"><h3><Clock3/>Timestamps</h3><dl><div><dt>Modified</dt><dd>{file.time}</dd></div><div><dt>Imported</dt><dd>{file.time}</dd></div></dl></section>
          <section className="preview-info-card"><h3><Tag/>Category</h3><div className="preview-card-tags">{file.categories?.[0]?<button onClick={()=>onApplyFilter(file.categories![0])}>{readable(file.categories[0])}</button>:<span>Uncategorized</span>}</div></section>
          <section className="preview-info-card"><h3><FolderOpen/>Collections</h3><div className="preview-collection-value">{file.collection?<button className="preview-filter-button" onClick={()=>onApplyFilter(file.collection!)}>{file.collection}</button>:"Not in a collection"}</div></section>
          <section className="preview-info-card"><h3><Tag/>Keywords & tags</h3><div className="preview-card-tags">{file.tags?.length?file.tags.map(tag=><button key={tag} onClick={()=>onApplyFilter(tag)}>{readable(tag)}</button>):<span>None</span>}</div></section>
          <section className="preview-info-card preview-path-card"><h3><FolderOpen/>Path</h3><div className="preview-card-path"><span title={file.path}>{file.path}</span><button onClick={copyPath} aria-label="Copy path"><Copy/></button></div></section>
          <section className="preview-info-card preview-actions-card"><h3><Wrench/>Actions</h3><div className="preview-action-buttons"><button className="primary"><ExternalLink/>Open</button><button><FolderOpen/>Open folder</button><button onClick={copyPath}><Copy/>{copied?"Copied":"Copy path"}</button><button aria-label="More actions"><MoreHorizontal/></button></div></section>
        </div>
      </aside>}
    </section>
  </div>;
}

import { useCallback, useEffect, useId, useRef, useState } from "react";
import { Archive, ChevronLeft, ChevronRight, Clock3, Copy, ExternalLink, File, FileCode2, FileText, Film, FolderOpen, Mic2, MoreHorizontal, Pencil, Star, Tag, Wrench, X } from "lucide-react";
import type { Collection } from "@/features/collections/api/collections";
import { updateFileMetadata } from "../api/files";
import { CollectionPicker } from "@/components/layout/CollectionPicker";
import type { DashboardFile } from "../types/file";
import { OriginalMediaPreview } from "./OriginalMediaPreview";

interface FilePreviewModalProps {
  files:DashboardFile[];
  index:number;
  collections:Collection[];
  categories:string[];
  onMetadataSaved:()=>void;
  onFavoriteSaved:(id:DashboardFile["id"],favorite:boolean)=>void;
  onIndexChange:(index:number)=>void;
  onClose:()=>void;
  onApplyFilter:(value:string)=>void;
  hasMore:boolean;
  loadingMore:boolean;
  loadError:string|null;
  onLoadMore:()=>void;
}

function formatSize(bytes?:number){if(bytes===undefined)return "Unknown";if(bytes<1024)return `${bytes} B`;const units=["KB","MB","GB","TB"];let value=bytes/1024,unit=0;while(value>=1024&&unit<units.length-1){value/=1024;unit++;}return `${value>=10?value.toFixed(0):value.toFixed(1)} ${units[unit]}`;}
function readable(value:string){return value.replace(/[_-]/g," ");}

export function FilePreviewModal({files,index,collections,categories,onMetadataSaved,onFavoriteSaved,onIndexChange,onClose,onApplyFilter,hasMore,loadingMore,loadError,onLoadMore}:FilePreviewModalProps){
  const file=files[index];
  const [pendingNextFrom, setPendingNextFrom] = useState<DashboardFile["id"]|null>(null);
  const atLoadedEnd = index >= files.length - 1;
  const next = useCallback(() => {
    if (index < files.length - 1) {
      setPendingNextFrom(null);
      onIndexChange(index + 1);
    } else if (hasMore) {
      setPendingNextFrom(file.id);
      if (!loadingMore) onLoadMore();
    }
  }, [index, files.length, hasMore, file.id, loadingMore, onLoadMore, onIndexChange]);
  const previous = useCallback(() => {
    setPendingNextFrom(null);
    if (index > 0) onIndexChange(index - 1);
  }, [index, onIndexChange]);

  useEffect(() => {
    if (pendingNextFrom === null) return;
    if (pendingNextFrom !== file.id) {
      setPendingNextFrom(null);
    } else if (!atLoadedEnd) {
      setPendingNextFrom(null);
      onIndexChange(index + 1);
    } else if (!loadingMore) {
      if (loadError || !hasMore) setPendingNextFrom(null);
      // A page can contain only duplicates after files shift between requests.
      else onLoadMore();
    }
  }, [pendingNextFrom, file.id, atLoadedEnd, index, loadingMore, loadError, hasMore, onLoadMore, onIndexChange]);

  useEffect(() => {
    // Prefetch near the boundary even when the grid's scroll sentinel is offscreen.
    if (files.length - index <= 3 && hasMore && !loadingMore && !loadError) onLoadMore();
  }, [files.length, index, hasMore, loadingMore, loadError, onLoadMore]);
  const [infoOpen,setInfoOpen]=useState(false);
  const infoId=useId();
  const infoToggleRef=useRef<HTMLButtonElement>(null);
  const closeInfo=()=>{setInfoOpen(false);infoToggleRef.current?.focus();};
  const [favorite,setFavorite]=useState(file.favorite ?? false);
  const [favoriteSaving,setFavoriteSaving]=useState(false);
  const [favoriteError,setFavoriteError]=useState("");
  const [copied,setCopied]=useState(false);
  const [editingMetadata,setEditingMetadata]=useState(false);
  const [draftName,setDraftName]=useState(file.name);
  const [draftCategory,setDraftCategory]=useState(file.categories?.[0] ?? "");
  const [draftTags,setDraftTags]=useState<string[]>(file.tags ?? []);
  const [draftCollectionIds,setDraftCollectionIds]=useState<string[]>([]);
  const [tagInput,setTagInput]=useState("");
  const [metadataSaving,setMetadataSaving]=useState(false);
  const [metadataError,setMetadataError]=useState("");
  const initialCollectionIds = () => collections.filter(collection => collectionsForFile(file).some(name => name.toLowerCase() === collection.name.toLowerCase())).map(collection => collection.id);
  useEffect(()=>{setFavorite(file.favorite ?? false);setFavoriteSaving(false);setFavoriteError("");setEditingMetadata(false);},[file.id,file.favorite]);
  useEffect(()=>{
    setDraftName(file.name);
    setDraftCategory(file.categories?.[0] ?? "");
    setDraftTags(file.tags ?? []);
    setDraftCollectionIds(initialCollectionIds());
    setTagInput("");
    setMetadataError("");
  // The collection catalog can finish loading after the preview opens.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  },[file.id,file.name,file.categories,file.tags,file.collections,file.collection,collections]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      const target = event.target;
      if (target instanceof HTMLElement && target.closest("input, textarea, select, [contenteditable='true']")) return;
      if (event.key === "Escape") onClose();
      if (event.key === "ArrowLeft") { event.preventDefault(); previous(); }
      if (event.key === "ArrowRight") { event.preventDefault(); next(); }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [onClose, previous, next]);
  const copyPath=async()=>{try{await navigator.clipboard.writeText(file.path);setCopied(true);window.setTimeout(()=>setCopied(false),1400);}catch{setCopied(false);}};
  const type=file.fileType??file.kind.toLowerCase();
  const canEditMetadata = type === "image";
  const fallbackIcon=type==="video"?Film:type==="audio"?Mic2:file.kind==="ZIP"?Archive:file.kind==="TS"?FileCode2:file.kind==="MD"?FileText:File;
  const FallbackIcon=fallbackIcon;
  const beginMetadataEdit = () => { setDraftName(file.name); setDraftCategory(file.categories?.[0] ?? ""); setDraftTags(file.tags ?? []); setDraftCollectionIds(initialCollectionIds()); setTagInput(""); setMetadataError(""); setEditingMetadata(true); };
  const cancelMetadataEdit = () => { setDraftName(file.name); setDraftCategory(file.categories?.[0] ?? ""); setDraftTags(file.tags ?? []); setDraftCollectionIds(initialCollectionIds()); setTagInput(""); setMetadataError(""); setEditingMetadata(false); };
  const addTags = (value:string) => {
    const additions = value.split(/[\s,_-]+/).map(tag => tag.trim().toLowerCase()).filter(Boolean);
    if (!additions.length) return;
    setDraftTags(current => additions.reduce((result, tag) => result.some(existing => existing === tag) ? result : [...result, tag], current));
    setTagInput("");
  };
  const saveMetadata = async () => {
    if (!file.driveId) { setMetadataError("This file cannot be updated because its drive is unavailable."); return; }
    setMetadataSaving(true); setMetadataError("");
    try {
      await updateFileMetadata(
        { driveId:file.driveId, path:file.path },
        { name:draftName.trim(), category:draftCategory, tags:draftTags, collectionIds:draftCollectionIds },
      );
      setEditingMetadata(false);
      onMetadataSaved();
    } catch (error) {
      setMetadataError(typeof error === "object" && error !== null && "message" in error && typeof error.message === "string" ? error.message : "Unable to save file metadata.");
    } finally { setMetadataSaving(false); }
  };
  const toggleFavorite = async () => {
    if (!file.driveId || favoriteSaving) return;
    const nextFavorite = !favorite;
    setFavoriteError("");
    setFavorite(nextFavorite);
    setFavoriteSaving(true);
    try {
      await updateFileMetadata({ driveId:file.driveId, path:file.path }, { favorite:nextFavorite });
      onFavoriteSaved(file.id, nextFavorite);
    } catch {
      setFavorite(!nextFavorite);
      setFavoriteError("Unable to update this favorite. Please try again.");
    } finally {
      setFavoriteSaving(false);
    }
  };
  return <div className="file-preview-backdrop" role="dialog" aria-modal="true" aria-label={`Preview ${file.name}`} onMouseDown={event=>{if(event.target===event.currentTarget)onClose();}}>
    <section className={`file-preview-modal ${infoOpen?"preview-info-open":"preview-info-collapsed"}`}>
      {!infoOpen&&<button className="preview-close" aria-label="Close preview" onClick={onClose}><X/></button>}
      <div className="preview-stage">
        <div className="preview-canvas">
          <div className="preview-top-actions">
            <button aria-label={favorite?"Remove from favorites":"Add to favorites"} title={favorite?"Remove from favorites":"Add to favorites"} aria-pressed={favorite} aria-busy={favoriteSaving} disabled={!file.driveId || favoriteSaving} className={favorite?"active":""} onClick={()=>void toggleFavorite()}><Star/></button>
            <button ref={infoToggleRef} aria-label={infoOpen?"Hide file information":"Show file information"} title={infoOpen?"Hide file information":"Show file information"} aria-controls={infoId} aria-expanded={infoOpen} onClick={()=>{if(infoOpen){cancelMetadataEdit();setInfoOpen(false);}else setInfoOpen(true);}}><MoreHorizontal/></button>
          </div>
          {file.image?<OriginalMediaPreview key={`${file.id}:${file.image}`} src={file.image} name={file.name} type={type==="video"?"video":"image"}/>:<div className="preview-fallback"><FallbackIcon/><strong>{file.name}</strong></div>}
          <button className="preview-nav preview-previous" disabled={index===0} aria-label="Previous file" onClick={previous}><ChevronLeft/></button>
          <button className="preview-nav preview-next" disabled={atLoadedEnd && !hasMore} aria-label="Next file" aria-busy={atLoadedEnd && loadingMore} onClick={next}><ChevronRight/></button>
          {atLoadedEnd && loadingMore && <div className="preview-pagination-status" role="status">Loading more files…</div>}
          {atLoadedEnd && loadError && <div className="preview-pagination-status" role="alert">Couldn’t load more files. <button onClick={next}>Retry</button></div>}
          {favoriteError && <div className="preview-pagination-status" role="alert">{favoriteError}</div>}
        </div>
      </div>
      {infoOpen && <aside id={infoId} className="preview-details" aria-label="File information">
        <div className="preview-info-heading"><div><strong>File information</strong><p>{editingMetadata?"View and edit the details for this file.":"View the details for this file."}</p></div><div className="preview-info-heading-actions">{!editingMetadata&&canEditMetadata&&<button type="button" className="preview-edit-action" aria-label="Edit file information" title="Edit file information" onClick={beginMetadataEdit}><Pencil/></button>}<button type="button" className="preview-heading-close" aria-label="Collapse file information" onClick={()=>{cancelMetadataEdit();closeInfo();}}><X/></button></div></div>
        <header className="preview-file-summary"><div className="preview-name-block">{editingMetadata?<input className="preview-file-name-input" value={draftName} autoFocus aria-label="File name" onChange={event=>setDraftName(event.target.value)} />:<h2 title={file.name}>{file.name}</h2>}</div><p className="preview-file-meta">{file.kind} <i/> {formatSize(file.sizeBytes)}</p></header>
        <div className="preview-info-grid">
          <section className="preview-info-card"><h3><File/>File information</h3><dl><div><dt>Type</dt><dd>{readable(type)}</dd></div><div><dt>Format</dt><dd>{file.kind}</dd></div><div><dt>Size</dt><dd>{formatSize(file.sizeBytes)}</dd></div></dl></section>
          <section className="preview-info-card"><h3><Clock3/>Timestamps</h3><dl><div><dt>Modified</dt><dd>{file.time}</dd></div><div><dt>Imported</dt><dd>{file.time}</dd></div></dl></section>
          <section className="preview-info-card"><h3><Tag/>Category</h3>{editingMetadata?<select className="preview-category-select" aria-label="Category" value={draftCategory} onChange={event=>setDraftCategory(event.target.value)}><option value="">Uncategorized</option>{[...new Set([...(file.categories??[]),...categories])].filter(Boolean).map(category=><option value={category} key={category}>{readable(category)}</option>)}</select>:<div className="preview-card-tags">{file.categories?.[0]?<button className="preview-metadata-chip" onClick={()=>onApplyFilter(file.categories![0])}>{readable(file.categories[0])}</button>:<span className="preview-metadata-chip">Uncategorized</span>}</div>}</section>
          <section className="preview-info-card"><div className="preview-info-card-heading"><h3><FolderOpen/>Collections</h3></div><div className="preview-collection-value">{editingMetadata ? (collections.length ? <CollectionPicker collections={collections} selectedIds={draftCollectionIds} onChange={setDraftCollectionIds} disabled={metadataSaving} addLabel="Add" /> : <p className="preview-metadata-empty">No collections available.</p>) : collectionsForFile(file).length?<div className="preview-collection-tags">{collectionsForFile(file).map(collection=><span className="preview-metadata-chip" key={collection}>{collection}</span>)}</div>:"Not in a collection"}</div></section>
          <section className="preview-info-card preview-tags-card-wide"><div className="preview-info-card-heading"><h3><Tag/>Keywords & tags</h3></div>{editingMetadata?<div className="preview-tag-editor"><div className="preview-card-tags">{draftTags.map(tag=><span className="preview-metadata-chip" key={tag}><span>{readable(tag)}</span><button type="button" disabled={metadataSaving} aria-label={`Remove ${tag}`} onClick={()=>setDraftTags(current=>current.filter(value=>value!==tag))}><X/></button></span>)}</div><input value={tagInput} disabled={metadataSaving} aria-label="Add tag" placeholder="Add a tag and press Enter" onChange={event=>setTagInput(event.target.value)} onKeyDown={event=>{if(event.key === "Enter" || event.key === ","){event.preventDefault();addTags(tagInput);}if(event.key === "Backspace"&&!tagInput&&draftTags.length)setDraftTags(current=>current.slice(0,-1));}} onBlur={()=>{if(tagInput.trim())addTags(tagInput);}} /></div>:<div className="preview-card-tags">{file.tags?.length?file.tags.map(tag=><button className="preview-metadata-chip" key={tag} onClick={()=>onApplyFilter(tag)}>{readable(tag)}</button>):<span className="preview-metadata-chip">None</span>}</div>}</section>
          <section className="preview-info-card preview-path-card"><h3><FolderOpen/>Path</h3><div className="preview-card-path"><span title={file.path}>{file.path}</span><button onClick={copyPath} aria-label="Copy path"><Copy/></button></div></section>
          <section className="preview-info-card preview-actions-card"><h3><Wrench/>Actions</h3><div className="preview-action-buttons"><button className="primary"><ExternalLink/>Open</button><button><FolderOpen/>Open folder</button><button onClick={copyPath}><Copy/>{copied?"Copied":"Copy path"}</button><button aria-label="More actions"><MoreHorizontal/></button></div></section>
        </div>
        {editingMetadata&&<div className="preview-metadata-actions">{metadataError&&<p role="alert">{metadataError}</p>}<div><button type="button" disabled={metadataSaving} onClick={cancelMetadataEdit}>Cancel</button><button type="button" className="primary" disabled={metadataSaving || !draftName.trim() || (collectionsForFile(file).length > 0 && collections.length === 0)} onClick={()=>void saveMetadata()}>{metadataSaving?"Saving…":"Save changes"}</button></div></div>}
      </aside>}
    </section>
  </div>;
}

function collectionsForFile(file:DashboardFile){return file.collections?.length ? file.collections : file.collection ? [file.collection] : [];}

import { File, Star } from "lucide-react";
import type { DashboardFile } from "../types/file";

interface FileInspectorProps { file:DashboardFile; favorite:boolean; onFavorite:()=>void; }

function readable(value: string) { return value.replace(/[_-]/g, " "); }
function formatSize(bytes?: number) {
  if (bytes === undefined) return "Unknown";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit += 1; }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

export function FileInspector({ file, favorite, onFavorite }: FileInspectorProps) {
  const categories = file.categories ?? [];
  const tags = file.tags ?? [];
  return <aside className="inspector">
    <header className="inspector-header"><span>File details</span></header>
    <div className="preview-image" style={file.image && file.fileType !== "video" ? { backgroundImage: `url(${file.image})` } : undefined}>
      {file.image && file.fileType === "video" && file.kind !== "WEBM" && <video src={file.image} controls preload="metadata" playsInline />}
      {(!file.image || file.kind === "WEBM") && <File aria-label="No preview available" />}
    </div>
    <div className="preview-title"><div><h2 title={file.name}>{file.name}</h2><p>{file.kind} <i/> {formatSize(file.sizeBytes)}</p></div>
      <button aria-label={favorite ? "Remove from favorites" : "Add to favorites"} className={favorite ? "favorite" : ""} onClick={onFavorite}><Star/></button>
    </div>
    <dl className="file-meta-grid">
      <div><dt>Category</dt><dd className="capitalize">{categories[0] ? readable(categories[0]) : "Uncategorized"}</dd></div>
      <div><dt>Collection</dt><dd>{file.collection ?? "No collection"}</dd></div>
      <div><dt>Modified on</dt><dd>{file.time}</dd></div>
      <div><dt>Type</dt><dd className="capitalize">{file.fileType ?? file.kind}</dd></div>
    </dl>
    <section className="tag-section"><h3>Tags</h3><div>
      {tags.length ? tags.map(tag => <span key={tag}>#{readable(tag)}</span>) : <span>No tags</span>}
    </div></section>
    <section className="tag-section file-path-section"><h3>Path</h3><p>{file.path}</p></section>
  </aside>;
}

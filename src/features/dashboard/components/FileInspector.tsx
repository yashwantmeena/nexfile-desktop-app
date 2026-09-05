import { useState } from "react";
import { Check, ChevronDown, Copy, Star } from "lucide-react";
import type { DashboardFile } from "../types/file";

interface FileInspectorProps { file:DashboardFile; favorite:boolean; onFavorite:()=>void; }

function readable(value: string) { return value.replace(/[_-]/g, " "); }
function formatSize(bytes?: number) {
  if (bytes === undefined) return "Unknown size";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit += 1; }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

export function FileInspector({ file, favorite, onFavorite }: FileInspectorProps) {
  const [copied, setCopied] = useState(false);
  const categories = file.categories ?? [];
  const tags = (file.tags ?? []).slice(0, 5);
  const collection = file.collection?.trim();
  const copyPath = async () => {
    try {
      await navigator.clipboard.writeText(file.path);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch { setCopied(false); }
  };

  return <aside className="inspector redesigned-inspector">
    <header className="inspector-header"><span>File details</span></header>
    <div className="inspector-identity"><div><h2 title={file.name}>{file.name}</h2><p><span className="identity-type">{readable(file.fileType ?? file.kind)}</span><i/>{file.kind}<i/>{formatSize(file.sizeBytes)}</p></div>
      <button aria-label={favorite ? "Remove from favorites" : "Add to favorites"} className={favorite ? "favorite" : ""} onClick={onFavorite}><Star/></button>
    </div>
    <section className="inspector-organization" aria-label="Organization">
      <div><span>Category</span><strong>{categories[0] ? readable(categories[0]) : "Uncategorized"}</strong></div>
      {collection && <div><span>Collection</span><strong>{collection}</strong></div>}
    </section>
    {tags.length > 0 && <section className="inspector-tags"><h3>Tags</h3><div>{tags.map(tag => <span key={tag}>{readable(tag)}</span>)}</div></section>}
    <details className="inspector-file-details">
      <summary><span>File information</span><ChevronDown/></summary>
      <dl><div><dt>Modified</dt><dd>{file.time}</dd></div><div><dt>Format</dt><dd>{file.kind}</dd></div><div className="inspector-path-row"><dt>Path</dt><dd title={file.path}>{file.path}</dd></div></dl>
      <button className="copy-path-button" onClick={copyPath}>{copied ? <Check/> : <Copy/>}{copied ? "Copied" : "Copy path"}</button>
    </details>
  </aside>;
}

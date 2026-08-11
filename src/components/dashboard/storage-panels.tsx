import { Folder } from "lucide-react";
import { folderSummaries } from "@/data/localmind-data";

interface StoragePanelsProps {
  sensitivity: number;
  onSensitivityChange: (value: number) => void;
}

const storageTypes = [
  { label: "Documents", value: "35%", color: "#6265f4" },
  { label: "Images", value: "28%", color: "#2dd4bf" },
  { label: "Videos", value: "22%", color: "#a78bfa" },
  { label: "Others", value: "15%", color: "#ffb51b" },
];

export function StoragePanels({ sensitivity, onSensitivityChange }: StoragePanelsProps) {
  return (
    <aside className="side-panels" aria-label="Storage details">
      <section className="side-card storage-card">
        <header><h2>Storage Overview</h2><span>55% used</span></header>
        <div className="storage-content">
          <div className="storage-donut" aria-label="284 gigabytes of 512 gigabytes used">
            <div><strong>284 GB</strong><small>of 512 GB</small></div>
          </div>
          <ul className="storage-legend">
            {storageTypes.map((type) => <li key={type.label}><i style={{ background: type.color }} /><span>{type.label}</span><small>{type.value}</small></li>)}
          </ul>
        </div>
      </section>

      <section className="side-card folder-card">
        <header><h2>Folder Overview</h2><span>4 folders</span></header>
        <div className="folder-list">
          {folderSummaries.map((folder) => (
            <div className="folder-item" key={folder.path}>
              <div><span><Folder />{folder.path}</span><small>{folder.detail}</small></div>
              <progress max="100" value={folder.progress} aria-label={`${folder.path} storage`} />
            </div>
          ))}
        </div>
      </section>

      <section className="side-card sensitivity-card">
        <h2>Index Sensitivity</h2>
        <p>Tune how aggressively the local model tags new files.</p>
        <input
          type="range"
          min="0"
          max="100"
          value={sensitivity}
          aria-label="Index sensitivity"
          style={{ "--slider-fill": `${sensitivity}%` } as React.CSSProperties}
          onChange={(event) => onSensitivityChange(Number(event.target.value))}
        />
        <div><span>Conservative</span><span>Aggressive</span></div>
      </section>
    </aside>
  );
}

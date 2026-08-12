import { Search } from "lucide-react";
import type { ArchiveAsset } from "@/features/indexing/types";

interface AssetGridProps {
  assets: ArchiveAsset[];
  emptyMessage?: string;
}

export function AssetGrid({ assets, emptyMessage = "No matching items" }: AssetGridProps) {
  return (
    <div className="asset-grid">
      {assets.length > 0 ? assets.map((asset) => (
        <article className="asset-card" key={asset.title} tabIndex={0}>
          <div className={`asset-image ${asset.imageClass}`} role="img" aria-label={asset.title} />
          <div className="asset-info">
            <div className="asset-title"><strong>{asset.title}</strong><span>{asset.size}</span></div>
            <div className="tag-list">{asset.tags.map((tag) => <span key={tag}>{tag}</span>)}</div>
          </div>
        </article>
      )) : <div className="empty-state"><Search size={22} /><span>{emptyMessage}</span></div>}
    </div>
  );
}

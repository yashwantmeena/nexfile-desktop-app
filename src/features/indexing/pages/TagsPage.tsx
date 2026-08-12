import { useMemo, useState } from "react";
import { Tag } from "lucide-react";
import { AssetGrid } from "@/features/indexing/components/AssetGrid";
import { PageHeader } from "@/features/indexing/components/PageHeader";
import { archiveAssets } from "@/features/indexing/data/archive-data";

export function TagsPage({ query }: { query: string }) {
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const tags = useMemo(() => Array.from(new Set(archiveAssets.flatMap((asset) => asset.tags))).sort(), []);
  const normalized = query.trim().toLowerCase();
  const matchingTags = tags.filter((tag) => !normalized || tag.toLowerCase().includes(normalized));
  const visibleAssets = archiveAssets.filter((asset) => !selectedTag || asset.tags.includes(selectedTag));

  return (
    <div className="page-view">
      <PageHeader title="Tags" description="Browse your library by subject, style, or place." />
      <section className="tag-browser panel">
        <div className="tag-browser-heading"><Tag size={17} /><strong>Browse tags</strong><span>{tags.length} tags</span></div>
        <div className="tag-cloud">
          <button className={!selectedTag ? "selected" : ""} onClick={() => setSelectedTag(null)}>All</button>
          {matchingTags.map((tag) => <button key={tag} className={selectedTag === tag ? "selected" : ""} onClick={() => setSelectedTag(tag)}>{tag}</button>)}
        </div>
      </section>
      <section className="tag-results">
        <h2>{selectedTag ? `${selectedTag} items` : "All tagged items"}</h2>
        <AssetGrid assets={visibleAssets} emptyMessage="No items use this tag yet" />
      </section>
    </div>
  );
}

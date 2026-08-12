import { useMemo, useState } from "react";
import { Tabs, TabsList, TabsTrigger } from "@/components/Tabs/Tabs";
import { AssetGrid } from "@/features/indexing/components/AssetGrid";
import { archiveAssets } from "@/features/indexing/data/archive-data";
import type { AssetFilter } from "@/features/indexing/types";

interface RecentIngestsProps {
  query: string;
  showHeading?: boolean;
}

const filters: AssetFilter[] = ["All", "Photos", "Documents"];

export function RecentIngests({ query, showHeading = true }: RecentIngestsProps) {
  const [filter, setFilter] = useState<AssetFilter>("All");
  const visibleAssets = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return archiveAssets.filter((asset) => {
      const matchesType = filter === "All" || asset.type === filter;
      const matchesQuery = !normalized || asset.title.toLowerCase().includes(normalized) || asset.tags.some((tag) => tag.toLowerCase().includes(normalized));
      return matchesType && matchesQuery;
    });
  }, [filter, query]);

  return (
    <section className="recent-section">
      <div className={showHeading ? "section-heading" : "section-heading filters-only"}>
        {showHeading && <h2>Recently Added</h2>}
        <Tabs value={filter} onValueChange={(value) => setFilter(value as AssetFilter)}>
          <TabsList className="segmented-control" aria-label="Show recently added items by type">
            {filters.map((item) => <TabsTrigger key={item} value={item}>{item}</TabsTrigger>)}
          </TabsList>
        </Tabs>
      </div>
      <AssetGrid assets={visibleAssets} />
    </section>
  );
}

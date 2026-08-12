import { CollectionsPage } from "@/features/indexing/pages/CollectionsPage";
import { HomePage } from "@/features/indexing/pages/IndexingHomePage";
import { RecentlyAddedPage } from "@/features/indexing/pages/RecentlyAddedPage";
import { TagsPage } from "@/features/indexing/pages/TagsPage";
import { StoragePage } from "@/pages/Storage/StoragePage";
import type { CollectionSummary, NavigationView } from "@/features/indexing/types";

interface ArchiveDashboardProps {
  activeView: NavigationView;
  query: string;
  collections: CollectionSummary[];
  onNotice: (message: string) => void;
}

export function ArchiveDashboard({ activeView, query, collections, onNotice }: ArchiveDashboardProps) {
  return (
    <main className="dashboard" key={activeView}>
      {activeView === "Home" && <HomePage query={query} onNotice={onNotice} />}
      {activeView === "Recently Added" && <RecentlyAddedPage query={query} />}
      {activeView === "Collections" && <CollectionsPage collections={collections} query={query} />}
      {activeView === "Tags" && <TagsPage query={query} />}
      {activeView === "Storage" && <StoragePage onNotice={onNotice} />}
    </main>
  );
}

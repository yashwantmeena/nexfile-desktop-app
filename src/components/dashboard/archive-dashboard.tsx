import { CollectionsPage } from "@/components/pages/collections-page";
import { HomePage } from "@/components/pages/home-page";
import { RecentlyAddedPage } from "@/components/pages/recently-added-page";
import { StoragePage } from "@/components/pages/storage-page";
import { TagsPage } from "@/components/pages/tags-page";
import type { CollectionSummary, NavigationView } from "@/types/archive";

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

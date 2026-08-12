import { Search } from "lucide-react";
import { SearchPanel } from "@/features/search";
import type { NavItem } from "@/types/navigation";
import type { SearchControls } from "@/types/search";

interface SearchPageProps extends SearchControls {
  activeView: Exclude<NavItem, "Home" | "Storage" | "Settings">;
  onNotice: (message: string) => void;
}

export function SearchPage({ activeView, query, activeTag, onQueryChange, onTagChange, onNotice }: SearchPageProps) {
  return (
    <main className="dashboard">
      <SearchPanel query={query} activeTag={activeTag} onQueryChange={onQueryChange} onTagChange={onTagChange} />
      <section className="secondary-page">
        <div className="secondary-page-card"><Search /><h1>{activeView}</h1><p>Search stays private and runs against your on-device index.</p><button type="button" onClick={() => onNotice(`${activeView} refreshed`)}>Refresh view</button></div>
      </section>
    </main>
  );
}

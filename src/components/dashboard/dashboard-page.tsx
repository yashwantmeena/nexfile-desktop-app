import { useMemo, useState } from "react";
import { HardDrive, Search } from "lucide-react";
import { OverviewCards } from "@/components/dashboard/overview-cards";
import { PrivacyBanner } from "@/components/dashboard/privacy-banner";
import { RecentFilesTable } from "@/components/dashboard/recent-files-table";
import { SearchPanel } from "@/components/dashboard/search-panel";
import { StoragePanels } from "@/components/dashboard/storage-panels";
import { recentFiles } from "@/data/localmind-data";
import type { NavItem } from "@/types/localmind";

interface DashboardPageProps {
  activeView: NavItem;
  onNotice: (message: string) => void;
}

export function DashboardPage({ activeView, onNotice }: DashboardPageProps) {
  const [query, setQuery] = useState("");
  const [activeTag, setActiveTag] = useState<string | null>(null);
  const [sensitivity, setSensitivity] = useState(68);

  const filteredFiles = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return recentFiles.filter((file) => {
      const matchesQuery = !needle || [file.name, file.location, ...file.tags].some((value) => value.toLowerCase().includes(needle));
      const matchesTag = !activeTag || file.tags.some((tag) => tag.includes(activeTag.replace("-docs", "")));
      return matchesQuery && matchesTag;
    });
  }, [activeTag, query]);

  if (activeView !== "Home") {
    const Icon = activeView === "Storage" ? HardDrive : Search;
    return (
      <main className="dashboard secondary-page">
        <div className="secondary-page-card"><Icon /><h1>{activeView}</h1><p>This area uses your private, on-device LocalMind index.</p><button type="button" onClick={() => onNotice(`${activeView} refreshed`)}>Refresh view</button></div>
      </main>
    );
  }

  return (
    <main className="dashboard">
      <SearchPanel query={query} activeTag={activeTag} onQueryChange={setQuery} onTagChange={setActiveTag} />
      <div className="dashboard-columns">
        <div className="main-column">
          <OverviewCards onAction={onNotice} />
          <RecentFilesTable files={filteredFiles} onNotice={onNotice} />
        </div>
        <StoragePanels sensitivity={sensitivity} onSensitivityChange={setSensitivity} />
      </div>
      <PrivacyBanner onLearnMore={() => onNotice("Everything stays on this device — including AI tagging")} />
    </main>
  );
}

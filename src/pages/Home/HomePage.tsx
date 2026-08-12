import { useMemo, useState } from "react";
import { filterFiles, SearchPanel } from "@/features/search";
import { OverviewCards } from "@/pages/Home/components/OverviewCards";
import { PrivacyBanner } from "@/pages/Home/components/PrivacyBanner";
import { RecentFilesTable } from "@/pages/Home/components/RecentFilesTable";
import { StoragePanels } from "@/pages/Home/components/StoragePanels";
import { recentFiles } from "@/pages/Home/data/home-data";
import type { SearchControls } from "@/types/search";

interface HomePageProps extends SearchControls {
  onNotice: (message: string) => void;
}

export function HomePage({ query, activeTag, onQueryChange, onTagChange, onNotice }: HomePageProps) {
  const [sensitivity, setSensitivity] = useState(68);
  const filteredFiles = useMemo(() => filterFiles(recentFiles, { query, activeTag }), [activeTag, query]);

  return (
    <main className="dashboard">
      <SearchPanel query={query} activeTag={activeTag} onQueryChange={onQueryChange} onTagChange={onTagChange} />
      <div className="dashboard-columns">
        <div className="main-column">
          <OverviewCards onAction={onNotice} />
          <RecentFilesTable files={filteredFiles} onNotice={onNotice} />
        </div>
        <StoragePanels sensitivity={sensitivity} onSensitivityChange={setSensitivity} />
      </div>
      <PrivacyBanner onLearnMore={() => onNotice("Everything stays on this device, including AI tagging")} />
    </main>
  );
}

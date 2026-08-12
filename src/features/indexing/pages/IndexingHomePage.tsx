import { IngestionLog } from "@/features/indexing/components/IngestionLog";
import { MetricGrid } from "@/features/indexing/components/MetricGrid";
import { RecentIngests } from "@/features/indexing/components/RecentIngests";
import { VaultOverview } from "@/features/indexing/components/VaultOverview";

interface HomePageProps {
  query: string;
  onNotice: (message: string) => void;
}

export function HomePage({ query, onNotice }: HomePageProps) {
  return (
    <div className="page-view">
      <VaultOverview onNotice={onNotice} />
      <MetricGrid />
      <RecentIngests query={query} />
      <IngestionLog />
    </div>
  );
}

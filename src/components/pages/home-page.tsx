import { IngestionLog } from "@/components/dashboard/ingestion-log";
import { MetricGrid } from "@/components/dashboard/metric-grid";
import { RecentIngests } from "@/components/dashboard/recent-ingests";
import { VaultOverview } from "@/components/dashboard/vault-overview";

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

import { IngestionLog } from "@/components/dashboard/ingestion-log";
import { RecentIngests } from "@/components/dashboard/recent-ingests";
import { PageHeader } from "@/components/pages/page-header";

export function RecentlyAddedPage({ query }: { query: string }) {
  return (
    <div className="page-view">
      <PageHeader title="Recently Added" description="See the photos and files you added most recently." />
      <RecentIngests query={query} showHeading={false} />
      <IngestionLog />
    </div>
  );
}

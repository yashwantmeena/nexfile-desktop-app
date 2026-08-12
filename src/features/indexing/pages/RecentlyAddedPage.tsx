import { IngestionLog } from "@/features/indexing/components/IngestionLog";
import { RecentIngests } from "@/features/indexing/components/RecentIngests";
import { PageHeader } from "@/features/indexing/components/PageHeader";

export function RecentlyAddedPage({ query }: { query: string }) {
  return (
    <div className="page-view">
      <PageHeader title="Recently Added" description="See the photos and files you added most recently." />
      <RecentIngests query={query} showHeading={false} />
      <IngestionLog />
    </div>
  );
}

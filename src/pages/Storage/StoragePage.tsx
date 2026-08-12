import { StorageDashboard } from "@/features/storage";

export function StoragePage({ onNotice }: { onNotice: (message: string) => void }) {
  return <StorageDashboard onNotice={onNotice} />;
}

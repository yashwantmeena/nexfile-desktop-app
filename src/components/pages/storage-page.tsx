import { FileText, Image, PackageOpen } from "lucide-react";
import { PageHeader } from "@/components/pages/page-header";
import { VaultOverview } from "@/components/dashboard/vault-overview";

const storageGroups = [
  { label: "Photos", value: "26,8 GB", percent: 63, icon: Image },
  { label: "Documents", value: "10,2 GB", percent: 24, icon: FileText },
  { label: "Other files", value: "5,4 GB", percent: 13, icon: PackageOpen },
];

export function StoragePage({ onNotice }: { onNotice: (message: string) => void }) {
  return (
    <div className="page-view">
      <PageHeader title="Storage" description="See how space is being used on this device." />
      <VaultOverview onNotice={onNotice} />
      <section className="panel storage-breakdown">
        <h2>What uses your space</h2>
        <div className="storage-group-list">
          {storageGroups.map(({ label, value, percent, icon: Icon }) => (
            <div className="storage-group" key={label}>
              <div className="storage-group-icon"><Icon size={17} /></div>
              <div className="storage-group-info"><strong>{label}</strong><div><span style={{ width: `${percent}%` }} /></div></div>
              <span>{value}</span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

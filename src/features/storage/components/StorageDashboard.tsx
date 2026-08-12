import { useEffect, useState } from "react";
import * as Slider from "radix-ui/slider";
import * as Switch from "radix-ui/switch";
import { Database, HardDrive, RefreshCw, Server, Zap } from "lucide-react";
import { Button } from "@/components/Button/Button";
import { useStorage } from "@/features/storage/hooks/useStorage";

const GB = 1_000_000_000;
const TB = 1_000 * GB;

function formatBytes(bytes: number) {
  if (bytes >= TB) return `${(bytes / TB).toFixed(1)} TB`;
  return `${Math.round(bytes / GB)} GB`;
}

export function StorageDashboard({ onNotice }: { onNotice: (message: string) => void }) {
  const {
    allocations,
    devices,
    error,
    loading,
    refresh,
    savingVolumeId,
    setDriveEnabled,
    updateQuota,
  } = useStorage();
  const [draftQuotas, setDraftQuotas] = useState<Record<string, number>>({});
  const allocationByVolume = new Map(allocations.map((allocation) => [allocation.volumeId, allocation]));

  useEffect(() => {
    setDraftQuotas(Object.fromEntries(allocations.map((allocation) => [allocation.volumeId, allocation.quotaBytes])));
  }, [allocations]);

  const total = devices.reduce((sum, device) => sum + device.totalBytes, 0);
  const allocated = allocations.reduce((sum, allocation) => sum + allocation.quotaBytes, 0);
  const appUsed = allocations.reduce((sum, allocation) => sum + allocation.vaultUsedBytes, 0);
  const remaining = Math.max(0, allocated - appUsed);
  const summary = [
    { label: "Total capacity", value: formatBytes(total), detail: `across ${devices.length} drives`, icon: HardDrive, tone: "indigo" },
    { label: "Allocated", value: formatBytes(allocated), detail: `${total ? Math.round(allocated / total * 100) : 0}% of total`, icon: Database, tone: "violet" },
    { label: "App used", value: formatBytes(appUsed), detail: `${allocated ? Math.round(appUsed / allocated * 100) : 0}% of allocated`, icon: Zap, tone: "amber" },
    { label: "App remaining", value: formatBytes(remaining), detail: `${allocated ? Math.round(remaining / allocated * 100) : 0}% free`, icon: Server, tone: "teal" },
  ];

  return (
    <main className="dashboard storage-feature-page">
      <header className="storage-feature-heading">
        <div><h1>Storage Management</h1><p>Manage drives, allocations, and monitor usage across all storage devices.</p></div>
        <Button onClick={() => { void refresh(); onNotice("Storage scan started"); }} disabled={loading}><RefreshCw className={loading ? "spin" : ""} />Scan Drives</Button>
      </header>

      <section className="storage-feature-summary" aria-label="Storage summary">
        {summary.map(({ label, value, detail, icon: Icon, tone }) => <article className="metric-card" key={label}><span className={`metric-icon ${tone}`}><Icon /></span><strong>{value}</strong><p>{label} · {detail}</p></article>)}
      </section>

      <section className="storage-drive-panel">
        <header><h2>Drive Management</h2><span>{devices.length} drives</span></header>
        {error && <p className="storage-error" role="alert">{error}</p>}
        {!loading && devices.length === 0 && <p className="storage-empty">No storage devices were detected.</p>}
        <div className="storage-drive-list">
          {devices.map((device) => {
            const allocation = allocationByVolume.get(device.volumeId);
            const used = allocation?.vaultUsedBytes ?? 0;
            const quota = draftQuotas[device.volumeId] ?? allocation?.quotaBytes ?? 0;
            const percent = quota ? Math.min(100, Math.round(used / quota * 100)) : 0;
            const maximumQuota = Math.max(used, device.availableBytes + used, GB);
            const Icon = device.kind === "hdd" ? Server : HardDrive;
            const saving = savingVolumeId === device.volumeId;

            return (
              <article className="storage-drive-row" key={device.volumeId}>
                <span className={`storage-drive-icon ${device.kind}`}><Icon /></span>
                <div className="storage-drive-info">
                  <strong>{device.hardwareModel || device.volumeLabel || device.name}</strong>
                  <span>{device.mountPoint} · {device.fileSystem} · {formatBytes(device.totalBytes)}</span>
                  <div><span style={{ width: `${percent}%` }} /></div>
                </div>

                <div className="storage-drive-control">
                  {allocation ? <>
                    <div><span>Allocation limit</span><strong>{formatBytes(quota)}</strong></div>
                    <Slider.Root
                      className="storage-allocation-slider"
                      value={[quota / GB]}
                      min={Math.max(1, Math.ceil(used / GB))}
                      max={Math.max(1, Math.floor(maximumQuota / GB))}
                      step={10}
                      disabled={saving}
                      aria-label={`${device.name} allocation limit`}
                      onValueChange={([value]) => setDraftQuotas((current) => ({ ...current, [device.volumeId]: value * GB }))}
                      onValueCommit={([value]) => { void updateQuota(device.volumeId, value * GB).then((saved) => { if (saved) onNotice(`${device.name} allocation saved`); }); }}
                    >
                      <Slider.Track className="storage-allocation-track"><Slider.Range className="storage-allocation-range" /></Slider.Track>
                      <Slider.Thumb className="storage-allocation-thumb" />
                    </Slider.Root>
                  </> : <span className="storage-not-allocated">Not allocated</span>}
                </div>

                <div className="storage-drive-toggle">
                  <Switch.Root
                    className="storage-drive-switch"
                    checked={Boolean(allocation)}
                    disabled={saving || device.isReadOnly}
                    aria-label={`${allocation ? "Disable" : "Enable"} ${device.name}`}
                    onCheckedChange={(checked) => { void setDriveEnabled(device.volumeId, checked).then((saved) => { if (saved) onNotice(`${device.name} ${checked ? "enabled" : "disabled"}`); }); }}
                  >
                    <Switch.Thumb className="storage-drive-switch-thumb" />
                  </Switch.Root>
                </div>
              </article>
            );
          })}
        </div>
      </section>
    </main>
  );
}

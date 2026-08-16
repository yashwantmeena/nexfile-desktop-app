import { Database, FileText, Gauge, HardDrive, RefreshCw } from "lucide-react";
import type { StorageData } from "../types/storage";

interface StorageOverviewProps { data: StorageData; }

function formatBytes(bytes: number): { value: string; unit: string } {
  if (bytes <= 0) return { value: "0", unit: "B" };
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const unitIndex = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** unitIndex;
  return { value: value.toLocaleString(undefined, { maximumFractionDigits: 1 }), unit: units[unitIndex] };
}

export function StorageOverview({ data }: StorageOverviewProps) {
  const total = formatBytes(data.totalBytes);
  const available = formatBytes(data.availableBytes);
  const appLimit = formatBytes(data.appLimitBytes);
  const appUsed = formatBytes(data.appUsedBytes);
  const mountedCount = data.drives.filter((drive) => drive.isConnected && drive.isMounted).length;
  const unmountedCount = data.drives.filter((drive) => drive.isConnected && !drive.isMounted).length;
  const availablePercent = data.totalBytes === 0 ? 0 : Math.round(data.availableBytes * 100 / data.totalBytes);
  const appUsedPercent = data.appLimitBytes === 0 ? 0 : Math.round(data.appUsedBytes * 100 / data.appLimitBytes);
  const cards = [
    { label:"Total Capacity", ...total, caption:`Combined across ${data.drivesDetected} drives`, icon:Database },
    { label:"Available Storage", ...available, caption:`${availablePercent}% of total capacity free`, icon:HardDrive },
    { label:"Drives Detected", value:data.drivesDetected.toLocaleString(), unit:"", caption:`${mountedCount} mounted • ${unmountedCount} unmounted`, icon:RefreshCw },
    { label:"Files Indexed", value:data.fileIndexed.toLocaleString(), unit:"", caption:"Across all saved drives", icon:FileText },
    { label:"App Storage Limit", ...appLimit, caption:"Maximum storage allowed", icon:Database },
    { label:"App Storage Usage", ...appUsed, caption:`${appUsedPercent}% of app limit`, icon:Gauge },
  ];

  return <section className="storage-overview">{cards.map(({label,value,unit,caption,icon:Icon}) => <article className="storage-metric-card" key={label}><span className="storage-metric-icon"><Icon /></span><div><p>{label}</p><strong>{value} <small>{unit}</small></strong><span>{caption}</span></div></article>)}</section>;
}

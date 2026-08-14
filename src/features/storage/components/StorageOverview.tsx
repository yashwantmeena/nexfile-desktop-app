import { Database, FileText, Gauge, HardDrive, RefreshCw } from "lucide-react";

const cards = [
  { label:"Total Capacity", value:"7.25", unit:"TB", caption:"Combined across 4 drives", icon:Database },
  { label:"Available Storage", value:"2.73", unit:"TB", caption:"38% of total capacity free", icon:HardDrive },
  { label:"Drives Detected", value:"4", unit:"", caption:"3 mounted • 1 unmounted", icon:RefreshCw },
  { label:"Files Indexed", value:"12,345", unit:"", caption:"72% of 17,204 files", icon:FileText },
  { label:"App Storage Limit", value:"500", unit:"GB", caption:"Maximum storage allowed", icon:Database },
  { label:"App Storage Usage", value:"491", unit:"GB", caption:"98% of app limit", icon:Gauge },
] as const;

export function StorageOverview() {
  return <section className="storage-overview">{cards.map(({label,value,unit,caption,icon:Icon}) => <article className="storage-metric-card" key={label}><span className="storage-metric-icon"><Icon /></span><div><p>{label}</p><strong>{value} <small>{unit}</small></strong><span>{caption}</span></div></article>)}</section>;
}

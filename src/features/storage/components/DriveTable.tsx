import { ArrowDown, ArrowUp, EllipsisVertical, HardDrive, Plug, Trash2, Unplug } from "lucide-react";
import type { StorageDrive } from "../types/storage";

interface DriveTableProps {
  title:string; description:string; drives:StorageDrive[];
  onMount?:(deviceId:string|null,partitionName:string)=>void; onUnmount?:(id:string)=>void;
  onRemove?:(id:string)=>void;
  onMovePriority?:(id:string,direction:"up"|"down")=>void;
}

function formatCapacity(value: number | undefined): string {
  if (value === undefined) return "—";
  if (value <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const unitIndex = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  const formatted = value / 1024 ** unitIndex;
  return `${formatted.toLocaleString(undefined, { maximumFractionDigits: 1 })} ${units[unitIndex]}`;
}

function bytesToGigabytes(value: number | undefined): number | undefined {
  if (value === undefined) return undefined;
  return Number((value / 1024 ** 3).toFixed(2));
}

function closeActionMenu(button: HTMLButtonElement): void {
  button.closest("details")?.removeAttribute("open");
}

export function DriveTable({ title, description, drives, onMount, onUnmount, onRemove, onMovePriority }: DriveTableProps) {
  return (
    <section className="drive-panel">
      <header><div><h2>{title} ({drives.length})</h2><p>{description}</p></div></header>
      <div className="drive-table-scroll"><div className="drive-table">
        <div className="drive-table-head"><span>Drive / Partition</span><span>Capacity</span><span>System Used</span><span>App Used</span><span>Free</span><span>Limit</span><span>Priority</span><span>Actions</span></div>
        {drives.map((drive,index) => <div className="drive-row" key={`${drive.driveId || drive.partitionName}-${index}`}>
          <div className="drive-identity"><span className="drive-icon"><HardDrive /></span><div><strong>{drive.driveName}</strong><small>{drive.partitionName}<i>•</i>{drive.fileSystem}</small></div></div>
          <strong>{drive.isConnected ? formatCapacity(drive.totalBytes) : "—"}</strong>
          <div className="drive-usage">{drive.isMounted && drive.systemUsedBytes !== undefined ? <><strong>{formatCapacity(drive.systemUsedBytes)}</strong><small>{drive.systemUsedPercent ?? 0}%</small><div><i style={{width:`${drive.systemUsedPercent ?? 0}%`}} /></div></> : <span>—</span>}</div>
          <div className="drive-usage app-drive-usage">{drive.isMounted && drive.appUsedBytes !== undefined ? <><strong>{formatCapacity(drive.appUsedBytes)}</strong><small>{drive.appUsedPercent ?? 0}% of limit</small><div><i style={{width:`${drive.appUsedPercent ?? 0}%`}} /></div></> : <span>—</span>}</div>
          <strong>{drive.isMounted ? formatCapacity(drive.availableBytes) : "—"}</strong>
          {drive.isMounted ? <div className="limit-cell"><label className="limit-input"><input type="number" min="0" step="1" defaultValue={bytesToGigabytes(drive.appLimitBytes)} placeholder="No limit" aria-label={`${drive.driveName} ${drive.partitionName} storage limit in GB`} /><span>GB</span><b>/</b><em>{formatCapacity(drive.totalBytes)}</em></label></div> : <span className="limit-empty">—</span>}
          {drive.isMounted ? <div className="priority-control"><button disabled={index===0} onClick={()=>onMovePriority?.(drive.driveId,"up")} aria-label={`Move ${drive.partitionName} up`} title="Move up"><ArrowUp /></button><button disabled={index===drives.length-1} onClick={()=>onMovePriority?.(drive.driveId,"down")} aria-label={`Move ${drive.partitionName} down`} title="Move down"><ArrowDown /></button></div> : <span className="priority-empty">—</span>}
          <div className="drive-actions">{drive.isMounted ? <details className="drive-action-menu"><summary aria-label={`Actions for ${drive.partitionName}`} title="Drive actions"><EllipsisVertical /></summary><div className="drive-action-options"><button onClick={(event)=>{closeActionMenu(event.currentTarget);onUnmount?.(drive.driveId);}}><Unplug />Unmount</button><button className="delete-drive-action" onClick={(event)=>{closeActionMenu(event.currentTarget);onRemove?.(drive.driveId);}}><Trash2 />Remove / Delete</button></div></details> : drive.isConnected ? <details className="drive-action-menu"><summary aria-label={`Actions for ${drive.partitionName}`} title="Drive actions"><EllipsisVertical /></summary><div className="drive-action-options"><button onClick={(event)=>{closeActionMenu(event.currentTarget);onMount?.(drive.deviceId,drive.partitionName);}}><Plug />Mount</button></div></details> : <span className="unavailable-label">Unavailable</span>}</div>
        </div>)}
      </div></div>
    </section>
  );
}

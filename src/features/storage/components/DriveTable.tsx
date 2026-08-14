import { ArrowDown, ArrowUp, HardDrive } from "lucide-react";
import type { StorageDrive } from "../types/storage";

interface DriveTableProps {
  title:string; description:string; drives:StorageDrive[];
  onMount?:(id:string)=>void; onUnmount?:(id:string)=>void;
  onMovePriority?:(id:string,direction:"up"|"down")=>void;
}

export function DriveTable({ title, description, drives, onMount, onUnmount, onMovePriority }: DriveTableProps) {
  return (
    <section className="drive-panel">
      <header><div><h2>{title} ({drives.length})</h2><p>{description}</p></div></header>
      <div className="drive-table-scroll"><div className="drive-table">
        <div className="drive-table-head"><span>Drive / Partition</span><span>Capacity</span><span>System Used</span><span>App Used</span><span>Free</span><span>Limit</span><span>Priority</span><span>Actions</span></div>
        {drives.map((drive,index) => <div className="drive-row" key={drive.id}>
          <div className="drive-identity"><span className="drive-icon"><HardDrive /></span><div><strong>{drive.driveName}</strong><small>{drive.partitionName}<i>•</i>{drive.fileSystem}</small></div></div>
          <strong>{drive.total}</strong>
          <div className="drive-usage">{drive.status === "mounted" && drive.used ? <><strong>{drive.used}</strong><small>{drive.usedPercent}%</small><div><i style={{width:`${drive.usedPercent}%`}} /></div></> : <span>—</span>}</div>
          <div className="drive-usage app-drive-usage">{drive.status === "mounted" && drive.appUsed ? <><strong>{drive.appUsed}</strong><small>{drive.appUsedPercent}% of limit</small><div><i style={{width:`${drive.appUsedPercent}%`}} /></div></> : <span>—</span>}</div>
          <strong>{drive.status === "mounted" ? drive.available ?? "—" : "—"}</strong>
          <div className="limit-cell"><label className="limit-input"><input type="number" min="0" step="1" defaultValue={drive.appLimitValue} placeholder="No limit" aria-label={`${drive.driveName} ${drive.partitionName} storage limit in GB`} /><span>GB</span>{drive.status === "mounted" && <><b>/</b><em>{drive.totalGb} GB</em></>}</label></div>
          {drive.status === "mounted" ? <div className="priority-control"><button disabled={index===0} onClick={()=>onMovePriority?.(drive.id,"up")} aria-label={`Move ${drive.partitionName} up`} title="Move up"><ArrowUp /></button><button disabled={index===drives.length-1} onClick={()=>onMovePriority?.(drive.id,"down")} aria-label={`Move ${drive.partitionName} down`} title="Move down"><ArrowDown /></button></div> : <span className="priority-empty">—</span>}
          <div className="drive-actions">{drive.status === "mounted" ? <button className="unmount-action" onClick={()=>onUnmount?.(drive.id)}>Unmount</button> : <button onClick={()=>onMount?.(drive.id)}>Mount</button>}</div>
        </div>)}
      </div></div>
    </section>
  );
}

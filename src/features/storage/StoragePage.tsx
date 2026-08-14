import { useState } from "react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import type { AppNavigationItem } from "@/types/navigation";
import { DriveTable } from "./components/DriveTable";
import { StorageHeader } from "./components/StorageHeader";
import { StorageOverview } from "./components/StorageOverview";
import { mountedDrives as initialMounted, unmountedDrives as initialUnmounted } from "./data/storage-data";
import "./storage.css";

interface StoragePageProps { activeNavigation:AppNavigationItem; onNavigationChange:(item:AppNavigationItem)=>void; }

export function StoragePage({ activeNavigation, onNavigationChange }: StoragePageProps) {
  const [mountedDrives,setMountedDrives] = useState(initialMounted);
  const [unmountedDrives,setUnmountedDrives] = useState(initialUnmounted);
  const [hasUnsavedChanges,setHasUnsavedChanges] = useState(false);
  const mountDrive = (id:string) => { const drive=unmountedDrives.find(item=>item.id===id); if(!drive)return; setUnmountedDrives(items=>items.filter(item=>item.id!==id)); setMountedDrives(items=>[...items,{...drive,priority:items.length+1,status:"mounted",available:drive.total}]); setHasUnsavedChanges(true); };
  const unmountDrive = (id:string) => { const drive=mountedDrives.find(item=>item.id===id); if(!drive)return; setMountedDrives(items=>items.filter(item=>item.id!==id).map((item,index)=>({...item,priority:index+1}))); setUnmountedDrives(items=>[...items,{...drive,status:"unmounted"}]); setHasUnsavedChanges(true); };
  const movePriority = (id:string,direction:"up"|"down") => { setMountedDrives(items=>{ const currentIndex=items.findIndex(item=>item.id===id); const targetIndex=direction==="up"?currentIndex-1:currentIndex+1; if(currentIndex<0||targetIndex<0||targetIndex>=items.length)return items; const reordered=[...items]; [reordered[currentIndex],reordered[targetIndex]]=[reordered[targetIndex],reordered[currentIndex]]; return reordered.map((item,index)=>({...item,priority:index+1})); }); setHasUnsavedChanges(true); };
  return <div className="nexfile-app"><AppSidebar activeItem={activeNavigation} onActiveItemChange={onNavigationChange} /><main className="nf-main storage-main"><div className="storage-page"><StorageHeader hasUnsavedChanges={hasUnsavedChanges} onSave={()=>setHasUnsavedChanges(false)} /><StorageOverview /><DriveTable title="Mounted Drives" description="Use the arrows to set storage priority. Higher drives are used first." drives={mountedDrives} onUnmount={unmountDrive} onMovePriority={movePriority} /><DriveTable title="Unmounted Drives" description="Connected partitions that are not mounted or are excluded from use." drives={unmountedDrives} onMount={mountDrive} /></div></main></div>;
}

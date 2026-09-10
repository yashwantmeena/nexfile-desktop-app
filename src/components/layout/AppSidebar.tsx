import { CollectionActions } from "./CollectionActions";
import { Clock3, FolderOpen, HardDrive, Files, Plus, Settings, ShieldCheck, Star, Trash2 } from "lucide-react";
import { useCollections } from "./CollectionsProvider";
import type { AppNavigationItem } from "@/types/navigation";
import { BackgroundActivities } from "@/features/background-activity/components/BackgroundActivities";

interface AppSidebarProps {
  activeItem: AppNavigationItem;
  onActiveItemChange: (item: AppNavigationItem) => void;
}

const navigation = [
  ["Search", Files],
  ["Recent", Clock3],
  ["Favorites", Star],
  ["Trash", Trash2],
] as const;

export function AppSidebar({ activeItem, onActiveItemChange }: AppSidebarProps) {
  const { collections, selected, select, openCreate, loadError, refresh } = useCollections();
  return (
    <aside className="nf-sidebar">
      <div className="nf-brand"><span>N</span><strong>NexFile</strong></div>
      <nav className="nf-nav" aria-label="Primary">
        {navigation.map(([label, Icon]) => (
          <button key={label} className={(activeItem === label || (label === "Search" && activeItem === "Collections" && !selected)) ? "active" : ""} onClick={() => { select(null); onActiveItemChange(label); }}>
            <Icon /><span>{label === "Search" ? "All files" : label}</span>
          </button>
        ))}
      </nav>
      <div className="collections-heading"><p className="sidebar-label">Collections</p><button type="button" aria-label="Create collection" onClick={openCreate}><Plus size={15}/></button></div>
      <nav className="nf-nav collection-nav" aria-label="Collections">
        {collections.map(item => <div className="collection-sidebar-item" key={item.id}><button title={item.name} aria-current={activeItem === "Collections" && selected?.id === item.id ? "page" : undefined} className={activeItem === "Collections" && selected?.id === item.id ? "active" : ""} onClick={() => { select(item.id); onActiveItemChange("Collections"); }}><FolderOpen/><span>{item.name}</span></button><CollectionActions id={item.id} name={item.name}/></div>)}
        {loadError ? <div className="collections-hint" role="alert">{loadError}<button type="button" onClick={refresh}>Retry</button></div> : !collections.length && <p className="collections-hint">Your collections will appear here.</p>}
      </nav>
      <p className="sidebar-label activity-label">Background activity</p>
      <BackgroundActivities />
      <div className="sidebar-bottom">
        <button onClick={() => onActiveItemChange("Settings")}><Settings /><span>Settings</span></button>
        <button className={activeItem === "Storage" ? "active" : ""} aria-current={activeItem === "Storage" ? "page" : undefined} onClick={() => onActiveItemChange("Storage")}><HardDrive /><span>Storage</span></button>
        <div className="local-card"><ShieldCheck /><div><strong>100% Local</strong><p>Your files never leave<br />this device.</p></div></div>
      </div>
    </aside>
  );
}



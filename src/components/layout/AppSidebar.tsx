import { Clock3, FolderOpen, HardDrive, Search, Settings, ShieldCheck, Star, Trash2 } from "lucide-react";
import type { AppNavigationItem } from "@/types/navigation";

interface AppSidebarProps {
  activeItem: AppNavigationItem;
  onActiveItemChange: (item: AppNavigationItem) => void;
}

const navigation = [
  ["Search", Search],
  ["Recent", Clock3],
  ["Favorites", Star],
  ["Collections", FolderOpen],
  ["Storage", HardDrive],
] as const;

export function AppSidebar({ activeItem, onActiveItemChange }: AppSidebarProps) {
  return (
    <aside className="nf-sidebar">
      <div className="nf-brand"><span>N</span><strong>NexFile</strong></div>
      <nav className="nf-nav" aria-label="Primary">
        {navigation.map(([label, Icon]) => (
          <button key={label} className={activeItem === label ? "active" : ""} onClick={() => onActiveItemChange(label)}>
            <Icon /><span>{label}</span>
          </button>
        ))}
      </nav>
      <p className="sidebar-label status-label">Status</p>
      <div className="index-status">
        <div><span>Indexing</span><strong>72%</strong></div>
        <div className="progress"><i /></div>
        <p>Scanning... /Projects</p>
        <span>12,345 of 17,204 files</span>
      </div>
      <div className="sidebar-bottom">
        <button onClick={() => onActiveItemChange("Settings")}><Settings /><span>Settings</span></button>
        <button onClick={() => onActiveItemChange("Trash")}><Trash2 /><span>Trash</span></button>
        <div className="local-card"><ShieldCheck /><div><strong>100% Local</strong><p>Your files never leave<br />this device.</p></div></div>
      </div>
    </aside>
  );
}

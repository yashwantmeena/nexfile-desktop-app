import {
  Clock3,
  Files,
  Folder,
  HardDrive,
  House,
  Search,
  Settings,
  ShieldCheck,
  X,
} from "lucide-react";
import { Button } from "@/components/Button/Button";
import type { NavItem } from "@/types/navigation";

interface AppSidebarProps {
  activeItem: NavItem;
  open: boolean;
  onActiveItemChange: (item: NavItem) => void;
  onOpenChange: (open: boolean) => void;
  onNotice: (message: string) => void;
}

const primaryNav: Array<{ label: NavItem; icon: typeof House }> = [
  { label: "Home", icon: House },
  { label: "Files", icon: Folder },
  { label: "Recent Access", icon: Clock3 },
];

const secondaryNav: Array<{ label: NavItem; icon: typeof House }> = [
  { label: "AI Search", icon: Search },
  { label: "Storage", icon: HardDrive },
  { label: "Settings", icon: Settings },
];

export function Sidebar({ activeItem, open, onActiveItemChange, onOpenChange, onNotice }: AppSidebarProps) {
  const selectItem = (item: NavItem) => {
    setTimeout(() => onOpenChange(false), 0);
    if (item === "Home") {
      onActiveItemChange(item);
      return;
    }
    onNotice(`${item} is ready for your local library`);
    onActiveItemChange(item);
  };

  const renderNav = (items: typeof primaryNav) => items.map(({ label, icon: Icon }) => (
    <Button
      key={label}
      type="button"
      variant="ghost"
      className={activeItem === label ? "nav-item active" : "nav-item"}
      onClick={() => selectItem(label)}
    >
      <Icon aria-hidden="true" />
      <span>{label}</span>
    </Button>
  ));

  return (
    <>
      <aside className={open ? "sidebar open" : "sidebar"} aria-label="Application sidebar">
        <div className="brand-row">
          <span className="brand-mark" aria-hidden="true"><ShieldCheck /></span>
          <span className="brand-copy"><strong>LocalMind</strong><small>Local AI Storage</small></span>
          <Button variant="ghost" size="icon-sm" className="mobile-close" aria-label="Close navigation" onClick={() => onOpenChange(false)}><X /></Button>
        </div>

        <nav className="main-nav" aria-label="Main navigation">
          {renderNav(primaryNav)}
          <p>AI Search</p>
          {renderNav(secondaryNav)}
        </nav>

        <div className="local-processing-card">
          <span className="processing-icon" aria-hidden="true"><Files /><ShieldCheck /></span>
          <p>100% Local Processing</p>
          <strong>Your files never leave this device</strong>
        </div>
      </aside>
      {open && <button type="button" className="backdrop" aria-label="Close navigation" onClick={() => onOpenChange(false)} />}
    </>
  );
}

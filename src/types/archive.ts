import type { LucideIcon } from "lucide-react";

export type AssetFilter = "All" | "Photos" | "Documents";
export type LogRange = "Today" | "This Week";
export type NavigationView = "Home" | "Recently Added" | "Collections" | "Tags" | "Storage";

export interface NavigationItem {
  label: NavigationView;
  icon: LucideIcon;
}

export interface CollectionSummary {
  id: string;
  name: string;
  itemCount: number;
  updatedLabel: string;
}

export interface ArchiveAsset {
  title: string;
  size: string;
  tags: string[];
  type: Exclude<AssetFilter, "All">;
  imageClass: string;
}

export interface IngestionLogEntry {
  name: string;
  date: string;
  size: string;
  density: "Few" | "Some" | "Many";
  status: "Ready" | "Adding";
}

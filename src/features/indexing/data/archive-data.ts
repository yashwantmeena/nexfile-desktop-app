import { Clock3, HardDrive, House, Layers3, Tag } from "lucide-react";
import type { ArchiveAsset, CollectionSummary, IngestionLogEntry, NavigationItem } from "@/features/indexing/types";

export const navigationItems: NavigationItem[] = [
  { label: "Home", icon: House },
  { label: "Recently Added", icon: Clock3 },
  { label: "Collections", icon: Layers3 },
  { label: "Tags", icon: Tag },
  { label: "Storage", icon: HardDrive },
];

export const archiveAssets: ArchiveAsset[] = [
  { title: "Misty Pine Forest", size: "4.2 MB", tags: ["Landscape", "Blue", "Minimalist"], type: "Photos", imageClass: "forest" },
  { title: "Minimalist Concrete Desk", size: "3.8 MB", tags: ["Interior", "Work"], type: "Photos", imageClass: "interior" },
  { title: "Brutalist Apartment", size: "5.1 MB", tags: ["Architecture", "Urban"], type: "Photos", imageClass: "city" },
  { title: "Copper Wire Abstract", size: "2.9 MB", tags: ["Abstract", "Pattern"], type: "Photos", imageClass: "abstract" },
];

export const ingestionLogs: IngestionLogEntry[] = [
  { name: "IMG_9041_DSC.raw", date: "2024-11-29 14:22", size: "24,6 MB", density: "Many", status: "Ready" },
  { name: "contract_v2.pdf", date: "2024-11-29 13:05", size: "1,2 MB", density: "Some", status: "Ready" },
  { name: "voice_memo_11.wav", date: "2024-11-29 11:48", size: "8,4 MB", density: "Few", status: "Adding" },
  { name: "design_system.fig", date: "2024-11-28 17:30", size: "31,0 MB", density: "Many", status: "Ready" },
];

export const defaultCollections: CollectionSummary[] = [
  { id: "collection-ideas", name: "Creative Ideas", itemCount: 18, updatedLabel: "Updated today" },
  { id: "collection-work", name: "Work References", itemCount: 42, updatedLabel: "Updated yesterday" },
  { id: "collection-travel", name: "Travel", itemCount: 27, updatedLabel: "Updated 3 days ago" },
];

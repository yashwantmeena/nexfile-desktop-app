import { StoragePage } from "@/features/storage/StoragePage";
import type { AppNavigationItem } from "@/types/navigation";

interface StorageRouteProps {
  activeNavigation: AppNavigationItem;
  onNavigationChange: (item: AppNavigationItem) => void;
}

export function StorageRoute({ activeNavigation, onNavigationChange }: StorageRouteProps) {
  return <StoragePage activeNavigation={activeNavigation} onNavigationChange={onNavigationChange} />;
}

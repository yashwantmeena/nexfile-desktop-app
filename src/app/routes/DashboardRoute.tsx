import { DashboardPage } from "@/features/dashboard/DashboardPage";
import type { AppNavigationItem } from "@/types/navigation";

interface DashboardRouteProps {
  activeNavigation: AppNavigationItem;
  onNavigationChange: (item: AppNavigationItem) => void;
}

export function DashboardRoute({ activeNavigation, onNavigationChange }: DashboardRouteProps) {
  return <DashboardPage activeNavigation={activeNavigation} onNavigationChange={onNavigationChange} />;
}

import { useState } from "react";
import { DashboardRoute } from "@/app/routes/DashboardRoute";
import { StorageRoute } from "@/app/routes/StorageRoute";
import type { AppNavigationItem } from "@/types/navigation";

function App() {
  const [activeNavigation, setActiveNavigation] = useState<AppNavigationItem>("Search");

  if (activeNavigation === "Storage") {
    return <StorageRoute activeNavigation={activeNavigation} onNavigationChange={setActiveNavigation} />;
  }

  return <DashboardRoute activeNavigation={activeNavigation} onNavigationChange={setActiveNavigation} />;
}

export default App;

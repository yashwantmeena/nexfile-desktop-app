import { useEffect, useState } from "react";
import { DashboardPage } from "@/components/dashboard/dashboard-page";
import { AppSidebar } from "@/components/layout/app-sidebar";
import { TopBar } from "@/components/layout/top-bar";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { NavItem } from "@/types/localmind";

function App() {
  const [activeNav, setActiveNav] = useState<NavItem>("Home");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!notice) return;
    const timeout = window.setTimeout(() => setNotice(null), 2800);
    return () => window.clearTimeout(timeout);
  }, [notice]);

  return (
    <TooltipProvider delayDuration={250}>
      <div className="app-shell">
        <AppSidebar
          activeItem={activeNav}
          open={sidebarOpen}
          onActiveItemChange={setActiveNav}
          onOpenChange={setSidebarOpen}
          onNotice={setNotice}
        />
        <section className="workspace">
          <TopBar onMenuOpen={() => setSidebarOpen(true)} onNotice={setNotice} />
          <DashboardPage activeView={activeNav} onNotice={setNotice} />
        </section>
        {notice && <div className="toast" role="status">{notice}</div>}
      </div>
    </TooltipProvider>
  );
}

export default App;

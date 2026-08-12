import { useEffect, useState } from "react";
import { AppRouter } from "@/app/router";
import { Header } from "@/components/Header/Header";
import { Sidebar } from "@/components/Sidebar/Sidebar";
import { NOTICE_DURATION_MS } from "@/lib/constants";
import type { NavItem } from "@/types/navigation";

function App() {
  const [activeNav, setActiveNav] = useState<NavItem>("Home");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    if (!notice) return;
    const timeout = window.setTimeout(() => setNotice(null), NOTICE_DURATION_MS);
    return () => window.clearTimeout(timeout);
  }, [notice]);

  return (
    <div className="app-shell">
        <Sidebar
          activeItem={activeNav}
          open={sidebarOpen}
          onActiveItemChange={setActiveNav}
          onOpenChange={setSidebarOpen}
          onNotice={setNotice}
        />
        <section className="workspace">
          <Header onMenuOpen={() => setSidebarOpen(true)} onNotice={setNotice} />
          <AppRouter activeView={activeNav} onNotice={setNotice} />
        </section>
        {notice && <div className="toast" role="status">{notice}</div>}
    </div>
  );
}

export default App;

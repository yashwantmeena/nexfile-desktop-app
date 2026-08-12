import { useState } from "react";
import { HomePage } from "@/pages/Home/HomePage";
import { SearchPage } from "@/pages/Search/SearchPage";
import { SettingsPage } from "@/pages/Settings/SettingsPage";
import { StoragePage } from "@/pages/Storage/StoragePage";
import type { NavItem } from "@/types/navigation";

interface AppRouterProps {
  activeView: NavItem;
  onNotice: (message: string) => void;
}

export function AppRouter({ activeView, onNotice }: AppRouterProps) {
  const [query, setQuery] = useState("");
  const [activeTag, setActiveTag] = useState<string | null>(null);

  if (activeView === "Home") {
    return (
      <HomePage
        query={query}
        activeTag={activeTag}
        onQueryChange={setQuery}
        onTagChange={setActiveTag}
        onNotice={onNotice}
      />
    );
  }

  if (activeView === "Storage") {
    return <StoragePage onNotice={onNotice} />;
  }

  if (activeView === "Settings") {
    return <SettingsPage onNotice={onNotice} />;
  }

  return (
    <SearchPage
      activeView={activeView}
      query={query}
      activeTag={activeTag}
      onQueryChange={setQuery}
      onTagChange={setActiveTag}
      onNotice={onNotice}
    />
  );
}

import { useMemo, useState } from "react";
import { ChevronDown, Grid2X2, LayoutGrid, List } from "lucide-react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppToolbar } from "@/components/layout/AppToolbar";
import type { DateFilter } from "@/components/layout/AppToolbar";
import type { AppNavigationItem } from "@/types/navigation";
import { CategoryFilters } from "./components/CategoryFilters";
import { FileGrid } from "./components/FileGrid";
import { FileInspector } from "./components/FileInspector";
import { FilterBar } from "./components/FilterBar";
import { categories, dashboardFiles } from "./data/dashboard-data";
import "./dashboard.css";

interface DashboardPageProps { activeNavigation:AppNavigationItem; onNavigationChange:(item:AppNavigationItem)=>void; }

export function DashboardPage({ activeNavigation,onNavigationChange }:DashboardPageProps) {
  const [query,setQuery]=useState(""); const [activeCategory,setActiveCategory]=useState("All"); const [selectedId,setSelectedId]=useState(1); const [favorites,setFavorites]=useState([1,7]); const [tags,setTags]=useState(["work","project-nexfile","design"]); const [dateFilter,setDateFilter]=useState<DateFilter>("any"); const [gridMode,setGridMode]=useState(true);
  const files=useMemo(()=>dashboardFiles.filter(file=>file.name.toLowerCase().includes(query.toLowerCase())),[query]);
  const selectedFile=dashboardFiles.find(file=>file.id===selectedId)??dashboardFiles[0];
  const toggleFavorite=(id:number)=>setFavorites(items=>items.includes(id)?items.filter(item=>item!==id):[...items,id]);
  return (
    <div className="nexfile-app">
      <AppSidebar activeItem={activeNavigation} onActiveItemChange={onNavigationChange}/>
      <main className="nf-main search-main">
        <AppToolbar query={query} dateFilter={dateFilter} onQueryChange={setQuery} onDateFilterChange={setDateFilter}/>
        <FilterBar tags={tags} dateFilterLabel={dateFilter === "any" ? undefined : ({today:"Today","7days":"Last 7 days","30days":"Last 30 days",year:"This year"} as const)[dateFilter]} onTagsChange={setTags} onClearDateFilter={()=>setDateFilter("any")} onReset={()=>{setTags([]);setDateFilter("any");}}/>
        <section className="content-shell">
          <div className="results-pane">
            <header className="results-heading">
              <div>
                <h1>Search results</h1>
                <div className="results-meta">
                  <p className="result-count">12,700 files found across your indexed locations</p>
                  <span className="results-status"><i/>Index up to date</span>
                </div>
              </div>
              <div className="results-controls">
                <div className="view-switch result-view-switch">
                  <button className={gridMode ? "active" : ""} onClick={() => setGridMode(true)} aria-label="Grid view"><LayoutGrid /></button>
                  <button className={gridMode ? "" : "active"} onClick={() => setGridMode(false)} aria-label="List view"><List /></button>
                  <button aria-label="Compact grid"><Grid2X2 /></button>
                </div>
                <button className="results-sort">Newest <ChevronDown /></button>
              </div>
            </header>
            <CategoryFilters categories={categories} activeCategory={activeCategory} onCategoryChange={setActiveCategory}/>
            <FileGrid files={files} gridMode={gridMode} selectedId={selectedId} favorites={favorites} onSelect={setSelectedId} onFavorite={toggleFavorite}/>
          </div>
          <FileInspector file={selectedFile} favorite={favorites.includes(selectedFile.id)} onFavorite={()=>toggleFavorite(selectedFile.id)}/>
        </section>
      </main>
    </div>
  );
}

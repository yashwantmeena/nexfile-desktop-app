import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, Grid2X2, LayoutGrid, List } from "lucide-react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppToolbar } from "@/components/layout/AppToolbar";
import type { DateFilter } from "./types/filter";
import type { AppNavigationItem } from "@/types/navigation";
import { CategoryFilters } from "./components/CategoryFilters";
import { FileGrid } from "./components/FileGrid";
import { FileInspector } from "./components/FileInspector";
import { FilterBar } from "./components/FilterBar";
import { dashboardFiles } from "./data/dashboard-data";
import type { HomeCounts } from "./types/home";
import "./dashboard.css";

interface DashboardPageProps { activeNavigation:AppNavigationItem; onNavigationChange:(item:AppNavigationItem)=>void; }

export function DashboardPage({ activeNavigation,onNavigationChange }:DashboardPageProps) {
  const [homeCounts, setHomeCounts] = useState<HomeCounts | null>(null);
  const [countsError, setCountsError] = useState<string | null>(null);
  const [countsLoading, setCountsLoading] = useState(true);
  const [refreshKey, setRefreshKey] = useState(0);
  useEffect(() => {
    let cancelled = false;
    let pending = false;
    const load = async () => {
      if (pending) return;
      pending = true;
      setCountsLoading(true);
      setHomeCounts(null);
      setCountsError(null);
      try {
        const result = await invoke<HomeCounts>("get_file_count");
        if (!cancelled) setHomeCounts(result);
      } catch {
        if (!cancelled) setCountsError("Unable to verify file counts. Please try again.");
      } finally {
        pending = false;
        if (!cancelled) setCountsLoading(false);
      }
    };
    void load();
    window.addEventListener("focus", load);
    return () => { cancelled = true; window.removeEventListener("focus", load); };
  }, [refreshKey]);
  const counts = homeCounts?.counts;
  const categories = [
    { label: "All", count: homeCounts?.totalCount },
    ...(counts ?? []).map(({ fileType, count }) => ({ label: fileType, count })),
  ].map(({ label, count }) => ({ label, count: count == null ? "—" : count.toLocaleString() }));
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
                  <p className="result-count">{homeCounts?.totalCount != null
                    ? `${homeCounts.totalCount.toLocaleString()} imported files across your saved drives`
                    : countsLoading ? "Verifying file counts…" : "File counts unavailable"}</p>
                  {counts && <span className="results-status"><i/>Counts verified</span>}
                </div>
              </div>
              <div className="results-controls">
                <button className="results-sort" disabled={countsLoading} onClick={() => setRefreshKey(key => key + 1)}>Refresh counts</button>
                <div className="view-switch result-view-switch">
                  <button className={gridMode ? "active" : ""} onClick={() => setGridMode(true)} aria-label="Grid view"><LayoutGrid /></button>
                  <button className={gridMode ? "" : "active"} onClick={() => setGridMode(false)} aria-label="List view"><List /></button>
                  <button aria-label="Compact grid"><Grid2X2 /></button>
                </div>
                <button className="results-sort">Newest <ChevronDown /></button>
              </div>
            </header>
            {(countsError || !!homeCounts?.issues.length) && <div className="count-warning" role="alert">
              <strong>File counts could not be verified</strong>
              {countsError && <p>{countsError}</p>}
              {homeCounts?.issues.map(issue => <p key={issue.driveId}><strong>{issue.driveName || issue.driveId}:</strong> {issue.message}</p>)}
            </div>}
            <CategoryFilters categories={categories} activeCategory={activeCategory} onCategoryChange={setActiveCategory}/>
            <FileGrid files={files} gridMode={gridMode} selectedId={selectedId} favorites={favorites} onSelect={setSelectedId} onFavorite={toggleFavorite}/>
          </div>
          <FileInspector file={selectedFile} favorite={favorites.includes(selectedFile.id)} onFavorite={()=>toggleFavorite(selectedFile.id)}/>
        </section>
      </main>
    </div>
  );
}

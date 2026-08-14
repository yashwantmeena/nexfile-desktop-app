import { useMemo, useState } from "react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppToolbar } from "@/components/layout/AppToolbar";
import type { AppNavigationItem } from "@/types/navigation";
import { CategoryFilters } from "./components/CategoryFilters";
import { FileGrid } from "./components/FileGrid";
import { FileInspector } from "./components/FileInspector";
import { FilterBar } from "./components/FilterBar";
import { categories, dashboardFiles } from "./data/dashboard-data";
import "./dashboard.css";

interface DashboardPageProps { activeNavigation:AppNavigationItem; onNavigationChange:(item:AppNavigationItem)=>void; }

export function DashboardPage({ activeNavigation,onNavigationChange }:DashboardPageProps) {
  const [query,setQuery]=useState(""); const [activeCategory,setActiveCategory]=useState("All"); const [selectedId,setSelectedId]=useState(1); const [favorites,setFavorites]=useState([1,7]); const [tags,setTags]=useState(["work","project-nexfile","design"]); const [gridMode,setGridMode]=useState(true);
  const files=useMemo(()=>dashboardFiles.filter(file=>file.name.toLowerCase().includes(query.toLowerCase())),[query]);
  const selectedFile=dashboardFiles.find(file=>file.id===selectedId)??dashboardFiles[0];
  const toggleFavorite=(id:number)=>setFavorites(items=>items.includes(id)?items.filter(item=>item!==id):[...items,id]);
  return <div className="nexfile-app"><AppSidebar activeItem={activeNavigation} onActiveItemChange={onNavigationChange}/><main className="nf-main"><AppToolbar query={query} gridMode={gridMode} onQueryChange={setQuery} onGridModeChange={setGridMode}/><FilterBar tags={tags} onTagsChange={setTags}/><section className="content-shell"><div className="results-pane"><p className="result-count">12,700 files found</p><CategoryFilters categories={categories} activeCategory={activeCategory} onCategoryChange={setActiveCategory}/><FileGrid files={files} gridMode={gridMode} selectedId={selectedId} favorites={favorites} onSelect={setSelectedId} onFavorite={toggleFavorite}/></div><FileInspector file={selectedFile} favorite={favorites.includes(selectedFile.id)} onFavorite={()=>toggleFavorite(selectedFile.id)}/></section></main></div>;
}

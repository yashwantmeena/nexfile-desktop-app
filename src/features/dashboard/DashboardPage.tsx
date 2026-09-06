import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown } from "lucide-react";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppToolbar } from "@/components/layout/AppToolbar";
import type { DateFilter } from "./types/filter";
import type { AppNavigationItem } from "@/types/navigation";
import { CategoryFilters } from "./components/CategoryFilters";
import { FileGrid } from "./components/FileGrid";
import { FilePreviewModal } from "./components/FilePreviewModal";
import { FilterBar } from "./components/FilterBar";
import { useFiles } from "./hooks/useFiles";
import type { DashboardFile } from "./types/file";
import type { HomeCounts } from "./types/home";
import "./dashboard.css";

interface DashboardPageProps { activeNavigation:AppNavigationItem; onNavigationChange:(item:AppNavigationItem)=>void; }

export function DashboardPage({ activeNavigation,onNavigationChange }:DashboardPageProps) {
  const [homeCounts, setHomeCounts] = useState<HomeCounts | null>(null);
  const [countsError, setCountsError] = useState<string | null>(null);
  const [activeCategory,setActiveCategory]=useState("All");
  const fetched = useFiles(activeCategory === "All" ? undefined : activeCategory.toLowerCase());
  const resultsPaneRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);
  const { nextOffset, loading, error, loadMore } = fetched;
  useEffect(() => {
    const root = resultsPaneRef.current;
    const sentinel = loadMoreRef.current;
    if (!root || !sentinel || nextOffset === null || loading || error) return;
    const observer = new IntersectionObserver(entries => {
      if (entries.some(entry => entry.isIntersecting)) loadMore();
    }, { root, rootMargin: "0px 0px 400px 0px" });
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [nextOffset, loading, error, loadMore]);
  useEffect(() => {
    let cancelled = false;
    let pending = false;
    const load = async () => {
      if (pending) return;
      pending = true;
      setCountsError(null);
      try {
        const result = await invoke<HomeCounts>("get_file_count");
        if (!cancelled) setHomeCounts(result);
      } catch {
        if (!cancelled) setCountsError("Unable to verify file counts. Please try again.");
      } finally {
        pending = false;
        // Loading state is represented by the category placeholders.
      }
    };
    const refreshWhenVisible = () => {
      if (document.visibilityState === "visible") void load();
    };
    void load();
    window.addEventListener("focus", load);
    document.addEventListener("visibilitychange", refreshWhenVisible);
    return () => {
      cancelled = true;
      window.removeEventListener("focus", load);
      document.removeEventListener("visibilitychange", refreshWhenVisible);
    };
  }, []);
  const counts = homeCounts?.counts;
  const categories = [
    { label: "All", count: homeCounts?.totalCount },
    ...(counts ?? []).map(({ fileType, count }) => ({ label: fileType, count })),
  ].map(({ label, count }) => ({ label, count: count == null ? "—" : count.toLocaleString() }));
  const [query,setQuery]=useState(""); const [tags,setTags]=useState<string[]>([]); const [dateFilter,setDateFilter]=useState<DateFilter>("any");
  const [previewIndex,setPreviewIndex]=useState<number|null>(null);
  const loadedFiles = useMemo<DashboardFile[]>(() => fetched.files.map(file => ({
    id: file.id, name: file.name, path: file.path, fileType: file.fileType,
    modifiedAtMs: file.modifiedAtMs,
    kind: file.name.includes(".") ? file.name.split(".").pop()!.toUpperCase() : file.fileType.toUpperCase(),
    time: file.modifiedAtMs === null ? "Unknown" : new Date(file.modifiedAtMs).toLocaleString(),
    image: file.imageUrl, sizeBytes: file.sizeBytes, categories: file.categories, tags: file.tags,
  })), [fetched.files]);
  const files=useMemo(()=>loadedFiles.filter(file=>{
    if(!file.name.toLowerCase().includes(query.toLowerCase()))return false;
    const metadata=[...(file.tags??[]),...(file.categories??[]),file.collection??""].map(value=>value.toLowerCase());
    return tags.every(tag=>metadata.some(value=>value===tag.toLowerCase()));
  }),[loadedFiles,query,tags]);
  return (
    <div className="nexfile-app">
      <AppSidebar activeItem={activeNavigation} onActiveItemChange={onNavigationChange}/>
      <main className="nf-main search-main">
        <AppToolbar query={query} dateFilter={dateFilter} onQueryChange={setQuery} onDateFilterChange={setDateFilter}/>
        <FilterBar tags={tags} dateFilterLabel={dateFilter === "any" ? undefined : ({today:"Today","7days":"Last 7 days","30days":"Last 30 days",year:"This year"} as const)[dateFilter]} onTagsChange={setTags} onClearDateFilter={()=>setDateFilter("any")} onReset={()=>{setTags([]);setDateFilter("any");}}/>
        <section className="content-shell">
          <div className="results-pane" ref={resultsPaneRef}>
            {(countsError || !!homeCounts?.issues.length) && <div className="count-warning" role="alert">
              <strong>File counts could not be verified</strong>
              {countsError && <p>{countsError}</p>}
              {homeCounts?.issues.map(issue => <p key={issue.driveId}><strong>{issue.driveName || issue.driveId}:</strong> {issue.message}</p>)}
            </div>}
            <div className="category-toolbar">
              <CategoryFilters categories={categories} activeCategory={activeCategory} onCategoryChange={setActiveCategory}/>
              <button className="results-sort category-sort" title="Filesystem modified time, descending">Modified: newest <ChevronDown /></button>
            </div>
            {(fetched.error || fetched.issues.length > 0) && <div className="count-warning" role="alert">
              {fetched.error && <p>{fetched.error}</p>}
              {fetched.issues.map(issue => <p key={issue.driveId}><strong>{issue.driveName || issue.driveId}:</strong> {issue.message}</p>)}
            </div>}
            {fetched.loading && !loadedFiles.length ? <div className="empty-state" role="status">Loading files…</div>
              : fetched.error && !loadedFiles.length ? null
              : <FileGrid files={files} onOpen={setPreviewIndex}/>}
            <div ref={loadMoreRef} className="files-load-more">
              {fetched.loading && loadedFiles.length > 0 && <span role="status">Loading more files…</span>}
              {fetched.error && fetched.nextOffset !== null && <button className="results-sort" onClick={fetched.loadMore}>Retry loading more files</button>}
            </div>
          </div>
        </section>
      </main>
      {previewIndex!==null&&files[previewIndex]&&<FilePreviewModal files={files} index={previewIndex} onIndexChange={setPreviewIndex} onClose={()=>setPreviewIndex(null)} onApplyFilter={value=>{setTags(current=>current.includes(value)?current:[...current,value]);setPreviewIndex(null);}}/>}
    </div>
  );
}

import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowUp, ChevronDown } from "lucide-react";
import { useCollections } from "@/components/layout/CollectionsProvider";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { AppToolbar, type SearchMode } from "@/components/layout/AppToolbar";
import type { AppNavigationItem } from "@/types/navigation";
import { CategoryFilters } from "./components/CategoryFilters";
import { ModelCategoryFilter } from "./components/ModelCategoryFilter";
import { FileGrid } from "./components/FileGrid";
import { FilePreviewModal } from "./components/FilePreviewModal";
import { FilterBar } from "./components/FilterBar";
import { useFiles } from "./hooks/useFiles";
import { emptyTrash } from "./api/files";
import type { DashboardFile } from "./types/file";
import type { HomeCounts } from "./types/home";
import "./dashboard.css";

interface DashboardPageProps { activeNavigation:AppNavigationItem; onNavigationChange:(item:AppNavigationItem)=>void; }

export function DashboardPage({ activeNavigation,onNavigationChange }:DashboardPageProps) {
  const { selected, collections } = useCollections();
  const activeCollection = activeNavigation === "Collections" ? selected : undefined;
  const [modelCategory, setModelCategory] = useState<string | undefined>();
  const [homeCounts, setHomeCounts] = useState<HomeCounts | null>(null);
  const [countsError, setCountsError] = useState<string | null>(null);
  const [activeCategory,setActiveCategory]=useState("All");
  const [query,setQuery]=useState(""); const [tags,setTags]=useState<string[]>([]);
  const [draftQuery, setDraftQuery] = useState("");
  const [searchMode, setSearchMode] = useState<SearchMode>("tags");
  const tagsKey = JSON.stringify(tags);
  const [refreshKey, setRefreshKey] = useState(0);
  const favoriteOnly = activeNavigation === "Favorites";
  const trashOnly = activeNavigation === "Trash";
  const [emptyingTrash, setEmptyingTrash] = useState(false);
  const fetched = useFiles(activeCategory === "All" ? undefined : activeCategory.toLowerCase(), query, searchMode, tags, refreshKey, activeCollection?.name, favoriteOnly, trashOnly);
  const resultsPaneRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);
  const [showBackToTop,setShowBackToTop]=useState(false);
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
    setHomeCounts(null);
    setCountsError(null);
    const load = async () => {
      try {
        const result = await invoke<HomeCounts>("get_file_count", { query, searchMode, tags: JSON.parse(tagsKey), collection: activeCollection?.name, favoriteOnly, trashOnly });
        if (!cancelled) setHomeCounts(result);
      } catch {
        if (!cancelled) setCountsError("Unable to count indexed files. Please try again.");
      }
    };
    const timer = window.setTimeout(() => void load(), query.trim() ? 150 : 0);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query, searchMode, tagsKey, refreshKey, activeCollection?.name, favoriteOnly, trashOnly]);
  const counts = homeCounts?.counts;
  const categories = [
    { label: "All", count: homeCounts?.totalCount },
    ...["image", "video", "audio", "document", "archive", "other"].map(fileType => ({ label: fileType, count: counts?.find(entry => entry.fileType === fileType)?.count })),
  ].map(({ label, count }) => ({ label, count: count == null ? "—" : count.toLocaleString() }));
  const [previewIndex,setPreviewIndex]=useState<number|null>(null);
  const loadedFiles = useMemo<DashboardFile[]>(() => fetched.files.map(file => ({
    id: file.id, driveId: file.driveId, name: file.name, path: file.path, fileType: file.fileType,
    modifiedAtMs: file.modifiedAtMs,
    kind: file.name.includes(".") ? file.name.split(".").pop()!.toUpperCase() : file.fileType.toUpperCase(),
    time: file.modifiedAtMs === null ? "Unknown" : new Date(file.modifiedAtMs).toLocaleString(),
    image: file.imageUrl, sizeBytes: file.sizeBytes, categories: file.categories, tags: file.tags,
    collections: file.collectionNames, collection: file.collectionNames[0], favorite: file.favorite, isTrashed: file.isTrashed,
  })), [fetched.files]);
  const modelCategories = [...new Set(loadedFiles.flatMap(file => file.categories ?? []))].sort();
  const files = loadedFiles.filter(file => !modelCategory || file.categories?.includes(modelCategory));
  return (
    <div className="nexfile-app">
      <AppSidebar activeItem={activeNavigation} onActiveItemChange={onNavigationChange}/>
      <main className="nf-main search-main">
        <AppToolbar query={draftQuery}
          onQueryChange={value => { setDraftQuery(value); if (searchMode === "name") setQuery(value); }}
          onQuerySubmit={value => { setDraftQuery(value); setQuery(value.trim()); }}
          searchMode={searchMode} onSearchModeChange={mode => { setSearchMode(mode); setDraftQuery(query); }}/>
        <FilterBar tags={tags} onTagsChange={setTags} onReset={()=>setTags([])}/>
        <section className="content-shell">
          <div className="results-pane" ref={resultsPaneRef} tabIndex={-1} onScroll={event=>setShowBackToTop(event.currentTarget.scrollTop>200)}>
            {(countsError || !!homeCounts?.issues.length) && <div className="count-warning" role="alert">
              <strong>Indexed file counts are unavailable</strong>
              {countsError && <p>{countsError}</p>}
              {homeCounts?.issues.map(issue => <p key={issue.driveId}><strong>{issue.driveName || issue.driveId}:</strong> {issue.message}</p>)}
            </div>}
            <div className="category-toolbar">
              <CategoryFilters categories={categories} activeCategory={activeCategory} onCategoryChange={setActiveCategory}/>
              <ModelCategoryFilter categories={modelCategories} selected={modelCategory} onSelect={value => { setModelCategory(value); setPreviewIndex(null); }}/>
              {trashOnly && <button className="empty-trash-button" disabled={emptyingTrash || !fetched.files.length} onClick={()=>void (async()=>{setEmptyingTrash(true);try{await emptyTrash();setPreviewIndex(null);fetched.clearFiles();}finally{setEmptyingTrash(false);}})()}>{emptyingTrash?"Emptying…":"Empty trash"}</button>}
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
            {showBackToTop && files.length > 0 && <div className="files-back-to-top">
              <button onClick={()=>{
                const pane=resultsPaneRef.current;
                pane?.focus({preventScroll:true});
                pane?.scrollTo({top:0,behavior:"instant"});
                setShowBackToTop(false);
                setPreviewIndex(null);
                setRefreshKey(current => current + 1);
              }}><ArrowUp aria-hidden="true"/>Back to top</button>
            </div>}
        </section>
      </main>
      {previewIndex!==null&&files[previewIndex]&&<FilePreviewModal files={files} index={previewIndex} collections={collections} categories={modelCategories} trashOnly={trashOnly} onMetadataSaved={()=>setRefreshKey(current=>current+1)} onDeleted={()=>{const nextIndex=previewIndex<files.length-1?previewIndex:previewIndex>0?previewIndex-1:null;fetched.removeFile(String(files[previewIndex].id));setPreviewIndex(nextIndex);}} onFavoriteSaved={(id,favorite)=>{fetched.updateFavorite(String(id),favorite);if(favoriteOnly&&!favorite){setPreviewIndex(null);setRefreshKey(current=>current+1);}}} onIndexChange={setPreviewIndex} hasMore={nextOffset!==null} loadingMore={loading} loadError={error} onLoadMore={loadMore} onClose={()=>setPreviewIndex(null)} onApplyFilter={value=>{setTags(current=>current.includes(value)?current:[...current,value]);setPreviewIndex(null);}}/>}
    </div>
  );
}




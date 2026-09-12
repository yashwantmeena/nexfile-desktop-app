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
import { BulkSelectionToolbar } from "./components/BulkSelectionToolbar";
import { FilePreviewModal } from "./components/FilePreviewModal";
import { FilterBar } from "./components/FilterBar";
import { useFiles } from "./hooks/useFiles";
import { emptyTrash, enqueueBulkOperation } from "./api/files";
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
  const [countRefreshKey, setCountRefreshKey] = useState(0);
  const favoriteOnly = activeNavigation === "Favorites";
  const trashOnly = activeNavigation === "Trash";
  const [emptyingTrash, setEmptyingTrash] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const [selectingAll, setSelectingAll] = useState(false);
  const [bulkDeleteBusy, setBulkDeleteBusy] = useState(false);
  const [bulkDeleteError, setBulkDeleteError] = useState("");
  const fetched = useFiles(activeCategory === "All" ? undefined : activeCategory.toLowerCase(), query, searchMode, tags, refreshKey, activeCollection?.name, favoriteOnly, trashOnly, modelCategory);
  const resultsPaneRef = useRef<HTMLDivElement>(null);
  const loadMoreRef = useRef<HTMLDivElement>(null);
  const [showBackToTop,setShowBackToTop]=useState(false);
  const { totalCount, nextOffset, loading, error, loadMore } = fetched;
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
        const result = await invoke<HomeCounts>("get_file_count", { query, searchMode, tags: JSON.parse(tagsKey), collection: activeCollection?.name, favoriteOnly, trashOnly, modelCategory });
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
  }, [query, searchMode, tagsKey, refreshKey, countRefreshKey, activeCollection?.name, favoriteOnly, trashOnly, modelCategory]);
  const counts = homeCounts?.counts;
  const categories = [
    { label: "All", count: homeCounts?.totalCount },
    ...["image", "video", "audio", "document", "archive", "other"].map(fileType => ({ label: fileType, count: counts?.find(entry => entry.fileType === fileType)?.count })),
  ].map(({ label, count }) => ({ label, count: count == null ? "—" : count.toLocaleString() }));
  const [previewIndex,setPreviewIndex]=useState<number|null>(null);
  const loadedFiles = useMemo<DashboardFile[]>(() => fetched.files.map(file => ({
    id: file.id, driveId: file.driveId, name: file.name, path: file.path, fileType: file.fileType,
    modifiedAtMs: file.modifiedAtMs,
    capturedAtMs: file.capturedAtMs,
    kind: file.name.includes(".") ? file.name.split(".").pop()!.toUpperCase() : file.fileType.toUpperCase(),
    time: file.modifiedAtMs === null ? "Unknown" : new Date(file.modifiedAtMs).toLocaleString(),
    image: file.imageUrl, sizeBytes: file.sizeBytes, categories: file.categories, tags: file.tags,
    collections: file.collectionNames, collection: file.collectionNames[0], favorite: file.favorite, isTrashed: file.isTrashed,
  })), [fetched.files]);
  const modelCategories = [...new Set(loadedFiles.flatMap(file => file.categories ?? []))].sort();
  const files = loadedFiles;
  const selectedFiles = useMemo(() => files.filter(file => selectedIds.has(String(file.id))), [files, selectedIds]);
  const allFilteredSelected = totalCount > 0 && selectedIds.size >= totalCount;
  const selectedCount = selectedIds.size;
  useEffect(() => {
    setSelectedIds(new Set());
    setBulkDeleteError("");
  }, [activeNavigation, activeCategory, query, tagsKey, activeCollection?.id, modelCategory]);
  const toggleSelection = (id:string) => {
    console.info("[NexFile] Toggling file selection", { id });
    setBulkDeleteError("");
    setSelectedIds(current => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };
  const clearSelection = () => {
    if (bulkDeleteBusy) return;
    console.info("[NexFile] Clearing file selection", { selectedCount:selectedIds.size });
    setSelectedIds(new Set());
    setBulkDeleteError("");
  };
  const resolveSelectedFiles = async () => {
    if (selectedIds.size <= selectedFiles.length) return selectedFiles;
    console.info("[NexFile] Loading remaining filtered pages before bulk action", { selectedCount:selectedIds.size, loadedSelectedCount:selectedFiles.length });
    const allFiles = await fetched.loadAll();
    const resolvedFiles = allFiles.filter(file => selectedIds.has(String(file.id)));
    console.info("[NexFile] Resolved bulk action files", { resolvedCount:resolvedFiles.length });
    return resolvedFiles;
  };
  const toggleSelectAll = async () => {
    if (selectingAll || bulkDeleteBusy) return;
    setBulkDeleteError("");
    setSelectingAll(true);
    console.info("[NexFile] Selecting all files matching active filters", { loadedCount: files.length, nextOffset, tags, query, activeCategory, modelCategory });
    try {
      const allFiles = await fetched.loadAll();
      setSelectedIds(new Set(allFiles.map(file => String(file.id))));
      console.info("[NexFile] Select all complete", { selectedCount: allFiles.length });
    } finally {
      setSelectingAll(false);
    }
  };
  const applyBulkMetadata = async (changes:{isTrashed?:boolean; favorite?:boolean}, successMessage:string, filesToUpdate?:Array<{id:string|number; driveId?:string; path:string; favorite?:boolean}>) => {
    if (!selectedIds.size || bulkDeleteBusy) return;
    setBulkDeleteBusy(true);
    setBulkDeleteError("");
    try {
      const actionFiles = filesToUpdate ?? (allFilteredSelected ? files : await resolveSelectedFiles());
      const selectedForQueue = allFilteredSelected ? undefined : actionFiles.map(file => String(file.id));
      const operation = changes.isTrashed
        ? "moveToTrash" as const
        : { setFavorite: { favorite: changes.favorite === true } };
      const filters = {
        query,
        searchMode,
        tags,
        collection: activeCollection?.name,
        mediaType: activeCategory === "All" ? undefined : activeCategory.toLowerCase(),
        favoriteOnly,
        trashOnly,
        modelCategory,
      };
      const queuedCount = allFilteredSelected ? totalCount : actionFiles.length;
      console.info("[NexFile] Creating bulk operation task", { operation, queuedCount, allFilteredSelected, selectedForQueue: selectedForQueue?.length ?? 0, filters });
      await enqueueBulkOperation(operation, filters, queuedCount, selectedForQueue);
      console.info("[NexFile] Bulk operation task queued", { operation, queuedCount });
      if (changes.isTrashed) actionFiles.forEach(file => fetched.removeFile(String(file.id)));
      if (changes.favorite !== undefined) actionFiles.forEach(file => fetched.updateFavorite(String(file.id), changes.favorite!));
      setSelectedIds(new Set());
      setCountRefreshKey(current => current + 1);
      setRefreshKey(current => current + 1);
      void successMessage;
    } catch (error) {
      console.error("[NexFile] Bulk operation task failed", { changes, error });
      setBulkDeleteError("Unable to queue this bulk operation. Please try again.");
    } finally {
      setBulkDeleteBusy(false);
    }
  };
  const deleteSelected = async () => {
    if (!selectedCount || bulkDeleteBusy) return;
    if (!window.confirm(`Move ${selectedCount} selected ${selectedCount === 1 ? "file" : "files"} to Trash?`)) return;
    await applyBulkMetadata({ isTrashed:true }, "moved to Trash");
  };
  const favoriteTarget = selectedFiles.some(file => !file.favorite);
  const favoriteSelected = async () => {
    const actionFiles = allFilteredSelected ? files : await resolveSelectedFiles();
    const target = actionFiles.some(file => !file.favorite);
    await applyBulkMetadata({ favorite:target }, target ? "added to Favorites" : "removed from Favorites", actionFiles);
  };
  return (
    <div className="nexfile-app">
      <AppSidebar activeItem={activeNavigation} onActiveItemChange={onNavigationChange}/>
      <main className="nf-main search-main">
        <AppToolbar query={draftQuery}
          onQueryChange={value => { setDraftQuery(value); if (searchMode === "name") setQuery(value); }}
          onQuerySubmit={value => {
            const submitted = value.trim();
            if (searchMode === "tags") {
              if (submitted) setTags(current => current.some(tag => tag.toLowerCase() === submitted.toLowerCase()) ? current : [...current, submitted]);
              setDraftQuery("");
              setQuery("");
              return;
            }
            setDraftQuery(value);
            setQuery(submitted);
          }}
          searchMode={searchMode} onSearchModeChange={mode => { setSearchMode(mode); setDraftQuery(query); }}/>
        <FilterBar tags={tags} onTagsChange={setTags} onReset={()=>setTags([])}
          selectAllLabel={allFilteredSelected ? "All selected" : selectingAll ? "Selecting…" : "Select all"}
          selectAllDisabled={trashOnly || !files.length || selectingAll || bulkDeleteBusy || loading || allFilteredSelected}
          selectAllPressed={allFilteredSelected}
          onSelectAll={!trashOnly && files.length ? ()=>void toggleSelectAll() : undefined}
          selectionActions={selectedCount > 0 && !trashOnly ? <BulkSelectionToolbar count={selectedCount} busy={bulkDeleteBusy || loading || selectingAll} error={bulkDeleteError} onDelete={()=>void deleteSelected()} onFavorite={()=>void favoriteSelected()} favoriteLabel={favoriteTarget ? "Add to Favorites" : "Remove from Favorites"} onClear={clearSelection}/> : undefined}/>
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
              {trashOnly && <button className="empty-trash-button" disabled={emptyingTrash || !fetched.files.length} onClick={()=>void (async()=>{setEmptyingTrash(true);try{await emptyTrash();setPreviewIndex(null);fetched.clearFiles();setCountRefreshKey(current=>current+1);}finally{setEmptyingTrash(false);}})()}>{emptyingTrash?"Emptying…":"Empty Trash"}</button>}
              <button className="results-sort category-sort" title="Filesystem modified time, descending">Modified: newest <ChevronDown /></button>
            </div>
            {(fetched.error || fetched.issues.length > 0) && <div className="count-warning" role="alert">
              {fetched.error && <p>{fetched.error}</p>}
              {fetched.issues.map(issue => <p key={issue.driveId}><strong>{issue.driveName || issue.driveId}:</strong> {issue.message}</p>)}
            </div>}
            {fetched.loading && !loadedFiles.length ? <div className="empty-state" role="status">Loading files…</div>
              : fetched.error && !loadedFiles.length ? null
              : <FileGrid files={files} selectable={!trashOnly} selectedIds={selectedIds} onToggleSelection={toggleSelection} onOpen={setPreviewIndex}/>}
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
      {previewIndex!==null&&files[previewIndex]&&<FilePreviewModal files={files} index={previewIndex} collections={collections} categories={modelCategories} trashOnly={trashOnly} onMetadataSaved={()=>setRefreshKey(current=>current+1)} onDeleted={()=>{const nextIndex=previewIndex<files.length-1?previewIndex:previewIndex>0?previewIndex-1:null;fetched.removeFile(String(files[previewIndex].id));setPreviewIndex(nextIndex);setCountRefreshKey(current=>current+1);}} onFavoriteSaved={(id,favorite)=>{fetched.updateFavorite(String(id),favorite);setCountRefreshKey(current=>current+1);if(favoriteOnly&&!favorite){setPreviewIndex(null);setRefreshKey(current=>current+1);}}} onIndexChange={setPreviewIndex} hasMore={nextOffset!==null} loadingMore={loading} loadError={error} onLoadMore={loadMore} onClose={()=>setPreviewIndex(null)} onApplyFilter={value=>{setTags(current=>current.includes(value)?current:[...current,value]);setPreviewIndex(null);}}/>}
    </div>
  );
}




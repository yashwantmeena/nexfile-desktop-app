import { useCallback, useEffect, useRef, useState } from "react";
import { fetchFiles, type FilePage, type FetchedFile } from "../api/files";


export function useFiles(mediaType?: string, query = "", searchMode = "tags", tags: string[] = [], refreshKey = 0, collection?: string, favoriteOnly = false, trashOnly = false, modelCategory?: string) {
  const tagsKey = JSON.stringify(tags);
  const [files, setFiles] = useState<FetchedFile[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [nextOffset, setNextOffset] = useState<number | null>(null);
  const [issues, setIssues] = useState<FilePage["issues"]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const pending = useRef(false);
  const load = useCallback(async (offset: number, reset = false, clear = reset): Promise<FilePage | null> => {
    if (pending.current) return null;
    if (reset) {
      generation.current += 1;
      if (clear) setFiles([]);
      setNextOffset(null);
      setIssues([]);
    }
    const request = generation.current;
    pending.current = true;
    setLoading(true);
    setError(null);
    console.debug("[NexFile] Fetching file page", { offset, limit:60, mediaType, query, searchMode, tags:JSON.parse(tagsKey), collection, favoriteOnly, trashOnly, modelCategory });
    try {
      const page = await fetchFiles(offset, 60, mediaType, query, searchMode, JSON.parse(tagsKey), collection, favoriteOnly, trashOnly, modelCategory);
      if (request !== generation.current) return null;
      setFiles(previous => reset ? page.files : [...new Map([...previous, ...page.files].map(file => [file.id, file])).values()]);
      setTotalCount(page.totalCount);
      setNextOffset(page.nextOffset);
      setIssues(page.issues);
      console.debug("[NexFile] File page loaded", { offset, fileCount:page.files.length, totalCount:page.totalCount, nextOffset:page.nextOffset, issueCount:page.issues.length });
      return page;
    } catch (error: unknown) {
      const message = typeof error === "object" && error !== null && "code" in error
        && error.code === "VALIDATION_ERROR" && "message" in error && typeof error.message === "string"
        ? error.message : "Unable to fetch files. Please try again.";
      console.error("[NexFile] File page failed", { offset, error });
      if (request === generation.current) setError(message);
    } finally {
      if (request === generation.current) {
        pending.current = false;
        setLoading(false);
      }
    }
    return null;
  }, [mediaType, query, searchMode, tagsKey, collection, favoriteOnly, trashOnly, modelCategory]);
  useEffect(() => {
    // Returning to the app must not replace loaded pages with page one:
    // shrinking the virtual grid would clamp the user's scroll position.
    // Invalidate immediately, including during the debounce window.
    generation.current += 1;
    pending.current = true;
    setFiles([]);
    setNextOffset(null);
    setIssues([]);
    setError(null);
    setLoading(true);
    const timer = window.setTimeout(() => {
      pending.current = false;
      void load(0, true, true);
    }, query.trim() ? 150 : 0);
    return () => {
      window.clearTimeout(timer);
      generation.current += 1;
      pending.current = false;
    };
  }, [load, query, refreshKey]);
  const loadMore = useCallback(() => {
    if (nextOffset !== null) void load(nextOffset);
  }, [load, nextOffset]);
  const loadAll = useCallback(async (): Promise<FetchedFile[]> => {
    let offset = nextOffset;
    let all = [...files];
    console.info("[NexFile] Loading all filtered pages for selection", { loadedCount:all.length, nextOffset:offset });
    while (offset !== null) {
      const page = await load(offset);
      if (!page) break;
      all = [...new Map([...all, ...page.files].map(file => [file.id, file])).values()];
      offset = page.nextOffset;
    }
    console.info("[NexFile] Finished loading all filtered pages", { selectedCandidateCount:all.length, nextOffset:offset });
    return all;
  }, [files, load, nextOffset]);
  const updateFavorite = useCallback((id: string, favorite: boolean) => {
    setFiles(current => current.map(file => file.id === id ? { ...file, favorite } : file));
  }, []);
  const addToCollection = useCallback((id: string, collectionName: string) => {
    setFiles(current => current.map(file => file.id === id && !file.collectionNames.some(name => name.toLowerCase() === collectionName.toLowerCase())
      ? { ...file, collectionNames:[...file.collectionNames, collectionName] }
      : file));
  }, []);
  const addTag = useCallback((id: string, tag: string) => {
    setFiles(current => current.map(file => file.id === id && !file.tags.some(value => value.toLowerCase() === tag.toLowerCase())
      ? { ...file, tags:[...file.tags, tag] }
      : file));
  }, []);
  const removeFile = useCallback((id: string) => {
    setFiles(current => current.filter(file => file.id !== id));
  }, []);
  const clearFiles = useCallback(() => {
    generation.current += 1;
    pending.current = false;
    setFiles([]);
    setTotalCount(0);
    setNextOffset(null);
    setIssues([]);
    setError(null);
  }, []);
  return { files, totalCount, nextOffset, issues, loading, error, loadMore, loadAll, updateFavorite, addToCollection, addTag, removeFile, clearFiles };
}

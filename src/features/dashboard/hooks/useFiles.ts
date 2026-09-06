import { useCallback, useEffect, useRef, useState } from "react";
import { fetchFiles, type FilePage, type FetchedFile } from "../api/files";


export function useFiles(mediaType?: string) {
  const [files, setFiles] = useState<FetchedFile[]>([]);
  const [nextOffset, setNextOffset] = useState<number | null>(null);
  const [issues, setIssues] = useState<FilePage["issues"]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const generation = useRef(0);
  const pending = useRef(false);
  const load = useCallback(async (offset: number, reset = false, clear = reset) => {
    if (pending.current) return;
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
    try {
      const page = await fetchFiles(offset, 60, mediaType);
      if (request !== generation.current) return;
      setFiles(previous => reset ? page.files : [...new Map([...previous, ...page.files].map(file => [file.id, file])).values()]);
      setNextOffset(page.nextOffset);
      setIssues(page.issues);
    } catch {
      if (request === generation.current) setError("Unable to fetch files. Please try again.");
    } finally {
      if (request === generation.current) {
        pending.current = false;
        setLoading(false);
      }
    }
  }, [mediaType]);
  useEffect(() => {
    // Returning to the app must not replace loaded pages with page one:
    // shrinking the virtual grid would clamp the user's scroll position.
    void load(0, true, true);
    return () => {
      generation.current += 1;
      pending.current = false;
    };
  }, [load]);
  const loadMore = useCallback(() => {
    if (nextOffset !== null) void load(nextOffset);
  }, [load, nextOffset]);
  return { files, nextOffset, issues, loading, error, loadMore };
}

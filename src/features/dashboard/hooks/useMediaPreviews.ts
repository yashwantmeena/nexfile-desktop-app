import { useEffect, useState, useSyncExternalStore } from "react";
import type { DashboardFile } from "../types/file";
import { PreviewCache } from "../media/previewCache";
import { renderPreview, type PreviewSource } from "../media/renderPreview";

export const previewKey = (file: DashboardFile) => JSON.stringify([file.image, file.modifiedAtMs, file.sizeBytes, file.fileType]);

export function useMediaPreviews(files: DashboardFile[], first: number, last: number) {
  const [cache] = useState(() => new PreviewCache(renderPreview));
  const urls = useSyncExternalStore(cache.subscribe, cache.snapshot);
  useEffect(() => {
    const source = (file: DashboardFile): PreviewSource[] => file.image && (file.fileType === "image" || file.fileType === "video")
      ? [{ key: previewKey(file), url: file.image, video: file.fileType === "video" }] : [];
    cache.setWindow(files.slice(first, last).flatMap(source));
  }, [cache, files, first, last]);
  useEffect(() => () => cache.clear(), [cache]);
  return urls;
}

import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export interface FetchedFile {
  id: string;
  driveId: string;
  name: string;
  path: string;
  fileType: string;
  sizeBytes: number;
  modifiedAtMs: number | null;
  categories: string[];
  tags: string[];
  collectionNames: string[];
  imageUrl?: string;
}

export interface FilePage {
  files: FetchedFile[];
  totalCount: number;
  nextOffset: number | null;
  issues: { driveId: string; driveName: string; message: string }[];
}

/** Indexed files, ordered by filesystem modification time descending.
 * Refresh from offset zero after imports/deletions, which can shift page offsets.
 * Search text and tags are filtered before pagination.
 */
export async function fetchFiles(offset = 0, limit = 60, mediaType?: string, query = "", searchMode = "tags", tags: string[] = [], collection?: string): Promise<FilePage> {
  const page = await invoke<FilePage>("fetch_files", { offset, limit, mediaType, query, searchMode, tags, collection });
  return {
    ...page,
    files: page.files.map(file => ({
      ...file,
      imageUrl: file.fileType === "image" || file.fileType === "video" ? convertFileSrc(file.path) : undefined,
    })),
  };
}

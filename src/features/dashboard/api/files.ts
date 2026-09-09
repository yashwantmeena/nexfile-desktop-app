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
  favorite: boolean;
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
export async function fetchFiles(offset = 0, limit = 60, mediaType?: string, query = "", searchMode = "tags", tags: string[] = [], collection?: string, favoriteOnly = false): Promise<FilePage> {
  const page = await invoke<FilePage>("fetch_files", { offset, limit, mediaType, query, searchMode, tags, collection, favoriteOnly });
  return {
    ...page,
    files: page.files.map(file => ({
      ...file,
      imageUrl: file.fileType === "image" || file.fileType === "video" ? convertFileSrc(file.path) : undefined,
    })),
  };
}

interface FileMetadataChanges {
  name?: string;
  category?: string;
  tags?: string[];
  collectionIds?: string[];
  favorite?: boolean;
}

export async function updateFileMetadata(file: { driveId: string; path: string }, changes: FileMetadataChanges): Promise<void> {
  await invoke("update_file_metadata", { driveId: file.driveId, path: file.path, ...changes });
}

export type FileKind = "pdf" | "image" | "sheet" | "doc" | "video";

export interface RecentFile {
  id: number;
  name: string;
  location: string;
  tags: string[];
  accessed: string;
  kind: FileKind;
}

export interface FolderSummary {
  path: string;
  detail: string;
  progress: number;
}

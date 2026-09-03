export interface FileTypeCount {
  fileType: string;
  count: number;
}

export interface HomeCounts {
  counts: FileTypeCount[] | null;
  totalCount: number | null;
  issues: { driveId: string; driveName: string; message: string }[];
}

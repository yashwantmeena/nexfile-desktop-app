export interface StorageDrive {
  driveId: string;
  deviceId: string | null;
  driveName: string;
  partitionName: string;
  fileSystem: string;
  totalBytes: number;
  systemUsedBytes?: number;
  systemUsedPercent?: number;
  appUsedBytes?: number;
  appUsedPercent?: number;
  availableBytes?: number;
  appLimitBytes?: number;
  fileCount: number;
  priority: number;
  isMounted: boolean;
  isConnected: boolean;
  isSystem: boolean;
}

export interface StorageData {
  totalBytes: number;
  availableBytes: number;
  drivesDetected: number;
  fileIndexed: number;
  appLimitBytes: number;
  appUsedBytes: number;
  drives: StorageDrive[];
}

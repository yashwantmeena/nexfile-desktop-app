export type DriveStatus = "mounted" | "unmounted";

export interface StorageDrive {
  driveId: string;
  driveName: string;
  partitionName: string;
  fileSystem: string;
  totalCapacity: number;
  systemUsed?: number;
  systemUsedPercent?: number;
  appUsed?: number;
  appUsedPercent?: number;
  available?: number;
  appLimitValue?: number;
  priority: number;
  status: DriveStatus;
}

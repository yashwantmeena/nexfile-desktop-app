export type DriveStatus = "mounted" | "unmounted";

export interface StorageDrive {
  id: string;
  driveName: string;
  partitionName: string;
  fileSystem: string;
  total: string;
  used?: string;
  usedPercent?: number;
  appUsed?: string;
  appUsedPercent?: number;
  available?: string;
  appLimitValue?: number;
  appLimitUnit: "GB" | "TB";
  priority: number;
  status: DriveStatus;
}

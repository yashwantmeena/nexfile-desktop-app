import type { StorageDrive } from "../types/storage";

export const mountedDrives: StorageDrive[] = [
  { driveId:"c", driveName:"WD Blue", partitionName:"System (C:)", fileSystem:"NTFS", totalCapacity:256, systemUsed:21, systemUsedPercent:8, appUsed:84, appUsedPercent:84, available:235, appLimitValue:100, priority:1, status:"mounted" },
  { driveId:"d", driveName:"WD Blue", partitionName:"Work (D:)", fileSystem:"NTFS", totalCapacity:2048, systemUsed:812, systemUsedPercent:41, appUsed:206, appUsedPercent:20, available:1228.8, appLimitValue:1024, priority:2, status:"mounted" },
  { driveId:"e", driveName:"Samsung T7", partitionName:"Projects (E:)", fileSystem:"NTFS", totalCapacity:1024, systemUsed:488, systemUsedPercent:49, appUsed:201, appUsedPercent:34, available:512, appLimitValue:600, priority:3, status:"mounted" },
];

export const unmountedDrives: StorageDrive[] = [
  { driveId:"f", driveName:"Seagate Expansion", partitionName:"Backup (F:)", fileSystem:"exFAT", totalCapacity:1024, priority:4, status:"unmounted" },
];

import type { StorageDrive } from "../types/storage";

export const mountedDrives: StorageDrive[] = [
  { driveId:"c", driveName:"WD Blue", partitionName:"System (C:)", fileSystem:"NTFS", totalBytes:256, systemUsedBytes:21, systemUsedPercent:8, appUsedBytes:84, appUsedPercent:84, availableBytes:235, appLimitBytes:100, fileCount:0, priority:1, isMounted:true, isConnected:true, isSystem:true },
  { driveId:"d", driveName:"WD Blue", partitionName:"Work (D:)", fileSystem:"NTFS", totalBytes:2048, systemUsedBytes:812, systemUsedPercent:41, appUsedBytes:206, appUsedPercent:20, availableBytes:1228.8, appLimitBytes:1024, fileCount:0, priority:2, isMounted:true, isConnected:true, isSystem:false },
  { driveId:"e", driveName:"Samsung T7", partitionName:"Projects (E:)", fileSystem:"NTFS", totalBytes:1024, systemUsedBytes:488, systemUsedPercent:49, appUsedBytes:201, appUsedPercent:34, availableBytes:512, appLimitBytes:600, fileCount:0, priority:3, isMounted:true, isConnected:true, isSystem:false },
];

export const unmountedDrives: StorageDrive[] = [
  { driveId:"f", driveName:"Seagate Expansion", partitionName:"Backup (F:)", fileSystem:"exFAT", totalBytes:1024, fileCount:0, priority:4, isMounted:false, isConnected:true, isSystem:false },
];

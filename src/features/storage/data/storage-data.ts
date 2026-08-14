import type { StorageDrive } from "../types/storage";

export const mountedDrives: StorageDrive[] = [
  { id:"c", driveName:"WD Blue", partitionName:"System (C:)", fileSystem:"NTFS", total:"256 GB", used:"21 GB", usedPercent:8, appUsed:"84 GB", appUsedPercent:84, available:"235 GB", appLimitValue:100, appLimitUnit:"GB", priority:1, status:"mounted" },
  { id:"d", driveName:"WD Blue", partitionName:"Work (D:)", fileSystem:"NTFS", total:"2 TB", used:"812 GB", usedPercent:41, appUsed:"206 GB", appUsedPercent:20, available:"1.2 TB", appLimitValue:1, appLimitUnit:"TB", priority:2, status:"mounted" },
  { id:"e", driveName:"Samsung T7", partitionName:"Projects (E:)", fileSystem:"NTFS", total:"1 TB", used:"488 GB", usedPercent:49, appUsed:"201 GB", appUsedPercent:34, available:"512 GB", appLimitValue:600, appLimitUnit:"GB", priority:3, status:"mounted" },
];

export const unmountedDrives: StorageDrive[] = [
  { id:"f", driveName:"Seagate Expansion", partitionName:"Backup (F:)", fileSystem:"exFAT", total:"1 TB", appLimitUnit:"GB", priority:4, status:"unmounted" },
];

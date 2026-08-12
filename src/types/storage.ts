export type StorageKind = "ssd" | "hdd" | "unknown";

export interface StorageDevice {
  volumeId: string;
  name: string;
  mountPoint: string;
  fileSystem: string;
  physicalDiskId: string | null;
  hardwareModel: string | null;
  volumeLabel: string | null;
  kind: StorageKind;
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
  isRemovable: boolean;
  isReadOnly: boolean;
}

export interface StorageAllocation {
  schemaVersion: number;
  priority: number;
  volumeId: string;
  physicalDiskId: string | null;
  hardwareModel: string | null;
  volumeLabel: string | null;
  mountPoint: string;
  quotaBytes: number;
  vaultUsedBytes: number;
  updatedAtUnixMs: number;
}

export interface StorageTarget {
  volumeId: string;
  physicalDiskId: string | null;
  hardwareModel: string | null;
  mountPoint: string;
  priority: number;
  quotaBytes: number;
  vaultUsedBytes: number;
  deviceAvailableBytes: number;
  writableBytes: number;
}

export interface SetStorageAllocationRequest {
  volumeId: string;
  quotaBytes: number;
  priority: number;
}

export interface SelectStorageTargetRequest {
  requiredBytes: number;
}

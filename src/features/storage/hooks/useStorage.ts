import { useCallback, useEffect, useState } from "react";
import {
  getAvailableStorage,
  getStorageAllocations,
  isTauriRuntime,
  removeStorageAllocation,
  setStorageAllocation,
} from "@/services/tauri";
import type { StorageAllocation, StorageDevice } from "@/types/storage";

const GB = 1_000_000_000;
const previewDevices: StorageDevice[] = [
  { volumeId: "preview-c", name: "Samsung NVMe", mountPoint: "C:\\", fileSystem: "NTFS", physicalDiskId: "PHYSICALDRIVE0", hardwareModel: "Samsung 980 Pro NVMe", volumeLabel: null, kind: "ssd", totalBytes: 2_000 * GB, availableBytes: 800 * GB, usedBytes: 1_200 * GB, isRemovable: false, isReadOnly: false },
  { volumeId: "preview-d", name: "WD Blue SSD", mountPoint: "D:\\", fileSystem: "NTFS", physicalDiskId: "PHYSICALDRIVE1", hardwareModel: "WD Blue SATA SSD", volumeLabel: "Archive", kind: "ssd", totalBytes: 4_000 * GB, availableBytes: 900 * GB, usedBytes: 3_100 * GB, isRemovable: false, isReadOnly: false },
  { volumeId: "preview-e", name: "Seagate HDD", mountPoint: "E:\\", fileSystem: "NTFS", physicalDiskId: "PHYSICALDRIVE2", hardwareModel: "Seagate Barracuda HDD", volumeLabel: null, kind: "hdd", totalBytes: 4_000 * GB, availableBytes: 1_600 * GB, usedBytes: 2_400 * GB, isRemovable: false, isReadOnly: false },
];
const previewAllocations: StorageAllocation[] = previewDevices.map((device, index) => ({ schemaVersion: 1, priority: index + 1, volumeId: device.volumeId, physicalDiskId: device.physicalDiskId, hardwareModel: device.hardwareModel, volumeLabel: device.volumeLabel, mountPoint: device.mountPoint, quotaBytes: [800, 2_500, 2_000][index] * GB, vaultUsedBytes: [480, 1_800, 900][index] * GB, updatedAtUnixMs: Date.now() }));

export function useStorage() {
  const [devices, setDevices] = useState<StorageDevice[]>(isTauriRuntime() ? [] : previewDevices);
  const [allocations, setAllocations] = useState<StorageAllocation[]>(isTauriRuntime() ? [] : previewAllocations);
  const [loading, setLoading] = useState(isTauriRuntime());
  const [savingVolumeId, setSavingVolumeId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!isTauriRuntime()) return;
    setLoading(true);
    setError(null);
    try {
      const [nextDevices, nextAllocations] = await Promise.all([getAvailableStorage(), getStorageAllocations()]);
      setDevices(nextDevices);
      setAllocations(nextAllocations);
    } catch {
      setError("Storage devices could not be loaded.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  const setDriveEnabled = useCallback(async (volumeId: string, enabled: boolean) => {
    const device = devices.find((item) => item.volumeId === volumeId);
    if (!device) return false;
    setSavingVolumeId(volumeId);
    setError(null);
    try {
      if (enabled) {
        const request = {
          volumeId,
          quotaBytes: Math.max(GB, Math.min(device.availableBytes, device.totalBytes)),
          priority: allocations.length + 1,
        };
        if (isTauriRuntime()) {
          setAllocations(await setStorageAllocation(request));
        } else {
          setAllocations((current) => [...current, {
            schemaVersion: 1,
            priority: request.priority,
            volumeId,
            physicalDiskId: device.physicalDiskId,
            hardwareModel: device.hardwareModel,
            volumeLabel: device.volumeLabel,
            mountPoint: device.mountPoint,
            quotaBytes: request.quotaBytes,
            vaultUsedBytes: 0,
            updatedAtUnixMs: Date.now(),
          }]);
        }
      } else if (isTauriRuntime()) {
        setAllocations(await removeStorageAllocation(volumeId));
      } else {
        setAllocations((current) => current.filter((allocation) => allocation.volumeId !== volumeId));
      }
      return true;
    } catch {
      setError("The storage setting could not be saved.");
      return false;
    } finally {
      setSavingVolumeId(null);
    }
  }, [allocations.length, devices]);

  const updateQuota = useCallback(async (volumeId: string, quotaBytes: number) => {
    const allocation = allocations.find((item) => item.volumeId === volumeId);
    if (!allocation) return false;
    setSavingVolumeId(volumeId);
    setError(null);
    try {
      if (isTauriRuntime()) {
        setAllocations(await setStorageAllocation({ volumeId, quotaBytes, priority: allocation.priority }));
      } else {
        setAllocations((current) => current.map((item) => item.volumeId === volumeId ? { ...item, quotaBytes, updatedAtUnixMs: Date.now() } : item));
      }
      return true;
    } catch {
      setError("The allocation limit could not be saved.");
      return false;
    } finally {
      setSavingVolumeId(null);
    }
  }, [allocations]);

  return { allocations, devices, error, loading, refresh, savingVolumeId, setDriveEnabled, updateQuota };
}

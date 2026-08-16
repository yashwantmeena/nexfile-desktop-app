import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppSidebar } from "@/components/layout/AppSidebar";
import type { AppNavigationItem } from "@/types/navigation";
import { DriveTable } from "./components/DriveTable";
import { StorageHeader } from "./components/StorageHeader";
import { StorageOverview } from "./components/StorageOverview";
import type { StorageData, StorageDrive } from "./types/storage";
import "./storage.css";

interface StoragePageProps {
  activeNavigation: AppNavigationItem;
  onNavigationChange: (item: AppNavigationItem) => void;
}

const emptyStorageData: StorageData = {
  totalBytes: 0,
  availableBytes: 0,
  drivesDetected: 0,
  fileIndexed: 0,
  appLimitBytes: 0,
  appUsedBytes: 0,
  drives: [],
};

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) return error.message;
  return "Unable to load storage information.";
}

export function StoragePage({ activeNavigation, onNavigationChange }: StoragePageProps) {
  const [storageData, setStorageData] = useState<StorageData>(emptyStorageData);
  const [isScanning, setIsScanning] = useState(true);
  const [loadError, setLoadError] = useState<string>();
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  const loadStorageData = useCallback(async () => {
    setIsScanning(true);
    setLoadError(undefined);

    try {
      const data = await invoke<StorageData>("get_storage_data");
      setStorageData(data);
      setHasUnsavedChanges(false);
    } catch (error) {
      setLoadError(getErrorMessage(error));
    } finally {
      setIsScanning(false);
    }
  }, []);

  useEffect(() => {
    void loadStorageData();
  }, [loadStorageData]);

  const mountedDrives = useMemo(
    () => storageData.drives
      .filter((drive) => drive.isConnected && drive.isMounted)
      .sort((left, right) => left.priority - right.priority),
    [storageData.drives],
  );
  const unmountedDrives = useMemo(
    () => storageData.drives.filter((drive) => drive.isConnected && !drive.isMounted),
    [storageData.drives],
  );
  const unavailableDrives = useMemo(
    () => storageData.drives.filter((drive) => !drive.isConnected),
    [storageData.drives],
  );

  const updateDrives = (update: (drives: StorageDrive[]) => StorageDrive[]) => {
    setStorageData((current) => ({ ...current, drives: update(current.drives) }));
  };

  const mountDrive = (id: string) => {
    const nextPriority = mountedDrives.length + 1;
    updateDrives((drives) => drives.map((drive) => (
      drive.driveId === id && drive.isConnected
        ? { ...drive, isMounted: true, priority: nextPriority }
        : drive
    )));
    setHasUnsavedChanges(true);
  };

  const unmountDrive = (id: string) => {
    updateDrives((drives) => drives.map((drive) => (
      drive.driveId === id ? { ...drive, isMounted: false, priority: 0 } : drive
    )));
    setHasUnsavedChanges(true);
  };

  const movePriority = (id: string, direction: "up" | "down") => {
    const currentIndex = mountedDrives.findIndex((drive) => drive.driveId === id);
    const targetIndex = direction === "up" ? currentIndex - 1 : currentIndex + 1;
    if (currentIndex < 0 || targetIndex < 0 || targetIndex >= mountedDrives.length) return;

    const reordered = [...mountedDrives];
    [reordered[currentIndex], reordered[targetIndex]] = [reordered[targetIndex], reordered[currentIndex]];
    const priorities = new Map(reordered.map((drive, index) => [drive.driveId, index + 1]));
    updateDrives((drives) => drives.map((drive) => (
      priorities.has(drive.driveId)
        ? { ...drive, priority: priorities.get(drive.driveId) ?? drive.priority }
        : drive
    )));
    setHasUnsavedChanges(true);
  };

  return (
    <div className="nexfile-app">
      <AppSidebar activeItem={activeNavigation} onActiveItemChange={onNavigationChange} />
      <main className="nf-main storage-main">
        <div className="storage-page">
          <StorageHeader
            hasUnsavedChanges={hasUnsavedChanges}
            isScanning={isScanning}
            onSave={() => setHasUnsavedChanges(false)}
            onScan={() => void loadStorageData()}
          />
          {loadError && <p className="storage-load-error" role="alert">{loadError}</p>}
          <StorageOverview data={storageData} />
          <DriveTable
            title="Mounted Drives"
            description="Use the arrows to set storage priority. Higher drives are used first."
            drives={mountedDrives}
            onUnmount={unmountDrive}
            onMovePriority={movePriority}
          />
          <DriveTable
            title="Unmounted Drives"
            description="Connected partitions that are not mounted or are excluded from use."
            drives={unmountedDrives}
            onMount={mountDrive}
          />
          {unavailableDrives.length > 0 && (
            <DriveTable
              title="Unavailable Drives"
              description="Saved drives that are not currently connected."
              drives={unavailableDrives}
            />
          )}
        </div>
      </main>
    </div>
  );
}

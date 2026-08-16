import { HardDrive, RefreshCw, Save } from "lucide-react";

interface StorageHeaderProps {
  hasUnsavedChanges: boolean;
  isScanning: boolean;
  onSave: () => void;
  onScan: () => void;
}

export function StorageHeader({ hasUnsavedChanges, isScanning, onSave, onScan }: StorageHeaderProps) {
  return (
    <header className="storage-heading">
      <div className="storage-title-icon"><HardDrive /></div>
      <div><h1>Storage</h1><p>Manage your drives, set storage limits, and control where NexFile can store data.</p></div>
      <div className="storage-heading-actions">
        <button className="save-changes-button" disabled={!hasUnsavedChanges} onClick={onSave}><Save />Save Changes</button>
        <button className="scan-drives-button" disabled={isScanning} onClick={onScan}><RefreshCw />{isScanning ? "Scanning…" : "Scan for New Drives"}</button>
      </div>
    </header>
  );
}

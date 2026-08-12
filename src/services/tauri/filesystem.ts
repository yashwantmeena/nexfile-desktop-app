import { invoke } from "@tauri-apps/api/core";
import type { StorageDevice } from "@/types/storage";

export function getAvailableStorage(): Promise<StorageDevice[]> {
  return invoke<StorageDevice[]>("get_available_storage");
}

import { invoke } from "@tauri-apps/api/core";
import type {
  SelectStorageTargetRequest,
  SetStorageAllocationRequest,
  StorageAllocation,
  StorageTarget,
} from "@/types/storage";

export function setStorageAllocation(request: SetStorageAllocationRequest): Promise<StorageAllocation[]> {
  return invoke<StorageAllocation[]>("set_storage_allocation", { request });
}

export function getStorageAllocations(): Promise<StorageAllocation[]> {
  return invoke<StorageAllocation[]>("get_storage_allocations");
}

export function removeStorageAllocation(volumeId: string): Promise<StorageAllocation[]> {
  return invoke<StorageAllocation[]>("remove_storage_allocation", { volumeId });
}

export function clearStorageAllocations(): Promise<void> {
  return invoke<void>("clear_storage_allocations");
}

export function selectStorageTarget(request: SelectStorageTargetRequest): Promise<StorageTarget> {
  return invoke<StorageTarget>("select_storage_target", { request });
}

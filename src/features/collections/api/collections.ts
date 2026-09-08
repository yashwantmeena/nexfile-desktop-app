import { invoke } from "@tauri-apps/api/core";

export interface Collection {
  id: string;
  name: string;
  createdAtMs: number;
  updatedAtMs: number;
}

export const createCollection = (name: string) =>
  invoke<Collection[]>("create_collection", { name });
export const listCollections = () =>
  invoke<Collection[]>("list_collections");
export const updateCollection = (id: string, name: string) =>
  invoke<Collection[]>("update_collection", { id, name });
export const deleteCollection = (id: string) =>
  invoke<Collection[]>("delete_collection", { id });

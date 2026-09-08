import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import type { BackgroundProcess } from "../types/background-process";

export async function selectAndImportFiles(collectionIds: string[] = []): Promise<BackgroundProcess | null> {
  const selectedPaths = await open({
    directory: false,
    multiple: true,
    title: "Import files",
  });

  if (selectedPaths === null) return null;

  const paths = Array.isArray(selectedPaths) ? selectedPaths : [selectedPaths];
  if (paths.length === 0) return null;

  return invoke<BackgroundProcess>("import_file", { paths, collectionIds });
}

export async function selectAndImportFolder(collectionIds: string[] = []): Promise<BackgroundProcess | null> {
  const selectedPath = await open({
    directory: true,
    multiple: false,
    title: "Import folder",
  });

  if (selectedPath === null) return null;

  const path = Array.isArray(selectedPath) ? selectedPath[0] : selectedPath;
  if (!path) return null;

  return invoke<BackgroundProcess>("import_folder", { path, collectionIds });
}

export function getImportErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  ) return error.message;

  return "Unable to start the file import.";
}


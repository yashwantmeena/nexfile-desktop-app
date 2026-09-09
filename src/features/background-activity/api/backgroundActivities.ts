import { invoke } from "@tauri-apps/api/core";
import type { BackgroundProcess } from "@/features/import/types/background-process";

export const getBackgroundActivities = () =>
  invoke<BackgroundProcess[]>("get_background_activities");

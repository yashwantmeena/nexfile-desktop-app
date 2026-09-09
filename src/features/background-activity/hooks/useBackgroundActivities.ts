import { useCallback, useEffect, useState } from "react";
import type { BackgroundProcess } from "@/features/import/types/background-process";
import { getBackgroundActivities } from "../api/backgroundActivities";

const REFRESH_INTERVAL_MS = 2_000;

export function useBackgroundActivities() {
  const [activities, setActivities] = useState<BackgroundProcess[]>([]);
  const [error, setError] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setActivities(await getBackgroundActivities());
      setError(false);
    } catch {
      setError(true);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => {
      if (document.visibilityState === "visible") void refresh();
    }, REFRESH_INTERVAL_MS);
    window.addEventListener("focus", refresh);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", refresh);
    };
  }, [refresh]);

  return { activities, error, refresh };
}

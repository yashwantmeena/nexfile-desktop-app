import { Check, CircleAlert, Download, LoaderCircle, Search, Sparkles } from "lucide-react";
import { useBackgroundActivities } from "../hooks/useBackgroundActivities";

const activityTypes = [
  { label: "File processing", types: ["import_file", "import_folder", "export_file", "delete_file"], icon: Download },
  { label: "AI processing", types: ["image_processing"], icon: Sparkles },
  { label: "Indexing", types: ["indexing", "delete_index"], icon: Search },
] as const;

export function BackgroundActivities() {
  const { activities, error, refresh } = useBackgroundActivities();

  if (error && !activities.length) {
    return <button className="background-activity-error" type="button" onClick={() => void refresh()}>
      <CircleAlert />
      <span><strong>Activity unavailable</strong><small>Click to retry</small></span>
    </button>;
  }

  const active = activityTypes.map(configuration => ({
    ...configuration,
    processes: activities.filter(process => configuration.types.some(type => type === process.processType)),
  })).filter(activity => activity.processes.length > 0);

  if (!active.length) {
    return <div className="background-activity-idle"><Check /><span>All tasks complete</span></div>;
  }

  return <div className="background-activities" aria-live="polite">
    {active.map(({ label, icon: Icon, processes }) => {
      const running = processes.some(process => process.status === "running");
      return <div className="background-activity-row" key={label}>
        <span className="background-activity-icon"><Icon /></span>
        <span className="background-activity-copy"><strong>{label}</strong><small>{running ? "Working…" : "Queued…"}</small></span>
        <LoaderCircle className="background-activity-spinner" aria-hidden="true" />
      </div>;
    })}
  </div>;
}

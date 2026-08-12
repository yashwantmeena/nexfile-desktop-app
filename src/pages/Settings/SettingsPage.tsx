import { Settings } from "lucide-react";

export function SettingsPage({ onNotice }: { onNotice: (message: string) => void }) {
  return (
    <main className="dashboard secondary-page">
      <div className="secondary-page-card"><Settings /><h1>Settings</h1><p>Manage LocalMind preferences and local processing options.</p><button type="button" onClick={() => onNotice("Settings refreshed")}>Refresh settings</button></div>
    </main>
  );
}

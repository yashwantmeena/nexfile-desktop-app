import { useState } from "react";
import { Check, Sparkles } from "lucide-react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ingestionLogs } from "@/data/archive-data";
import type { LogRange } from "@/types/archive";

const ranges: LogRange[] = ["Today", "This Week"];

export function IngestionLog() {
  const [range, setRange] = useState<LogRange>("Today");

  return (
    <section className="panel log-panel">
      <div className="log-header">
        <h2>Recent Activity</h2>
        <Tabs value={range} onValueChange={(value) => setRange(value as LogRange)}>
          <TabsList className="segmented-control" aria-label="Show activity by date">
            {ranges.map((item) => <TabsTrigger key={item} value={item}>{item}</TabsTrigger>)}
          </TabsList>
        </Tabs>
      </div>
      <div className="table-scroll">
        <table>
          <thead><tr><th>Name</th><th>Date Added</th><th>Size</th><th>Tags</th><th>Status</th></tr></thead>
          <tbody>
            {ingestionLogs.map((log) => (
              <tr key={log.name}>
                <td>{log.name}</td><td>{log.date}</td><td>{log.size}</td><td>{log.density}</td>
                <td><span className={log.status === "Ready" ? "status indexed" : "status processing"}>{log.status === "Ready" ? <Check size={13} /> : <Sparkles size={13} />}{log.status}</span></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

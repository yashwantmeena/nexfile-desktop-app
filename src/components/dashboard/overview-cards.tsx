import {
  Database,
  FileText,
  Folder,
  FolderSearch,
  SlidersHorizontal,
  Sparkles,
  Tags,
  Upload,
} from "lucide-react";

interface OverviewCardsProps {
  onAction: (message: string) => void;
}

const metrics = [
  { icon: FileText, value: "12,847", label: "Total Files", tone: "indigo" },
  { icon: Database, value: "284", suffix: "GB / 512 GB", label: "Storage Used", tone: "teal" },
  { icon: Tags, value: "3,421", label: "AI Tags Generated", tone: "violet" },
  { icon: Folder, value: "64", label: "Folders Indexed", tone: "amber" },
];

const quickActions = [
  { icon: Upload, title: "Import Files", detail: "Add to vault", tone: "indigo", message: "Choose files with the Import Files button above" },
  { icon: FolderSearch, title: "Scan Folders", detail: "Index new dirs", tone: "teal", message: "Choose a folder with the Scan Folders button above" },
  { icon: Sparkles, title: "Generate Tags", detail: "Run AI locally", tone: "violet", message: "Local AI tag generation started" },
  { icon: SlidersHorizontal, title: "Manage Storage", detail: "Free up space", tone: "amber", message: "Storage manager opened" },
];

export function OverviewCards({ onAction }: OverviewCardsProps) {
  return (
    <>
      <section className="metric-grid" aria-label="Library overview">
        {metrics.map(({ icon: Icon, value, suffix, label, tone }) => (
          <article className="metric-card" key={label}>
            <span className={`metric-icon ${tone}`}><Icon /></span>
            <strong>{value}{suffix && <small> {suffix}</small>}</strong>
            <p>{label}</p>
          </article>
        ))}
      </section>

      <section className="quick-grid" aria-label="Quick actions">
        {quickActions.map(({ icon: Icon, title, detail, tone, message }) => (
          <button type="button" className="quick-card" key={title} onClick={() => onAction(message)}>
            <span className={`quick-icon ${tone}`}><Icon /></span>
            <span><strong>{title}</strong><small>{detail}</small></span>
          </button>
        ))}
      </section>
    </>
  );
}

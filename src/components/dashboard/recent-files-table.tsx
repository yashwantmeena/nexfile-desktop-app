import { FileText, History, Image, MoreVertical, Sheet, Video } from "lucide-react";
import type { RecentFile } from "@/types/localmind";

interface RecentFilesTableProps {
  files: RecentFile[];
  onNotice: (message: string) => void;
}

const fileIcons = {
  pdf: FileText,
  image: Image,
  sheet: Sheet,
  doc: FileText,
  video: Video,
};

export function RecentFilesTable({ files, onNotice }: RecentFilesTableProps) {
  return (
    <section className="recent-panel">
      <header className="panel-heading">
        <h2><History />Recently Accessed Files</h2>
        <button type="button" onClick={() => onNotice("Showing all recently accessed files")}>View all</button>
      </header>

      <div className="file-table" role="table" aria-label="Recently accessed files">
        <div className="file-table-header" role="row">
          <span role="columnheader">File name</span>
          <span role="columnheader">Tags</span>
          <span role="columnheader">Accessed</span>
          <span role="columnheader">Action</span>
        </div>
        {files.length === 0 ? (
          <div className="no-results"><strong>No matching files</strong><span>Try a different name or tag.</span></div>
        ) : files.map((file) => {
          const Icon = fileIcons[file.kind];
          return (
            <div className="file-row" role="row" key={file.id}>
              <div className="file-name-cell" role="cell">
                <span className={`file-icon ${file.kind}`}><Icon /></span>
                <span><strong>{file.name}</strong><small>{file.location}</small></span>
              </div>
              <div className="row-tags" role="cell">
                {file.tags.map((tag) => <span key={tag}>#{tag}</span>)}
              </div>
              <span className="accessed-cell" role="cell">{file.accessed}</span>
              <button type="button" className="row-menu" aria-label={`More actions for ${file.name}`} onClick={() => onNotice(`Actions opened for ${file.name}`)}><MoreVertical /></button>
            </div>
          );
        })}
      </div>
    </section>
  );
}

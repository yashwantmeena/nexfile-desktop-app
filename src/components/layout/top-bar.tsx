import { useRef } from "react";
import { FolderSearch, Menu, Plus } from "lucide-react";
import { Button } from "@/components/ui/button";

interface TopBarProps {
  onMenuOpen: () => void;
  onNotice: (message: string) => void;
}

export function TopBar({ onMenuOpen, onNotice }: TopBarProps) {
  const fileInput = useRef<HTMLInputElement>(null);
  const folderInput = useRef<HTMLInputElement>(null);

  return (
    <header className="topbar">
      <Button type="button" variant="ghost" size="icon" className="mobile-menu" aria-label="Open navigation" onClick={onMenuOpen}><Menu /></Button>
      <div className="greeting-avatar" aria-hidden="true">A</div>
      <div className="greeting-copy">
        <strong>Good morning, Alex</strong>
        <span>Your files are private, organized, and ready.</span>
      </div>
      <div className="top-actions">
        <Button type="button" className="primary-action" onClick={() => fileInput.current?.click()}><Plus />Import Files</Button>
        <Button type="button" variant="outline" className="secondary-action" onClick={() => folderInput.current?.click()}><FolderSearch />Scan Folders</Button>
      </div>
      <input
        ref={fileInput}
        className="visually-hidden"
        type="file"
        multiple
        onChange={(event) => onNotice(`${event.target.files?.length ?? 0} file${event.target.files?.length === 1 ? "" : "s"} queued for local import`)}
      />
      <input
        ref={folderInput}
        className="visually-hidden"
        type="file"
        multiple
        {...({ webkitdirectory: "", directory: "" } as React.InputHTMLAttributes<HTMLInputElement>)}
        onChange={(event) => onNotice(`${event.target.files?.length ?? 0} folder item${event.target.files?.length === 1 ? "" : "s"} queued for indexing`)}
      />
    </header>
  );
}

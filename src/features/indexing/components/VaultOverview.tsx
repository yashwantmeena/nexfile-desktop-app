import { useRef, type InputHTMLAttributes } from "react";
import { FolderInput } from "lucide-react";
import { Button } from "@/components/Button/Button";

interface VaultOverviewProps {
  onNotice: (message: string) => void;
}

export function VaultOverview({ onNotice }: VaultOverviewProps) {
  const fileInput = useRef<HTMLInputElement>(null);

  return (
    <section className="vault-card">
      <div className="storage-overview">
        <div className="storage-heading">
          <p className="eyebrow">STORAGE OVERVIEW</p>
          <p>Everything stays on this device</p>
        </div>
        <div className="storage-stats">
          <div><span>Total storage</span><strong>128 GB</strong></div>
          <div><span>Used</span><strong>42,4 GB</strong></div>
          <div><span>Free</span><strong>85,6 GB</strong></div>
        </div>
        <div className="storage-progress" role="progressbar" aria-label="Storage used" aria-valuemin={0} aria-valuemax={128} aria-valuenow={42.4}>
          <span />
        </div>
      </div>
      <div className="vault-rings" aria-hidden="true" />
      <Button className="import-button" onClick={() => fileInput.current?.click()}><FolderInput />Add Folder</Button>
      <input
        ref={fileInput}
        className="visually-hidden"
        type="file"
        multiple
        {...({ webkitdirectory: "" } as InputHTMLAttributes<HTMLInputElement>)}
        onChange={(event) => onNotice(`${event.target.files?.length ?? 0} files selected`)}
      />
    </section>
  );
}

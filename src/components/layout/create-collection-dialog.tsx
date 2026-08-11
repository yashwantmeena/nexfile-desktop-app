import { useState, type FormEvent } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Dialog, DialogClose, DialogContent, DialogDescription, DialogTitle, DialogTrigger } from "@/components/ui/dialog";

interface CreateCollectionDialogProps {
  onCreated: (name: string) => void;
}

export function CreateCollectionDialog({ onCreated }: CreateCollectionDialogProps) {
  const [open, setOpen] = useState(false);
  const [name, setName] = useState("");

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const value = name.trim();
    if (!value) return;
    onCreated(value);
    setName("");
    setOpen(false);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="ghost" className="create-button"><Plus />New Collection</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogTitle>New collection</DialogTitle>
        <DialogDescription>Keep related photos and files together in one place.</DialogDescription>
        <form className="collection-form" onSubmit={submit}>
          <label htmlFor="collection-name">Name</label>
          <input id="collection-name" value={name} onChange={(event) => setName(event.target.value)} placeholder="For example: Family photos" autoFocus />
          <div className="dialog-actions">
            <DialogClose asChild><Button type="button" variant="ghost">Cancel</Button></DialogClose>
            <Button type="submit" disabled={!name.trim()}>Save</Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}

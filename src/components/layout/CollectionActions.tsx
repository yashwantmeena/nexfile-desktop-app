import { DropdownMenu } from "radix-ui";
import { Ellipsis, Pencil, Trash2 } from "lucide-react";
import { useCollections } from "./CollectionsProvider";

export function CollectionActions({ id, name }: { id: string; name: string }) {
  const { openEdit, openDelete } = useCollections();
  return <DropdownMenu.Root>
    <DropdownMenu.Trigger asChild><button type="button" className="collection-actions-trigger" aria-label={`Manage ${name}`} title="Manage collection"><Ellipsis size={17}/></button></DropdownMenu.Trigger>
    <DropdownMenu.Portal><DropdownMenu.Content className="collection-actions-menu" align="end" sideOffset={5} collisionPadding={12}>
      <DropdownMenu.Item onSelect={() => { window.setTimeout(() => openEdit(id), 0); }}><Pencil size={15}/>Rename collection</DropdownMenu.Item>
      <DropdownMenu.Separator/>
      <DropdownMenu.Item className="collection-delete-action" onSelect={() => { window.setTimeout(() => openDelete(id), 0); }}><Trash2 size={15}/>Delete collection</DropdownMenu.Item>
    </DropdownMenu.Content></DropdownMenu.Portal>
  </DropdownMenu.Root>;
}

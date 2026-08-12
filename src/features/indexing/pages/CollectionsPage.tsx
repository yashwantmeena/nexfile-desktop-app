import { Folder, Images } from "lucide-react";
import { PageHeader } from "@/features/indexing/components/PageHeader";
import type { CollectionSummary } from "@/features/indexing/types";

interface CollectionsPageProps {
  collections: CollectionSummary[];
  query: string;
}

export function CollectionsPage({ collections, query }: CollectionsPageProps) {
  const normalized = query.trim().toLowerCase();
  const visibleCollections = collections.filter((collection) => !normalized || collection.name.toLowerCase().includes(normalized));

  return (
    <div className="page-view">
      <PageHeader title="Collections" description="Keep related photos and files together." />
      {visibleCollections.length > 0 ? (
        <section className="collection-grid" aria-label="Your collections">
          {visibleCollections.map((collection, index) => (
            <article className="collection-card" key={collection.id} tabIndex={0}>
              <div className={`collection-cover cover-${index % 3 + 1}`}><Folder size={24} /></div>
              <div className="collection-card-body">
                <strong>{collection.name}</strong>
                <span><Images size={14} />{collection.itemCount} items</span>
                <small>{collection.updatedLabel}</small>
              </div>
            </article>
          ))}
        </section>
      ) : <div className="page-empty"><Folder size={25} /><strong>No collections found</strong><span>Try another search or create a new collection.</span></div>}
    </div>
  );
}

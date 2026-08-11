import { Search, Sparkles } from "lucide-react";

interface SearchPanelProps {
  query: string;
  activeTag: string | null;
  onQueryChange: (query: string) => void;
  onTagChange: (tag: string | null) => void;
}

const suggestedTags = ["invoices", "mountain-trip", "tax-2024", "work-docs"];

export function SearchPanel({ query, activeTag, onQueryChange, onTagChange }: SearchPanelProps) {
  return (
    <section className="search-panel" aria-label="Search your files">
      <label className="search-control">
        <Search aria-hidden="true" />
        <input
          type="search"
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder="Search files by name, tags, or describe what you're looking for..."
          aria-label="Search files"
        />
        <span className="ai-badge"><Sparkles />AI</span>
        <kbd>⌘K</kbd>
      </label>
      <div className="suggested-tags" aria-label="Suggested search tags">
        {suggestedTags.map((tag) => (
          <button
            key={tag}
            type="button"
            className={activeTag === tag ? "selected" : ""}
            onClick={() => onTagChange(activeTag === tag ? null : tag)}
          >
            #{tag}
          </button>
        ))}
      </div>
    </section>
  );
}

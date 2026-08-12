import type { RecentFile } from "@/types/file";
import type { SearchState } from "@/types/search";

export function filterFiles(files: RecentFile[], { query, activeTag }: SearchState) {
  const needle = query.trim().toLowerCase();
  return files.filter((file) => {
    const matchesQuery = !needle || [file.name, file.location, ...file.tags].some((value) => value.toLowerCase().includes(needle));
    const matchesTag = !activeTag || file.tags.some((tag) => tag.includes(activeTag.replace("-docs", "")));
    return matchesQuery && matchesTag;
  });
}

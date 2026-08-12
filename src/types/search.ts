export interface SearchState {
  query: string;
  activeTag: string | null;
}

export interface SearchControls extends SearchState {
  onQueryChange: (query: string) => void;
  onTagChange: (tag: string | null) => void;
}

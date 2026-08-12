export interface SearchCommandRequest {
  query: string;
  tags?: string[];
}

// Search commands will be added here when the Rust search boundary is implemented.
export type SearchCommandResult = never;

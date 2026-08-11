import type { FolderSummary, RecentFile } from "@/types/localmind";

export const recentFiles: RecentFile[] = [
  { id: 1, name: "Q3_Financial_Report.pdf", location: "/Documents/Finance", tags: ["invoice", "finance"], accessed: "2 hours ago", kind: "pdf" },
  { id: 2, name: "mountain_trip_2024.jpg", location: "/Photos/Trips", tags: ["travel", "landscape"], accessed: "Yesterday", kind: "image" },
  { id: 3, name: "Tax_Returns_2023.xlsx", location: "/Documents/Taxes", tags: ["tax-2024", "important"], accessed: "3 days ago", kind: "sheet" },
  { id: 4, name: "Project_Proposal_v3.docx", location: "/Work/Projects", tags: ["work", "proposal"], accessed: "4 days ago", kind: "doc" },
  { id: 5, name: "family_reunion.mp4", location: "/Videos/Family", tags: ["family", "video"], accessed: "1 week ago", kind: "video" },
];

export const folderSummaries: FolderSummary[] = [
  { path: "/Documents", detail: "4,231 files · 82 GB", progress: 76 },
  { path: "/Photos", detail: "3,890 files · 94 GB", progress: 91 },
  { path: "/Videos", detail: "421 files · 78 GB", progress: 68 },
  { path: "/Work", detail: "2,104 files · 30 GB", progress: 54 },
];

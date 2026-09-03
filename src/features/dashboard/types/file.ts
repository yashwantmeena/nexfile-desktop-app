export interface DashboardFile {
  id:string | number; name:string; path:string; kind:string; time:string;
  sizeBytes?:number; fileType?:string;
  categories?:string[];
  tags?:string[]; collection?:string;
  image?:string; duration?:string;
}

export interface FileCategory { label:string; count:string; }

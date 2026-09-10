export interface DashboardFile {
  id:string | number; driveId?:string; name:string; path:string; kind:string; time:string;
  sizeBytes?:number; fileType?:string;
  modifiedAtMs?:number | null;
  capturedAtMs?:number | null;
  categories?:string[];
  tags?:string[]; collection?:string; collections?:string[];
  favorite?:boolean;
  isTrashed?:boolean;
  image?:string; duration?:string;
}

export interface FileCategory { label:string; count:string; }

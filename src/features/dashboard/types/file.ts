export interface DashboardFile {
  id:number; name:string; path:string; kind:string; time:string;
  image?:string; duration?:string;
}

export interface FileCategory { label:string; count:string; }

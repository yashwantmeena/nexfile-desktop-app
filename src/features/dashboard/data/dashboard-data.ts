import type { DashboardFile, FileCategory } from "../types/file";

export const mountainImage="https://images.unsplash.com/photo-1501785888041-af3ef285b470?auto=format&fit=crop&w=900&q=84";
const interiorImage="https://images.unsplash.com/photo-1600210492486-724fe5c67fb0?auto=format&fit=crop&w=900&q=84";
const beachImage="https://images.unsplash.com/photo-1507525428034-b723cf961d3e?auto=format&fit=crop&w=900&q=84";

export const dashboardFiles:DashboardFile[]=[
  {id:1,name:"mountains_lake.jpg",path:"/Photos/Travel",kind:"JPG",time:"2m ago",image:mountainImage},
  {id:2,name:"Project_Proposal.pdf",path:"/Work/Proposals",kind:"PDF",time:"10m ago"},
  {id:3,name:"Dashboard_UI.fig",path:"/Design/NexFile",kind:"FIG",time:"15m ago"},
  {id:4,name:"Meeting_Notes.docx",path:"/Work/Meetings",kind:"DOCX",time:"1h ago"},
  {id:5,name:"Q2_Budget.xlsx",path:"/Work/Finance",kind:"XLSX",time:"2h ago"},
  {id:6,name:"assets_backup.zip",path:"/Design/Backup",kind:"ZIP",time:"3h ago"},
  {id:7,name:"interior_design.png",path:"/Design/References",kind:"PNG",time:"5h ago",image:interiorImage},
  {id:8,name:"product_demo.mp4",path:"/Videos/Demos",kind:"MP4",time:"6h ago",image:beachImage,duration:"01:24"},
  {id:9,name:"lofi_focus.mp3",path:"/Audio/Music",kind:"MP3",time:"8h ago"},
  {id:10,name:"readme.md",path:"/Docs",kind:"MD",time:"1d ago"},
  {id:11,name:"search_service.ts",path:"/Projects/NexFile/src",kind:"TS",time:"1d ago"},
  {id:12,name:"Product_Update.pptx",path:"/Work/Presentations",kind:"PPTX",time:"2d ago"},
];

export const categories:FileCategory[]=[
  {label:"All",count:"12,700"},{label:"Images",count:"4,219"},{label:"PDFs",count:"1,843"},{label:"Documents",count:"2,306"},
  {label:"Videos",count:"812"},{label:"Audio",count:"342"},{label:"Archives",count:"556"},{label:"Others",count:"622"},
];

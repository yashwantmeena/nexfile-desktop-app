import { useEffect, useState } from "react";
import { Archive, File, FileCode2, FileText, Film, Mic2, Play } from "lucide-react";
import type { DashboardFile } from "../types/file";

interface FileCardProps { file:DashboardFile; previewUrl?:string; onOpen:()=>void; }

function FileArtwork({ file, showPreview, onPreviewError }: { file:DashboardFile; showPreview:boolean; onPreviewError:()=>void }) {
  if(showPreview && file.image) return <div className="photo-art"><img src={file.image} alt="" loading="lazy" decoding="async" onError={onPreviewError}/>{(file.fileType === "video" || file.duration)&&<><span className="play"><Play /></span>{file.duration&&<b className="duration">{file.duration}</b>}</>}</div>;
  const isWebm = file.kind === "WEBM";
  const isVideo = file.fileType === "video" && !isWebm;
  const isAudio = file.fileType === "audio";
  const Icon=isVideo?Film:file.kind==="ZIP"?Archive:file.kind==="TS"?FileCode2:file.kind==="MD"?FileText:File;
  const fallbackType = isWebm ? "other" : file.fileType ?? "other";
  return <div className={`fallback-art fallback-${fallbackType} fallback-kind-${file.kind.toLowerCase()}`}><span className={`fallback-icon${isAudio?" voice-file-icon":""}`}>{isAudio?<Mic2/>:<Icon/>}</span>{isAudio&&<span className="fallback-waveform"><i/><i/><i/><i/><i/><i/><i/></span>}</div>;
}

export function FileCard({ file, previewUrl, onOpen }:FileCardProps) {
  const [previewFailed, setPreviewFailed] = useState(false);
  useEffect(() => setPreviewFailed(false), [previewUrl]);
  const showPreview = Boolean(previewUrl) && !previewFailed;
  return <article className={`file-card visual-file-card ${showPreview?"visual-only-file-card":"named-file-card"}`} role="button" tabIndex={0} aria-label={`Preview ${file.name}`} onClick={onOpen} onKeyDown={event=>{if(event.key==="Enter"||event.key===" "){event.preventDefault();onOpen();}}}>
    <div className="variant-artwork"><FileArtwork file={{...file, image:previewUrl}} showPreview={showPreview} onPreviewError={()=>setPreviewFailed(true)}/></div>
    {!showPreview && <div className="fallback-file-details">
      <strong title={file.name}>{file.name}</strong>
    </div>}
  </article>;
}

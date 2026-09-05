import { useEffect, useState } from "react";
import { Archive, File, FileCode2, FileText, Film, Mic2, Play } from "lucide-react";
import type { DashboardFile } from "../types/file";

interface FileCardProps { file:DashboardFile; onOpen:()=>void; }

function FileArtwork({ file, showPreview, onPreviewError }: { file:DashboardFile; showPreview:boolean; onPreviewError:()=>void }) {
  if(showPreview && file.image && file.fileType === "video") return <div className="photo-art video-art"><video src={file.image} preload="metadata" muted playsInline onLoadedData={event => { if (event.currentTarget.duration > .1) event.currentTarget.currentTime = .1; }} onError={onPreviewError}/><span className="play"><Play /></span></div>;
  if(showPreview && file.image) return <div className="photo-art"><img src={file.image} alt="" loading="lazy" onError={onPreviewError}/>{file.duration&&<><span className="play"><Play /></span><b className="duration">{file.duration}</b></>}</div>;
  const isWebm = file.kind === "WEBM";
  const isVideo = file.fileType === "video" && !isWebm;
  const isAudio = file.fileType === "audio";
  const Icon=isVideo?Film:file.kind==="ZIP"?Archive:file.kind==="TS"?FileCode2:file.kind==="MD"?FileText:File;
  const fallbackType = isWebm ? "other" : file.fileType ?? "other";
  return <div className={`fallback-art fallback-${fallbackType} fallback-kind-${file.kind.toLowerCase()}`}><span className={`fallback-icon${isAudio?" voice-file-icon":""}`}>{isAudio?<Mic2/>:<Icon/>}</span>{isAudio&&<span className="fallback-waveform"><i/><i/><i/><i/><i/><i/><i/></span>}</div>;
}

export function FileCard({ file, onOpen }:FileCardProps) {
  const [previewFailed, setPreviewFailed] = useState(false);
  useEffect(() => setPreviewFailed(false), [file.image]);
  const showPreview = Boolean(file.image) && !previewFailed && file.kind !== "WEBM";
  return <article className={`file-card visual-file-card ${showPreview?"visual-only-file-card":"named-file-card"}`} role="button" tabIndex={0} aria-label={`Preview ${file.name}`} onClick={onOpen} onKeyDown={event=>{if(event.key==="Enter"||event.key===" "){event.preventDefault();onOpen();}}}>
    <div className="variant-artwork"><FileArtwork file={file} showPreview={showPreview} onPreviewError={()=>setPreviewFailed(true)}/><span className={`card-extension-badge ${file.kind.toLowerCase()}`}>{file.kind}</span></div>
    {!showPreview && <div className="fallback-file-details">
      <strong title={file.name}>{file.name}</strong>
    </div>}
  </article>;
}

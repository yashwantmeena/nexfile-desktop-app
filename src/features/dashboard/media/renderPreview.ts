export interface PreviewSource { key: string; url: string; video: boolean; }

export function renderPreview(source: PreviewSource, signal: AbortSignal): Promise<Blob> {
  return source.video ? renderVideo(source.url, signal) : renderImage(source.url, signal);
}

function renderImage(url: string, signal: AbortSignal): Promise<Blob> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(new DOMException("Cancelled", "AbortError")); return; }
    const worker = new Worker(new URL("./preview.worker.ts", import.meta.url), { type: "module" });
    const cleanup = () => { clearTimeout(timeout); signal.removeEventListener("abort", abort); worker.terminate(); };
    const abort = () => { cleanup(); reject(new DOMException("Cancelled", "AbortError")); };
    const timeout = setTimeout(() => { cleanup(); reject(new Error("Image preview timed out")); }, 30000);
    signal.addEventListener("abort", abort, { once: true });
    worker.onmessage = (event: MessageEvent<{ blob?: Blob; error?: string }>) => {
      cleanup();
      if (event.data.blob) resolve(event.data.blob);
      else reject(new Error(event.data.error || "Unable to resize image"));
    };
    worker.onerror = () => { cleanup(); reject(new Error("Image preview worker failed")); };
    worker.postMessage({ url });
  });
}

// Videos need a media element to decode a frame. The scheduler permits only one
// at a time; it is detached and released immediately after producing a poster.
function renderVideo(url: string, signal: AbortSignal): Promise<Blob> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(new DOMException("Cancelled", "AbortError")); return; }
    const video = document.createElement("video");
    let capturing = false;
    let settled = false;
    const cleanup = () => {
      settled = true;
      clearTimeout(timeout);
      signal.removeEventListener("abort", abort);
      video.onloadedmetadata = video.onloadeddata = video.onseeked = video.onerror = null;
      video.pause();
      video.removeAttribute("src");
      video.load();
    };
    const fail = (error: Error) => { if (!settled) { cleanup(); reject(error); } };
    const abort = () => fail(new DOMException("Cancelled", "AbortError"));
    const timeout = setTimeout(() => fail(new Error("Video preview timed out")), 15000);
    const capture = () => {
      if (capturing || settled || video.readyState < 2 || video.seeking) return;
      capturing = true;
      try {
        if (!video.videoWidth || !video.videoHeight) throw new Error("Empty video frame");
        const scale = Math.min(1, 512 / Math.max(video.videoWidth, video.videoHeight));
        const canvas = document.createElement("canvas");
        canvas.width = Math.max(1, Math.round(video.videoWidth * scale));
        canvas.height = Math.max(1, Math.round(video.videoHeight * scale));
        const context = canvas.getContext("2d");
        if (!context) throw new Error("Video canvas unavailable");
        context.drawImage(video, 0, 0, canvas.width, canvas.height);
        canvas.toBlob(blob => {
          canvas.width = canvas.height = 1;
          if (settled) return;
          if (blob) { cleanup(); resolve(blob); }
          else fail(new Error("Unable to encode video poster"));
        }, "image/webp", 0.75);
      } catch (error) { fail(error instanceof Error ? error : new Error(String(error))); }
    };
    signal.addEventListener("abort", abort, { once: true });
    video.crossOrigin = "anonymous";
    video.muted = true;
    video.playsInline = true;
    video.preload = "auto";
    video.onloadedmetadata = () => {
      if (video.duration > 0.1) video.currentTime = 0.1;
    };
    video.onloadeddata = capture;
    video.onseeked = capture;
    video.onerror = () => fail(new Error("Unsupported or unreadable video"));
    video.src = url;
    video.load();
  });
}

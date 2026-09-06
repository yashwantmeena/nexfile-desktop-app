import { useLayoutEffect, useRef, useState } from "react";
import { Maximize2, Minus, Plus } from "lucide-react";

interface OriginalImagePreviewProps { src: string; name: string; }

/** Scale the original's intrinsic dimensions, rather than scaling an already
 * fitted CSS box. Null zoom follows the available viewport as it resizes. */
export function OriginalImagePreview({ src, name }: OriginalImagePreviewProps) {
  const viewportRef = useRef<HTMLDivElement>(null);
  const [viewport, setViewport] = useState({ width: 0, height: 0 });
  const [natural, setNatural] = useState({ width: 0, height: 0 });
  const [zoom, setZoom] = useState<number | null>(null);
  const [failed, setFailed] = useState(false);
  useLayoutEffect(() => {
    const viewport = viewportRef.current!;
    const measure = () => {
      // clientWidth rounds fractional CSS pixels. Leave one pixel to avoid
      // scrollbar oscillation on displays with fractional Windows scaling.
      const width = Math.max(0, viewport.clientWidth - 1);
      const height = Math.max(0, viewport.clientHeight - 1);
      setViewport(previous => previous.width === width && previous.height === height ? previous : { width, height });
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(viewport);
    return () => observer.disconnect();
  }, []);
  const fit = natural.width && natural.height && viewport.width && viewport.height
    ? Math.min(1, viewport.width / natural.width, viewport.height / natural.height) : 1;
  const scale = zoom ?? fit;
  const width = natural.width * scale;
  const height = natural.height * scale;
  useLayoutEffect(() => {
    // Keep the center accessible when zooming; all edges remain scrollable.
    const element = viewportRef.current!;
    element.scrollLeft = Math.max(0, (width - element.clientWidth) / 2);
    element.scrollTop = Math.max(0, (height - element.clientHeight) / 2);
  }, [width, height]);
  return <>
    <div className="preview-image-viewport" ref={viewportRef}>
      {failed ? <div className="preview-fallback" role="status">Unable to display this image.</div> :
        <div className="preview-image-surface" style={{ width: Math.max(viewport.width, width), height: Math.max(viewport.height, height) }}>
          <img src={src} alt={name} decoding="async" draggable={false}
            onLoad={event => setNatural({ width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight })}
            onError={() => setFailed(true)}
            style={{ width: width || 1, height: height || 1, visibility: natural.width ? "visible" : "hidden" }}/>
        </div>}
    </div>
    {!failed && <div className="preview-zoom">
      <button aria-label="Zoom out" disabled={!natural.width} onClick={() => setZoom(Math.max(fit / 4, scale / 1.25))}><Minus/></button>
      <strong title="Scale relative to the original image dimensions">{natural.width ? `${Number((scale * 100).toFixed(1))}%` : "…"}</strong>
      <button aria-label="Zoom in" disabled={!natural.width} onClick={() => setZoom(Math.min(4, scale * 1.25))}><Plus/></button>
      <i/>
      <button aria-label="Actual size (100%)" title="Actual size (100%)" disabled={!natural.width} onClick={() => setZoom(1)}>1:1</button>
      <button aria-label="Fit to screen" title="Fit complete image" onClick={() => setZoom(null)}><Maximize2/></button>
    </div>}
  </>;
}

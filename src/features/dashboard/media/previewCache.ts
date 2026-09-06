import type { PreviewSource } from "./renderPreview";

type Renderer = (source: PreviewSource, signal: AbortSignal) => Promise<Blob>;

/** The caller supplies only visible sources.
 * Only this moving window is retained. No persistent cache or source blobs. */
export class PreviewCache {
  private desired: PreviewSource[] = [];
  private active = new Map<string, { controller: AbortController; video: boolean }>();
  private failed = new Set<string>();
  private urls = new Map<string, string>();
  private listeners = new Set<() => void>();

  constructor(private render: Renderer) {}
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  snapshot = () => this.urls;
  private notify() { for (const listener of this.listeners) listener(); }

  setWindow(sources: PreviewSource[]) {
    this.desired = [...new Map(sources.map(source => [source.key, source])).values()];
    const keys = new Set(this.desired.map(source => source.key));
    const next = new Map(this.urls);
    for (const [key, url] of next) {
      if (!keys.has(key)) { URL.revokeObjectURL(url); next.delete(key); }
    }
    for (const [key, job] of this.active) if (!keys.has(key)) job.controller.abort();
    for (const key of this.failed) if (!keys.has(key)) this.failed.delete(key);
    if (next.size !== this.urls.size) { this.urls = next; this.notify(); }
    this.pump();
  }

  private pump() {
    while (this.active.size < 2) {
      const videoBusy = [...this.active.values()].some(job => job.video);
      const source = this.desired.find(item => !this.urls.has(item.key) && !this.failed.has(item.key) && !this.active.has(item.key) && !(item.video && videoBusy));
      if (!source) break;
      const controller = new AbortController();
      this.active.set(source.key, { controller, video: source.video });
      void Promise.resolve().then(() => this.render(source, controller.signal)).then(blob => {
        if (controller.signal.aborted || !this.desired.some(item => item.key === source.key)) return;
        this.urls = new Map(this.urls).set(source.key, URL.createObjectURL(blob));
        this.notify();
      }).catch(() => {
        if (!controller.signal.aborted) this.failed.add(source.key);
      }).finally(() => { this.active.delete(source.key); this.pump(); });
    }
  }

  clear() { this.setWindow([]); }
}

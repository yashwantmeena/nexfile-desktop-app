import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/features/dashboard/media/previewCache.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { PreviewCache } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`);
const tick = () => new Promise(resolve => setImmediate(resolve));
const item = (key, video = false) => ({ key, url: key, video });
const calls = [];
const revoked = [];
const originalRevoke = URL.revokeObjectURL;
URL.revokeObjectURL = url => { revoked.push(url); originalRevoke(url); };
const pending = new Map();
const cache = new PreviewCache((source, signal) => {
  calls.push(source.key);
  return new Promise((resolve, reject) => {
    pending.set(source.key, { resolve: () => resolve(new Blob([source.key])), reject, signal });
    signal.addEventListener("abort", () => reject(new DOMException("Cancelled", "AbortError")), { once: true });
  });
});
try {
  cache.setWindow([item("visible-1"), item("visible-2"), item("next"), item("previous")]);
  await tick();
  assert.deepEqual(calls, ["visible-1", "visible-2"], "visible cards run first, at most two at a time");
  pending.get("visible-1").resolve();
  await tick();
  assert.equal(calls.at(-1), "next");
  const retained = cache.snapshot().get("visible-1");
  cache.setWindow([item("visible-1"), item("new-visible"), item("next")]);
  await tick();
  assert.ok(pending.get("visible-2").signal.aborted, "work outside the new window is cancelled");
  assert.ok(!calls.includes("previous"), "stale queued cards never start");
  assert.equal(cache.snapshot().get("visible-1"), retained, "overlapping previews are reused");
  pending.get("new-visible").reject(new Error("Unsupported file"));
  pending.get("next").resolve();
  await tick();
  assert.equal(cache.snapshot().size, 2);
  cache.setWindow([item("video-1", true), item("video-2", true), item("image")]);
  await tick();
  assert.ok(revoked.includes(retained), "eviction releases blob URLs");
  assert.deepEqual(calls.slice(-2), ["video-1", "image"], "one video decoder maximum, alongside an image worker");
  cache.clear();
  await tick();
  assert.equal(cache.snapshot().size, 0);
  assert.ok(pending.get("video-1").signal.aborted);
  assert.ok(pending.get("image").signal.aborted);

  const immediate = new PreviewCache(async () => new Blob(["preview"]));
  const window = (first, last) => [
    ...Array.from({ length: last - first }, (_, i) => item(String(first + i))),
  ];
  immediate.setWindow(window(10, 30));
  await tick();
  assert.equal(immediate.snapshot().size, 20, "only twenty visible previews are retained");
  const overlap = immediate.snapshot().get("20");
  immediate.setWindow(window(20, 40));
  await tick();
  assert.equal(immediate.snapshot().size, 20, "scrolling does not grow the cache");
  assert.equal(immediate.snapshot().get("20"), overlap);
  assert.ok(!immediate.snapshot().has("19"), "previous offscreen previews are released");
  assert.ok(!immediate.snapshot().has("40"), "next offscreen previews are not loaded");
  immediate.clear();
  assert.equal(immediate.snapshot().size, 0);
  console.log("Preview cache tests passed: priority, concurrency, cancellation, reuse, eviction, video limit, cleanup, visible-only window.");
} finally { cache.clear(); URL.revokeObjectURL = originalRevoke; }

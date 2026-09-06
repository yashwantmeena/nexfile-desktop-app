// This module runs in a worker; originals never become card image sources.
export {};

self.onmessage = async (event: MessageEvent<{ url: string }>) => {
  let bitmap: ImageBitmap | undefined;
  try {
    const response = await fetch(event.data.url, { cache: "no-store" });
    if (!response.ok) throw new Error(`Image read failed (${response.status})`);
    bitmap = await createImageBitmap(await response.blob(), { imageOrientation: "from-image" });
    const scale = Math.min(1, 512 / Math.max(bitmap.width, bitmap.height));
    const canvas = new OffscreenCanvas(Math.max(1, Math.round(bitmap.width * scale)), Math.max(1, Math.round(bitmap.height * scale)));
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Preview canvas unavailable");
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    bitmap.close();
    bitmap = undefined;
    const blob = await canvas.convertToBlob({ type: "image/webp", quality: 0.75 });
    canvas.width = canvas.height = 1;
    self.postMessage({ blob });
  } catch (error) {
    self.postMessage({ error: String(error) });
  } finally {
    bitmap?.close();
  }
};

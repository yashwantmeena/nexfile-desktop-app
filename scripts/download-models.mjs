import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { mkdir, rename, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

const MODEL_REVISION = "6ef1ebc8b0766a7a8d11b146462c99cdf74dd22d";
const MODEL_BASE_URL =
  `https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/${MODEL_REVISION}`;
const MODEL_DIRECTORY = resolve(
  "src-tauri/resources/ai-models/clip-vit-base-patch32",
);

const MODEL_FILES = [
  {
    source: "onnx/vision_model_quantized.onnx",
    destination: "vision_model.onnx",
    sha256: "583fd1110a514667812fee7d684952aaf82a99b959760c8d7dca7e0ab9839299",
  },
  {
    source: "onnx/text_model_quantized.onnx",
    destination: "text_model.onnx",
    sha256: "73baab855d406190da9faa498cfedf65f15cf309f4cc7385b7b032e6d08e5c3a",
  },
  {
    source: "tokenizer.json",
    destination: "tokenizer.json",
    sha256: "f7f3b7af117d467b58374797691a6438d3e6b9e9cef800dfd5dced7f697a90cd",
  },
];

async function sha256(filePath) {
  const hash = createHash("sha256");

  for await (const chunk of createReadStream(filePath)) {
    hash.update(chunk);
  }

  return hash.digest("hex");
}

async function hasExpectedChecksum(filePath, expectedChecksum) {
  try {
    return (await sha256(filePath)) === expectedChecksum;
  } catch (error) {
    if (error?.code === "ENOENT") {
      return false;
    }

    throw error;
  }
}

async function downloadModelFile(modelFile) {
  const destination = resolve(MODEL_DIRECTORY, modelFile.destination);

  if (await hasExpectedChecksum(destination, modelFile.sha256)) {
    console.log(`Using verified ${modelFile.destination}`);
    return;
  }

  await mkdir(dirname(destination), { recursive: true });

  const temporaryDestination = `${destination}.part`;
  await rm(temporaryDestination, { force: true });

  const url = `${MODEL_BASE_URL}/${modelFile.source}`;
  console.log(`Downloading ${modelFile.destination}`);

  const response = await fetch(url);
  if (!response.ok || !response.body) {
    throw new Error(
      `Failed to download ${modelFile.destination}: HTTP ${response.status} ${response.statusText}`,
    );
  }

  try {
    await pipeline(
      Readable.fromWeb(response.body),
      createWriteStream(temporaryDestination),
    );

    const downloadedChecksum = await sha256(temporaryDestination);
    if (downloadedChecksum !== modelFile.sha256) {
      throw new Error(
        `Checksum mismatch for ${modelFile.destination}: expected ${modelFile.sha256}, received ${downloadedChecksum}`,
      );
    }

    await rm(destination, { force: true });
    await rename(temporaryDestination, destination);
    console.log(`Verified ${modelFile.destination}`);
  } catch (error) {
    await rm(temporaryDestination, { force: true });
    throw error;
  }
}

for (const modelFile of MODEL_FILES) {
  await downloadModelFile(modelFile);
}

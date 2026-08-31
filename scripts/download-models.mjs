import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { mkdir, rename, rm } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

const MODELS = [
  {
    name: "CLIP ViT-B/32",
    baseUrl:
      "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/6ef1ebc8b0766a7a8d11b146462c99cdf74dd22d",
    directory: resolve("src-tauri/resources/ai-models/clip-vit-base-patch32"),
    files: [
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
    ],
  },
  {
    name: "Florence-2 base fine-tuned",
    baseUrl:
      "https://huggingface.co/onnx-community/Florence-2-base-ft/resolve/2f19d9e71b5ad76bb878b12496d53fbad7fff1cd",
    directory: resolve("src-tauri/resources/ai-models/florence-2-base-ft"),
    files: [
      {
        source: "onnx/vision_encoder_int8.onnx",
        destination: "onnx/vision_encoder_int8.onnx",
        sha256: "c66e24f41be64047ae24b4413caab9017269b96561c2919f2050940b3fa8f112",
      },
      {
        source: "onnx/embed_tokens_int8.onnx",
        destination: "onnx/embed_tokens_int8.onnx",
        sha256: "6b2258db1c8ee9b160576ccde3cd3814d83a2edaed0dd1c6ca9ff3c38fa62214",
      },
      {
        source: "onnx/encoder_model_int8.onnx",
        destination: "onnx/encoder_model_int8.onnx",
        sha256: "f4ad7a68f1fb875d3bcf735ea14a7021b7ba7e83baf7cf10289881b4ed6d9b85",
      },
      {
        source: "onnx/decoder_model_int8.onnx",
        destination: "onnx/decoder_model_int8.onnx",
        sha256: "c529b26bafce2ee76f886f3a0e374bb646b07a6d8b7640fd8a50d7a48843dd67",
      },
      {
        source: "tokenizer.json",
        destination: "tokenizer.json",
        sha256: "847bbeab6174d66a88898f729d52fa8d355fafe1bea101cf960dd404581df70e",
      },
    ],
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

async function downloadModelFile(model, modelFile) {
  const destination = resolve(model.directory, modelFile.destination);

  if (await hasExpectedChecksum(destination, modelFile.sha256)) {
    console.log(`Using verified ${model.name}: ${modelFile.destination}`);
    return;
  }

  await mkdir(dirname(destination), { recursive: true });

  const temporaryDestination = `${destination}.part`;
  await rm(temporaryDestination, { force: true });

  const url = `${model.baseUrl}/${modelFile.source}`;
  console.log(`Downloading ${model.name}: ${modelFile.destination}`);

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

for (const model of MODELS) {
  for (const modelFile of model.files) {
    await downloadModelFile(model, modelFile);
  }
}

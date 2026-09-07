# Florence-2 base fine-tuned ONNX assets

The generated files in this directory come from the int8 ONNX export in
[`onnx-community/Florence-2-base-ft`](https://huggingface.co/onnx-community/Florence-2-base-ft).
They are used to generate a caption for each image processed by the background
image-processing worker.

Generation uses `decoder_model_int8.onnx` for the first step and
`decoder_model_merged_int8.onnx` for subsequent steps with a KV cache.
The cache belongs to one prompt and is reset between caption, OCR, and detection
tasks. Image features are reused between tasks on the same image. The standalone
`decoder_with_past_model_int8.onnx` export is unsuitable here because its input
embedding sequence length is fixed at 16; the merged export supports one token.
Startup logs report `[Florence-2] KV cache enabled` when the companion is loaded.

The model binaries and tokenizer are not committed to Git. Run
`npm run prepare:models` from the repository root to download them from the
pinned upstream revision and verify their SHA-256 checksums. Tauri runs this
command automatically before development and production builds.

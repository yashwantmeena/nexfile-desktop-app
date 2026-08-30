# CLIP ViT-B/32 ONNX assets

These files come from the quantized ONNX export in
[`Xenova/clip-vit-base-patch32`](https://huggingface.co/Xenova/clip-vit-base-patch32):

- `onnx/vision_model_quantized.onnx` is stored as `vision_model.onnx`.
- `onnx/text_model_quantized.onnx` is stored as `text_model.onnx`.
- `tokenizer.json` keeps its upstream name.

The shorter local names are the convention expected by `ClipModelPaths`.

The generated model files are not committed to Git. Run
`npm run prepare:models` from the repository root to download them from the
pinned upstream revision and verify their SHA-256 checksums. Tauri runs this
command automatically before development and production builds.

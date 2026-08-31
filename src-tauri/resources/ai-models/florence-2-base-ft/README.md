# Florence-2 base fine-tuned ONNX assets

The generated files in this directory come from the int8 ONNX export in
[`onnx-community/Florence-2-base-ft`](https://huggingface.co/onnx-community/Florence-2-base-ft).
They are used to generate a caption for each image processed by the background
image-processing worker.

The model binaries and tokenizer are not committed to Git. Run
`npm run prepare:models` from the repository root to download them from the
pinned upstream revision and verify their SHA-256 checksums. Tauri runs this
command automatically before development and production builds.

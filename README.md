# NexFile Desktop App

Tauri desktop application with a React frontend and a Rust backend.

## Frontend structure

```text
src/
├── app/                 # Application shell, providers, and route selection
├── pages/               # Route-level Home, Search, Storage, and Settings screens
├── features/            # Search, storage, indexing, and AI domain modules
├── components/          # Shared reusable UI and layout components
├── hooks/               # Cross-feature React hooks
├── stores/              # Application and preferences state boundaries
├── services/
│   ├── tauri/           # Typed React-to-Rust command adapters
│   └── api/             # Future HTTP-backed integrations
├── lib/                 # Framework-independent utilities and configuration
├── types/               # Shared cross-feature types
├── assets/              # Bundled images and icons
├── styles/              # Global styles and design variables
└── main.tsx             # React entry point
```

Pages compose features, features own domain behavior, and shared components remain domain-independent. Tauri `invoke` calls are isolated under `src/services/tauri`.

## Commands

```powershell
npm run dev
npm run build
npm run tauri dev
cargo test --manifest-path src-tauri/Cargo.toml
```

On Windows, a clean Rust build also needs CMake, Perl, and NASM 2.16.x on
`PATH` to compile the bundled AVIF decoder. Git for Windows includes Perl.
NASM 3.x is not currently compatible with the pinned libaom release.

## Image processing

Image classification and Florence-2 paragraph-level captioning accept `.avif`, `.avip`, `.bmp`, `.gif`, `.heic`, `.heif`,
`.ico`, `.jpeg`, `.jpg`, `.png`, `.svg`, `.tif`, `.tiff`, and `.webp` files.
The original file stays unchanged. Before model processing, each image is decoded
or rasterized, resized so its longest side is at most 2048 pixels, flattened onto
white when it has transparency, and saved as a temporary quality-90 JPEG. The same
prepared file can be shared by CLIP and Florence-2, and is deleted automatically
when processing finishes or fails.

The primary CLIP decision separates images with meaningful readable text from visual images.
Text images are sent directly to Florence-2 OCR with regions. Visual images receive the
existing secondary and tertiary classification plus a paragraph-level caption. The resulting
`<image-name>.<extension>.json` sidecar contains either `ocr` or `caption`, CLIP-ranked search
`tags`, and the applicable CLIP `classification`. Run `npm run prepare:models` to download
the pinned, checksum-verified CLIP and Florence-2 ONNX assets. Model binaries and
tokenizers are generated dependencies and remain excluded from Git.

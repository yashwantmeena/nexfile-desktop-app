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

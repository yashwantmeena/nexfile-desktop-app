# Backend tests

Run from the project root:

```powershell
npm test
```

This runs the unit suite, integration suite, and documentation tests, continuing
to the remaining suites even if one fails. Unit-test
registration is centralized in `tests/unit/mod.rs`; integration-test registration
starts in `tests/integration.rs`. Production service, repository, mapper, utility,
and worker files do not register tests. The library has one test-only declaration
that loads the centralized unit suite.

To also run every test marked ignored:

```powershell
$env:NEXFILE_CALIBRATION_DIR = 'C:\path\to\calibration-images'
npm run test:all
```

The ignored tests require the bundled CLIP and Florence-2 model resources, and
the calibration test requires a nonempty directory of supported images. These
tests perform real model inference and can take substantially longer.

Cargo equivalents (also usable without npm):

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast
cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast -- --include-ignored
```

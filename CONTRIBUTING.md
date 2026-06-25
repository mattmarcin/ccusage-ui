# Contributing

Thanks for considering a contribution to ccusage UI.

## Local Setup

1. Install Rust, Bun, and the platform prerequisites for Tauri v2.
2. Install `ccusage` separately. The app invokes your existing CLI installation and does not bundle it.
3. Run:

```powershell
bun install
bun run tauri dev
```

## Checks

Before opening a pull request, run:

```powershell
bun run check
bun run build
cd src-tauri
cargo test
cargo fmt -- --check
```

## Privacy

Avoid sharing private usage data. Tests should use fixtures or fake `ccusage` output rather than real local CLI logs.
# ccusage UI

A local desktop dashboard for [`ccusage`](https://github.com/ccusage/ccusage) token and cost reports.

ccusage UI is a Tauri v2 app with a Rust backend and React/TypeScript frontend. It runs your existing `ccusage` CLI installation, reads the JSON report output, and displays usage by model, token type, and estimated cost.

## Features

- Local desktop UI for `ccusage daily --json` reports.
- Usage grouped by model.
- Cost, total, input, output, cache, and reasoning token summaries.
- Daily trend chart.
- Light and dark mode.
- Configurable `ccusage` path, Claude config directories, timezone, cache TTL, and offline pricing mode.
- Rust-only process execution; the frontend does not get shell access.

## Privacy

This app does not parse Claude, Codex, or other agent log directories directly. It invokes `ccusage` and consumes the JSON that `ccusage` returns.

The displayed costs come from `ccusage`; ccusage UI does not calculate model prices itself. When offline pricing is enabled, `ccusage` uses its cached pricing data where supported.

## Requirements

- Windows, macOS, or Linux with Tauri v2 prerequisites.
- Rust 1.77 or newer.
- Bun.
- `ccusage` installed and available on PATH, or configured in the app settings.

On Windows, the app also checks common install locations such as `~\.bun\bin\ccusage.exe`.

If you use multiple Claude Code accounts with separate config directories, set `Claude config dirs` in the app settings to a comma-separated list such as:

```text
C:\Users\m\.claude,C:\Users\m\.claude-alt
```

The app passes that value to `ccusage` as `CLAUDE_CONFIG_DIR`.

## Development

```powershell
bun install
bun run tauri dev
```

## Checks

```powershell
bun run check
bun run build
cd src-tauri
cargo test
cargo fmt -- --check
```

## Build

```powershell
bun run tauri build
```

Release artifacts are written under `src-tauri/target/release/bundle/`.

## License

MIT. See [LICENSE](LICENSE).
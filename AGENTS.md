# traversal

Single-crate Rust CLI binary (edition 2024). Discovers `[traverse-tgt: <tag>]` and `[traverse-lnk: <tag>]` annotations in files.

## Build & run

```bash
cargo build --release
cargo run --release [path ...]
```

No tests yet. No CI. No formatter/lint config.

## Key structure

| Path | Role |
|------|------|
| `crates/traversal/main.rs` | Entire binary. CLI via `clap::Parser`, parallel walk via `ignore::WalkBuilder`, regex search via the `grep` crate. |
| `test_workspace/` | Small hand-curated fixture with a few tags. |
| `generate_large_workspace.py` | Creates large example workspaces. Run with `--num-files`, `--num-tags`, `--output-dir` args. |

## What it does

1. Walks paths (respects `.gitignore` via `ignore::WalkBuilder`).
2. Regex matches `[traverse-tgt: <tag>]` (capture group 1) and `[traverse-lnk: <tag>]` (capture group 2).
3. Prints all targets and links with `path:line: tag`.

## Gotchas

- Regex is compile-time const via `const_format::formatcp` — edit `TARGET_TAG_REGEX` or `LINK_TAG_REGEX` in `main.rs:17-18` to change tag syntax.
- Uses `grep::searcher::BinaryDetection::quit(b'\x00')` — binary files that contain null bytes will be silently skipped.
- The single `[[bin]]` in `Cargo.toml` has `bench = false`, so `cargo bench` is a no-op.

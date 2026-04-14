# Changelog

All notable changes to this project will be documented in this file.

## [0.8.0] - 2026-04-14

### Breaking Changes
- Building from source now requires a Rust toolchain supporting edition 2024. Update your stable toolchain if needed.
- Workspace symbol support requires the new `symbols:` block in `pickls.yaml` with `source: universal-ctags` set explicitly. Omit the block entirely to disable workspace symbols; specifying the block without `source` is a configuration error.
- Language-level `root_markers` is now inherited by all linters and formatters under that language by default. Previously, tools computed their root independently. To preserve the old behavior for a specific tool, set `root_markers: []` on that linter/formatter.

### Added
- Inline Assist code action powered by LLMs. Supports OpenAI and Ollama providers configured via the new top-level `ai:` block in `pickls.yaml` (`ai.inline_assistants`, `ai.system_prompt`, `ai.inline_assistant_prompt_template`, `ai.openai.api_key_cmd`, `ai.ollama.api_address`). Multiple assistants can be queried in parallel.
- Optional inclusion of workspace files in the inline-assist prompt via `ai.inline_assistant_include_workspace_files` (uses `git ls-files`, disabled by default).
- Workspace symbol support (`workspaceSymbolProvider`) by proxying to [universal-ctags](https://ctags.io/). Configurable via `symbols.source: universal-ctags`, with tunable `symbols.ctags_timeout_ms` (default 500ms).
- Per-linter and per-formatter `root_markers` fields, overriding the language-level default.
- Progress notifications for long-running inline-assist jobs.
- Formatter config gains a `stderr_indicates_error` flag: when true, any output on stderr aborts the format run.
- README now documents Neovim setup, Zed setup, primary LSP server capabilities, and troubleshooting.

### Changed
- Migrated to Rust edition 2024.
- Error handling migrated to `anyhow`; error messages surfaced to the client are more descriptive.
- Runtime reworked to a single-threaded model with a background worker for LLM and linter/formatter calls, fixing ordering issues around concurrent diagnostic updates.
- LSP transport now uses the `lsp-types` crate directly instead of `tower-lsp`. No user-visible behavior change for standard LSP clients.
- Dependencies bulk-upgraded to latest: tokio 1.52, reqwest 0.13, nix 0.31, sysinfo 0.38, thiserror 2.x, xdg 3.0, schemars 1.2, handlebars 6.4, libc 0.2.185, regex 1.12, serde 1.0.228, serde_json 1.0.149, anyhow 1.0.102, crossbeam-channel 0.5.15, futures 0.3.32, log 0.4.29.

### Fixed
- Zombie child-process leak: linter and formatter subprocesses are now reaped correctly after completion or termination.
- Linting no longer fails when the target file disappears mid-run; the missing-file case is handled gracefully.
- ctags symbol generation now uses a timeout (`ctags_timeout_ms`) rather than a hard-coded max symbol count, so large workspaces don't silently truncate.

### Removed
- Dropped the `toml` and `allms` dependencies (dead code paths); config remains YAML-only, and LLM calls go through `reqwest` directly.
- Removed the internal `error` module and custom `Error` enum in favor of `anyhow::Error`.

# Changelog

All notable changes to the Path Server will be documented in this file.

## [Unreleased]
### Added
- **VS Code**: Added remote window (e.g. Remote SSH, Dev Container) support.

### Fixed
- **Core**: Fixed error when open an untitled document.
- **Core**: Fixed occasional crashes with error "receiver already dropped" when initializing the server.

### Improved
- **Core**: Replaced the long-unmaintained `tower-lsp` dependency with a maintained fork `tower-lsp-server`.

## [0.5.2] 2026-03-14
### Fixed
- **Core**: Fixed an issue where path in markdown quote and html block may not extract correctly.
- **Core**: Fixed an issue where raw string in rust may not extract correctly.
- **Core**: Added deduplication into parser to avoid unnecessary resolving performance costs.

### Added
- **Core**: Added tree-sitter HTML, C, and C++ support.

## [0.5.1] - 2026-03-12
### Added
- **Core**: Support to provide hover information on paths.
- **Core**: Added tree-sitter markdown support.

### Changed
- **Core**: Change default build to single thread to reduce resource consumption. You can enable multi-threading by building with the `multi-thread` feature.

### Improved
- **Core**: Enhance completion UX：
    - Added descriptions to completion items to show which `base_paths` they originated from.
    - Improved suggestion ordering: entries now respect the order defined in the `base_path` configuration.
    - Support filtering completion items based on `base_path` categories.
- Add demo gif in README and reorganize README for more information.

## [0.5.0] - 2026-03-11
### Added
- **VS Code**: Included `CHANGELOG.md` inside package.
- **Core**: Added config entry: `path-server.highlight.highlightDirectory` to control whether to highlight directory paths. 

### Fixed
- **Core**: Resolved an issue where directory highlighting didn't work as expected.
- **Core**: Fixed a regression in "Go to Definition" functionality.

### Improved
- **Core**: Enhanced build profile to reduce executable size.
- **Core**: Improved log messages with better readability.
- **Core**: Added configuration cache to avoid frequent IPC polling.
- **Core**: Support partial config merging, now user can override single setting rather than provide the entire configuration.
- **Core**: Significant performance improvements by introducing a verified path tokens cache.

## [0.4.0] - 2026-03-10
### Added
- **Core**: Support automatically triggering next completion after selecting a completion.
- **Core**: Added config entry: `path-server.completion.triggerNextCompletion`.
- **Core**: Added **Document Links** provider support in supported editors. (zed's api does not supported it for now)
    - Automatically detects and underlines valid file paths in the editor.
    - Making paths clickable and allowing users to jump directly to the target file.
    - *Note: Currently not supported in **Zed** as it does not yet implement the LSP Document Link feature.*
- **Core**: Added **Go to Definition** provider support.
    - Enables standard "Go to Definition" functionality for string-based file paths.
    - Users can now use editor shortcuts (e.g., `Cmd/Ctrl + Click` or `F12`) to instantly open the file referenced by a path.
- **Core**: Added config entry: `path-server.highlight.enable` to control highlighting of file paths in editor.
- **VS Code**: Add command `Path Server: Restart Server` to restart the Path Server.

### Changed
- **Core**: Promote config entry from `path-server.completion.basePath` to `path-server.basePath`. Now this configuration is used for both completion and other features. (*Note: old config is no longer supported*)

## [0.3.0] 2026-03-06
### Fixed
- **Zed**: Fix version-compatibility check — correctly parse the major version so `v10.x.x` is not mistaken for `v1.x.x`.

### Added
- **Core**: Add version log during initialization.
- **Core**: Support custom configuration.
    - `path-server.completion.maxResults`: Max results shown in completion.
    - `path-server.completion.showHiddenFiles`: Whether to show hidden files in completion.
    - `path-server.completion.exclude`: List of paths to exclude from completion. Supports glob patterns.
    - `path-server.completion.basePath`:  Base paths for relative path completion.
- **Zed**: Support read custom configuration from `settings.json` > `lsp.path-server.settings`.
- **VS Code**: Support reading custom configuration from settings panel `path-server`.
- **VS Code**: Add command `Path Server: Open Configuration` to open Path Server configuration.
- Add detailed description of configuration usage and configuration options.

## [0.2.0] - 2026-03-04
### Added
- **VS Code**: Initial release of VS Code Extension with Path Server support.
    - Self-contained extension providing Path Server integration.
    - Basic path auto-completion for relative and absolute paths across programming languages from Path Server.
    - Log redirection to the "Output" panel.
- **Zed**: Initial release of Zed Extension with Path Server support.
    - Auto-download and automatic upgrades of the Path Server executable.
    - Basic path auto-completion for relative and absolute paths across programming languages from Path Server.

### Changed
- **Core**: Refactored completion logic to improve maintainability.
- Repository reorganized into a monorepo (consolidated `path-server-zed` and `path-server-vscode` into `path-server`).
- Change release assets naming style from `-` to `_` for readability.
- Improved README readability.

## [0.1.0] - 2026-03-03
Initial release of **Path Server**.

### Added
- **Core**: Support path completion, both relative and absolute paths.
- **Core**: Support relative path based on workspace root or current document.
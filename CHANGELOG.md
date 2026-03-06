# Changelog

All notable changes to the Path Server will be documented in this file.

## [Unreleased]
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
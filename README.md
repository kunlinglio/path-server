# Path Server

Path Server is a fast and lightweight Language Server Protocol (LSP) implementation designed to provide path completion and navigation, offering a **Path Intellisense** experience.

## Features
- **Path Completion**: Provides real-time suggestions for both relative and absolute paths.
- **Path highlight and jump**: Automatically detects and underlines valid file paths in the editor, making them clickable for direct navigation.
- **Fast and Lightweight**: Native-level response speed. Consumes only ~5MB memory with very low CPU usage.
- **Language Compatibility**: Supports all text files, regardless of the programming language.
- **Cross IDEs**: Works seamlessly with any editor that supports the Language Server Protocol (e.g., VS Code, Zed, Neovim).

## Support Platforms

| Platform | x86_64 | Aarch64 |
| :--- | :--- | :--- |
| **Windows** | Build & Test | Build Only |
| **Linux** | Build & Test | Build Only |
| **macOS** | Build Only | Build & Test |

## Usage
You can use it by installing specified extension for your editor.

### VS Code
1. Search for `Path Server` in the VS Code extensions marketplace or download `.vsix` file from [Releases](https://github.com/kunlinglio/path-server/releases) and install it manually. 
2. Start typing a path prefix like `./`, `/` or `C:` in any file to trigger suggestions.
3. Open the settings and search for `path-server` or use the command `Path Server: Open Configuration` to customize the configuration options.

*You can toggle Output panel and choose `Path Server Language Server` to view detailed logs.*

### Zed
1. Search for `Path Server` in the Zed extensions catalog. 
2. Start typing a path prefix like `./`, `/` or `C:` in any file to trigger suggestions.
3. Toggle command panel and input `zed: open settings file` to edit settings. You can add configuration options there. For example:
```json
{
  // ...other configs...
  "lsp": {
    "path-server": {
      "settings": {
        "completion": {
          "maxResults": 5,
          "showHiddenFiles": false,
          "exclude": ["**/node_modules/**", "**/.git/**"],
          "basePath": ["${workspaceFolder}", "${root}"]
        }
      }
    }
  }
}
```

### Configuration
You can customize Path Server's behavior via your editor's settings.

| Property | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `path-server.basePath` | Array | `["${workspaceFolder}", "${document}"]` | Base paths for relative path completion highlight and jump. You can use `${workspaceFolder}`, `${document}`, and `${userHome}` as placeholders. |
| `path-server.completion.maxResults` | Number | `0` | Max results shown in completion. `0` indicates no limit. |
| `path-server.completion.showHiddenFiles` | Boolean | `true` | Whether to show hidden files in completion. |
| `path-server.completion.exclude` | Array | `["**/node_modules", "**/.git", "**/.DS_Store"]` | List of paths to exclude from completion. Supports glob patterns. |
| `path-server.completion.triggerNextCompletion` | Boolean | `true` | Whether to automatically trigger the next completion after selecting a path. |
| `path-server.highlight.enable` | Boolean | `true` | Whether to highlight paths in the editor with underscore. |

## Resources
- [GitHub Repository](https://github.com/kunlinglio/path-server)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=LKL.path-server)

## Development
### File Structure
The **Path Server** project is organized in mono-repository structure with core LSP server implementation and extensions for different editors.

- The core LSP server implementation is located in the repository root.
- The **Zed Extension** is located in `./extensions/zed`.
- The **VS Code** is located in `./extensions/vscode`.

### Open with multi-root workspace
If you are using VS Code, you can open this repository with multiple root folders by open `.vscode/path-server.code-workspace` file.

### Develop: Path Server binary
#### Build
```shell
cargo build --release
```

#### Run test
```shell
cargo test
```

#### Run check
```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

#### Try fix warnings
```bash
cargo fix --allow-dirty
cargo clippy --fix --allow-dirty
```

### Develop: Zed extension
#### Switch to directory
```bash
cd extensions/zed
```

#### Install as dev extension
1. Open Zed editor.
2. Toggle the Command Palette (`Ctrl + Shift + P` on Windows/Linux, `Cmd + Shift + P` on macOS).
3. Run `zed: install dev extension`.
4. Select this path-server/extensions/zed folder.

### Develop: VS Code extension
#### Switch to directory
```bash
cd extensions/zed
```

#### Run dev extension
1. Open workspace with VS Code (Use VS Code to open `.vscode/path-server.code-workspace`)
2. Toggle the Command Palette (`Ctrl + Shift + P` on Windows/Linux, `Cmd + Shift + P` on macOS).
3. Run `Debug: Select and Start Debugging`.
4. Select `Run Extension (VS Code Extension)`

## TODO
- [x] Support relative and absolute path completion.
- [x] Support customizable configurations.
- [x] Automatically trigger next completion.
- [ ] Implement "Go to Definition" for file paths.
- [ ] Support path highlight.
- [ ] Support remote URL.
- [ ] **Zed**: Support all language by use "wildcard" in extension.toml (Waiting for Zed extension api support)

## License
Distributed under the terms of the Apache 2.0 license.

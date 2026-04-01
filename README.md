# Path Server
Path Server is a fast and lightweight Language Server Protocol (LSP) implementation designed to provide path completion, highlight and navigation, offering a **Path Intellisense** experience across different editors.

<div align="center">
    <img src="./assets/demo-vscode.gif" alt="demo" style="width: 600px">
</div>

<details>
<summary><b>Also available On Zed</b></summary>
    <div align="center">
        <img src="./assets/demo-zed.gif" alt="demo" style="width: 600px">
    </div>
</details>

## Features
- **Path Completion**: Provides real-time suggestions for both relative and absolute paths.
- **Path highlight and jump**: Automatically detects and underlines valid file paths in the editor, making them clickable for direct navigation.
- **Fast and Lightweight**: Native-level response speed. Consumes only ~10MB memory with very low CPU usage.
- **Language Compatibility**: Supports all text files, regardless of programming languages.
- **Cross IDEs**: Works seamlessly with any editors that support the Language Server Protocol (e.g., VS Code, Zed, Neovim).

## Support Platforms
| Platform | x86_64 | Aarch64 |
| :--- | :--- | :--- |
| **Windows** | Build & Test | Build Only |
| **Linux** | Build & Test | Build Only |
| **macOS** | Build Only | Build & Test |

## Usage
You can use Path Server by installing the extension for your editor, or by building it from source.

After installing, start typing a path prefix like `./`, `/` or `C:\` in any file to trigger path suggestions.

### VS Code
1. Search for `Path Server` in the [VS Code extensions marketplace](https://marketplace.visualstudio.com/items?itemName=LKL.path-server), or download the `.vsix` file from [releases](https://github.com/kunlinglio/path-server/releases) and install it manually.
2. Open Settings and search for `path-server`, or run the command `Path Server: Open Configuration` to customize options.
3. Toggle the Output panel and select `Path Server Language Server` to view detailed logs.

### Zed
1. Search for `Path Server` in the Zed extensions catalog.
2. Run `zed: open settings file` from the command palette to edit settings. Example:

```json
{
  "lsp": {
    "path-server": {
      "settings": {
        "basePath": ["${workspaceFolder}", "${document}"],
        "completion": {
          "triggerNextCompletion": true
        },
        "highlight": {
          "enable": true,
          "highlightDirectory": true
        }
      }
    }
  }
}
```

> **Note**: Document Links (path underline highlight) is not yet supported in Zed as it does not implement the LSP Document Link feature.

### Build from Source
If you prefer to build the binary yourself, you'll need [Rust](https://rustup.rs/) installed.

#### Standard build
```shell
cargo build --release
```

#### Multi-threaded build
Path Server defaults to single-threaded mode for minimal resource usage. Enable multi-threading with the `multi-thread` feature flag:
```shell
cargo build --release --features multi-thread
```

#### Package VS Code Extension (`.vsix`)
```shell
cd extensions/vscode
npm install
npm run build
```

The packaged `.vsix` file will be output to the `dist/` directory. You can install it manually via:
```shell
code --install-extension path-server_vscode_*.vsix
```

> **Note**: Zed does not support package extension manually for now.

### Configuration
You can customize Path Server's behavior via your editor's settings.

| Property | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `path-server.basePath` | Array | `[ "${document}", "${workspaceFolder}"]` | Base paths for relative path completion, highlight and jump. You can use `${workspaceFolder}`, `${document}`, and `${userHome}` as placeholders. The order determines the priority in suggestions.|
| `path-server.completion.maxResults` | Number | `0` | Max results shown in completion. `0` indicates no limit. |
| `path-server.completion.showHiddenFiles` | Boolean | `true` | Whether to show hidden files in completion. |
| `path-server.completion.exclude` | Array | `["**/node_modules", "**/.git", "**/.DS_Store"]` | List of paths to exclude from completion. Supports glob patterns. |
| `path-server.completion.triggerNextCompletion` | Boolean | `true` | Whether to automatically trigger the next completion after selecting a path. |
| `path-server.highlight.enable` | Boolean | `true` | Whether to highlight paths in the editor with underlines. |
| `path-server.highlight.highlightDirectory` | Boolean | `true` | Whether to highlight directory paths. (Jump behavior may vary by editor/OS).|

## Resources
- [GitHub Repository](https://github.com/kunlinglio/path-server)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=LKL.path-server)
- [Path Server Icon](https://pictogrammers.com/library/mdi/icon/slash-forward-box/)

## TODO
- [x] Support relative and absolute path completion.
- [x] Support customizable configurations.
- [x] Automatically trigger next completion.
- [x] Implement "Go to Definition" for file paths.
- [x] Support path highlight.
- [x] Support remote window.
- [x] Improve path extraction precision.
- [ ] **Zed**: Support all language by use "wildcard" in extension.toml (Waiting for Zed extension api support)

## Development
### Recommended Workflow
If you use VS Code, you can open this repository with the provided workspace file:
```bash
code .vscode/path-server.code-workspace
```

This workspace is pre-configured with multi-root folders and debug task settings.

### File Structure
The **Path Server** project is organized in mono-repository structure with core LSP server implementation and extensions for different editors.

- The core LSP server implementation and tests are located in the repository root.
- The **Zed Extension** is located in `./extensions/zed`.
- The **VS Code** is located in `./extensions/vscode`.

### Core: LSP Server
The core logic is written in Rust (`./src/main.rs`).

- Build: 
    ```bash
    cargo build
    ```
- Test: 
    ```bash
    cargo test
    ```
- Lint: 
    ```bash
    cargo fmt --all -- --check
    cargo clippy -- -D warnings
    ```
- Format:
    ```bash
    cargo fix --allow-dirty
    cargo clippy --fix --allow-dirty
    ```

### Extension: Zed
Zed extensions are compiled to WASM.

1. Install Dev Extension:  
    Open Zed and run command zed: install dev extension.
    Select the zed folder.
2. View Logs:   
    Open logs to debug LSP communication.

### Extension: VS Code
The VS Code extension acts as a client that launches the Rust binary.

1. Setup:
    ```bash
    npm install
    ```
2. Debug:  
    Press `F5` or `run Debug`: Select and Start Debugging -> Run Extension.

    > This will build the language server automatically and launch a "Extension Development Host" window.

3. View Logs:
    The server logs will be redirect to `Output panel` -> `Path Server Language Server`

## License
Distributed under the terms of the Apache 2.0 license.

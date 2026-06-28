# Path Server
Path Server is an extremely fast and lightweight Language Server Protocol (LSP) server written in Rust, acting as a lightweight and cross-editor replacement for [Path Intellisense](https://github.com/ChristianKohler/PathIntellisense).

**Path Server** automatically **highlights valid file paths** in the editor, **provides completion suggestions** for both relative and absolute paths, and **allows users to jump to the paths** in the editor.

It's currently compatible with **VS Code** and **Zed** officially, and can be easily used or ported to any editors that implement the LSP protocol (e.g., Neovim). We are welcome to [issues](https://github.com/kunlinglio/path-server/issues) or [contributions](https://github.com/kunlinglio/path-server/pulls) about your favorite editors.

<div align="center">
    <img src="./assets/demo-vscode.gif" alt="demo" style="width: 600px">
</div>

<details>
<summary><b>Same experience on Zed</b></summary>
    <div align="center">
        <img src="./assets/demo-zed.gif" alt="demo" style="width: 600px">
    </div>
</details>

## Features
- **Path Completion**: Provides real-time suggestions for both relative and absolute paths.
- **Path highlight and jump**: Automatically detects and underlines valid file paths in the editor, making them clickable for direct navigation.
- **Highly Customizable**: Offers various configuration options to tailor the behavior to your needs, such as setting base paths for completion, excluding paths with specific patterns, and [more](#configuration).
- **Fast and Lightweight**: Native-level response speed. Consumes only ~10MB memory with very low CPU usage.
- **Language Compatibility**: Supports all text files, regardless of programming languages.
- **Cross IDEs**: Works seamlessly with any editors that support the Language Server Protocol (e.g., VS Code, Zed, Neovim).

## Usage
### Visual Studio Code
#### Installation via VS Code marketplace (Recommended)
The official `Path Server` extension is available in [VS Code extensions marketplace](https://marketplace.visualstudio.com/items?itemName=LKL.path-server). You can search `Path Server` and install it in the VS Code extensions page.

#### Manual Installation
If you are using an open source version of VSCode, you might need to install the extension manually.
1. Navigate to the latest [release](https://github.com/kunlinglio/path-server/releases) and download the `.vsix` file compatible with your system.
2. Copy the file to your `.vscode/extensions` directory.
3. Install via the command line `code --install-extension /path/to/path-server_vscode_*.vsix`

#### Build from source
If you prefer to build the binary yourself, you'll need [Rust](https://rustup.rs/) installed.

1. Build the Path Server binary.
    ```shell
    cargo build --release
    ```
    Path Server defaults to single-threaded mode for minimal resource usage. If you prefer to build with multi-threading support, enable multi-threading with the `multi-thread` feature flag:
    ```shell
    cargo build --release --features multi-thread
    ```
2. Package VS Code Extension (`.vsix`).
    ```shell
    cd extensions/vscode
    npm install
    npm run build
    ```
3. Install `.vsix` file manually.
    The packaged `.vsix` file will be output to the `dist/` directory. You can install it manually via:
    ```shell
    code --install-extension path-server_vscode_*.vsix
    ```

#### Commands
You can call Path Server commands via the Command Palette (`Cmd/Ctrl + Shift + P`):
- `Path Server: Restart Server`: Restart the Path Server language server.
- `Path Server: Open Configuration`: Open the Path Server **configuration** in a new tab.

### Zed
Search for `Path Server` in the Zed extensions catalog and click install.

> **Note**: Zed does not support package extension manually for now.

> **Note**: Document Links (path underline highlight) is not yet supported in Zed as it does not implement the LSP Document Link feature.

### Other Editors (Helix, Neovim, etc.)
For other editors that support LSP, Path Server should be compatible as well. You can follow the instructions below to get started:

1. Install Path Server binary via `cargo install path-server`.
    > The default version is single-threaded for minimal resource usage. If you prefer with multi-threading support, install by `cargo install path-server --features multi-thread`.
2. Configure your editor to start the Path Server language server with the command `path-server` and set the communication to use STDIN/STDOUT.

*If there is any issue with the compatibility, please feel free to open an issue or contribute a PR to fix it.*

## Configuration
Path Server support custom configuration via LSP workspace configuration. You can customize Path Server's behavior through your editor.

| Property | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `path-server.basePath` | Array | `[ "${document}", "${workspaceFolder}"]` | Base paths for relative path completion, highlight and jump. You can use `${workspaceFolder}`, `${document}`, and `${userHome}` as placeholders. The order determines the priority in suggestions.|
| `path-server.completion.maxResults` | Number | `0` | Max results shown in completion. `0` indicates no limit. |
| `path-server.completion.showHiddenFiles` | Boolean | `true` | Whether to show hidden files in completion. |
| `path-server.completion.exclude` | Array | `["**/node_modules", "**/.git", "**/.DS_Store"]` | List of paths to exclude from completion. Supports glob patterns. |
| `path-server.completion.triggerNextCompletion` | Boolean | `true` | Whether to automatically trigger the next completion after selecting a path. |
| `path-server.highlight.enable` | Boolean | `true` | Whether to highlight paths in the editor with underlines. |
| `path-server.highlight.highlightDirectory` | Boolean | `true` | Whether to highlight directory paths. (Jump behavior may vary by editor/OS).|

### Visual Studio Code
Open Settings and search for `path-server`, or run the command `Path Server: Open Configuration` to open customizable options in a new tab.

### Zed
Run `zed: open settings file` from the command palette to edit user settings json file. And append path server settings below it. For example:

```json
{
  "lsp": {
    "path-server-lsp": {
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

## Support Platforms
| Platform | x86_64 | Aarch64 |
| :--- | :--- | :--- |
| **Windows** | Build & Test | Build Only |
| **Linux** | Build & Test | Build Only |
| **macOS** | Build Only | Build & Test |

## References
- [GitHub Repository](https://github.com/kunlinglio/path-server)
- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=LKL.path-server)
- [Download VSIX](https://github.com/kunlinglio/path-server/releases/latest)
- [Zed Extension](https://zed.dev/extensions/path-server-lsp)
- [Path Server Icon](https://pictogrammers.com/library/mdi/icon/slash-forward-box/)
- [Path Server icon color](https://code.visualstudio.com/brand)
- [Crates.io](https://crates.io/crates/path-server)

## TODO
- [x] Support relative and absolute path completion.
- [x] Support customizable configurations.
- [x] Automatically trigger next completion.
- [x] Implement "Go to Definition" for file paths.
- [x] Support path highlight.
- [x] Support remote window.
- [x] Improve path extraction precision.
- [x] Extract paths from comments that use markdown syntax.

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
- Audit dependencies:
    ```bash
    cargo audit
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

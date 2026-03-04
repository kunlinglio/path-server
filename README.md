# Path Server

Path Server is a fast and lightweight Language Server Protocol (LSP) implementation designed to provide path completion and navigation, offering a **Path Intellisense** experience.

## Features
- **Path Completion**: Provides real-time suggestions for both relative and absolute paths.
- **Fast and Lightweight**: Native-level response speed and consume only ~10MB memory with very low cpu usage.
- **Language Compatibility**: Support all text files discarding programming languages.
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
Search for `Path Server` in the VS Code extensions marketplace or download `.vsix` file and install it manually. Start typing a path prefix like `./`, `/` or `C:` in any file to trigger suggestions.

You can toggle Output panel and choose `Path Server Language Server` to view detailed logs.

### Zed
Search for `Path Server` in the Zed extensions catalog. Start typing a path prefix like `./`, `/` or `C:` in any file to trigger suggestions.

## Resources
- [GitHub Repository](https://github.com/kunlinglio/path-server)

## Development
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
- [ ] Implement "Go to Definition" for file paths.
- [ ] Support path highlight.
- [ ] Support remote URL.

## License
Distributed under the terms of the Apache 2.0 license.

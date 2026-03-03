# Path Server Extension for Zed

Path Server is a high-performance extension for Zed that provides fast and lightweight path autocompletion in any language, offering a **Path Intellisense** like experience.

It is the extension for the [Path Server](https://github.com/kunlinglio/path-server) project, a specialized LSP implementation for file path navigation.

## Features
- **Path Completion**: Real-time suggestions for both relative and absolute paths.
- **Fast and Lightweight**: Native-level response speed and consume only ~10MB memory with very low cpu usage.
- **Language Compatibility**: Support multiple programming languages.
- **Auto download & Auto upgrade**: Automatically downloads and manages the Path Server binary for your platform.

For more information, please refer to [Path Server Repository](https://github.com/kunlinglio/path-server)

## Usage
Search for `Path Server` in the Zed extensions catalog or install it via the "zed: install dev extension" command for local development. Start typing a path prefix like `./` or `/` in any language to trigger suggestions.

## Resources
- [GitHub Repository](https://github.com/kunlinglio/path-server-zed)
- [Path Server](https://github.com/kunlinglio/path-server)

## Development
### Install dev extension
To install the extension locally for development:
1. Open the Command Palette (`Cmd+Shift+P` on macOS).
2. Run `zed: install dev extension`.
3. Select this project folder.

## License
Distributed under the terms of the Apache 2.0 license.

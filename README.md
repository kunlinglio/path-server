# Path Server

Path Server is a fast and lightweight Language Server Protocol (LSP) implementation designed to provide path completion and navigation. 

## Features
- **Path Completion**: Provides real-time suggestions for both relative and absolute paths.
- **Light Weight**: Consume only 10MB memory and very low cpu usage.
- **Cross IDEs**: Works seamlessly with any editor that supports the Language Server Protocol (e.g., VS Code, Zed, Neovim).

## Support Platforms

| Platform | x86_64 | Aarch64 |
| :--- | :--- | :--- |
| **Windows** | Build & Test | Build Only |
| **Linux** | Build & Test | Build Only |
| **macOS** | Build Only | Build & Test |

## Usage
Typically, you don't need to run Path Server manually. It is intended to be used as a backend for editor extensions.

- **VS Code**: To be supported.
- **Zed**: Refer to the [Path Server zed extension](https://github.com/KunlingLio/path-server-zed) repository for integration details.

## Resources
- [GitHub Repository](https://github.com/KunlingLio/path-server)
- [Zed Extension Integration](https://github.com/KunlingLio/path-server-zed)

## Development
### Build
```shell
cargo build --release
```

### Run test
```shell
cargo test
```

### Run check
```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
```

### Try fix
```bash
cargo fix --allow-dirty
cargo clippy --fix --allow-dirty
```

## TODO
- [x] Support relative and absolute path completion.
- [ ] Implement "Go to Definition" for file paths.
- [ ] Support remote URL.

## License
This project is licensed under the Apache License 2.0.

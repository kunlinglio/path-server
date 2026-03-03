# Path Server
A fast and lightweight language server for path completion, go to definition and so on.

## Support platforms
### Build and test on
- Windows x86_64
- Linux x86_64
- MacOS Aarch64

### Only Build on
- Windows Aarch64
- Linux Aarch64
- Macos X86_64

## Resources
- [zed extension](https://github.com/KunlingLio/path-server-zed)

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
- [ ] Support remote url.


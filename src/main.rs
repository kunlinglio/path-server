use path_server::PathServer;

#[cfg(feature = "multi-thread")]
fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(inner());
}

#[cfg(not(feature = "multi-thread"))]
fn main() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(4)
        .build()
        .unwrap();

    rt.block_on(inner());
}

async fn inner() {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = tower_lsp_server::LspService::new(PathServer::new);
    tower_lsp_server::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}

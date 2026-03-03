use path_server::PathServer;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = tower_lsp::LspService::new(PathServer::new);
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}

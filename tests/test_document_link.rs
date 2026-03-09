mod utils;
use tower_lsp::LanguageServer;
use tower_lsp::lsp_types::*;
use utils::*;

#[tokio::test]
async fn test_document_link_integration() {
    let harness = TestHarness::new().await;

    harness.create_file("data/linked.txt");
    harness.create_file("src/main.rs");

    let content = "let s = \"../data/linked.txt\";";
    let uri = harness.open_doc("src/main.rs", content).await;

    let params = DocumentLinkParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let res = harness.get_server().document_link(params).await.unwrap();
    let links = res.unwrap_or_default();

    assert!(!links.is_empty(), "Expected at least one document link");
    let target_url = links[0].target.as_ref().expect("expected target url");

    let expected = tokio::fs::canonicalize(harness.root_path().join("data/linked.txt"))
        .await
        .unwrap();
    assert_eq!(
        tokio::fs::canonicalize(&target_url.to_file_path().unwrap())
            .await
            .unwrap(),
        expected
    );
}

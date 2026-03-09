mod utils;
use tower_lsp::LanguageServer;
use tower_lsp::lsp_types::*;
use utils::*;

#[tokio::test]
async fn test_goto_definition_integration() {
    let harness = TestHarness::new().await;

    harness.create_file("docs/config.json");
    harness.create_file("src/main.rs");

    let rel = "../docs/config.json";
    let text = format!("let p = \"{}\";", rel);
    let uri = harness.open_doc("src/main.rs", &text).await;

    let start_offset = text.find(rel).unwrap();
    let line = 0usize;
    let character = text[..start_offset].chars().count();

    let params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position::new(line as u32, (character + 1) as u32),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let res = harness.get_server().goto_definition(params).await.unwrap();
    assert!(res.is_some(), "Expected a definition result");

    match res.unwrap() {
        GotoDefinitionResponse::Scalar(loc) => {
            let expected = tokio::fs::canonicalize(harness.root_path().join("docs/config.json"))
                .await
                .unwrap();
            assert_eq!(
                tokio::fs::canonicalize(&loc.uri.to_file_path().unwrap())
                    .await
                    .unwrap(),
                expected
            );
        }
        _ => panic!("Expected scalar location"),
    }
}

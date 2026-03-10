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

#[tokio::test]
async fn test_goto_definition_with_base_path() {
    let harness = TestHarness::new().await;

    // Create a target file in a folder normally outside the default relative range
    harness.create_file("extra/libs/math.js");
    harness.create_file("src/main.rs");

    // Configure base_path to look in 'extra'
    let cfg = path_server::Config {
        base_path: vec!["${workspaceFolder}/extra".into()],
        completion: path_server::Completion {
            max_results: 0,
            show_hidden_files: true,
            exclude: vec![],
            trigger_next_completion: true,
        },
        highlight: path_server::Highlight { enable: true },
    };
    harness.set_config(cfg).await;

    // Path starts from 'extra' folder
    let path_str = "libs/math.js";
    let text = format!("import {{ add }} from '{}';", path_str);
    let uri = harness.open_doc("src/main.rs", &text).await;

    let start_offset = text.find(path_str).unwrap();
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
    assert!(
        res.is_some(),
        "Expected a definition result for base_path relative path"
    );

    match res.unwrap() {
        GotoDefinitionResponse::Scalar(loc) => {
            let expected = tokio::fs::canonicalize(harness.root_path().join("extra/libs/math.js"))
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

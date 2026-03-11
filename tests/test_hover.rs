mod utils;
use tower_lsp::LanguageServer;
use tower_lsp::lsp_types::*;
use utils::*;

#[tokio::test]
async fn test_hover_relative_integration() {
    // create harness where client does NOT advertise documentLink support
    let harness = TestHarness::new_with_document_link(false).await;

    harness.create_file("data/rel.txt");
    harness.create_file("src/main.rs");

    let content = "let s = \"../data/rel.txt\";";
    let uri = harness.open_doc("src/main.rs", content).await;

    // try scanning across the line to find any hover result inside the path
    let mut found_hover: Option<Hover> = None;
    for c in 0..(content.len() as u32 + 1) {
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: c,
                },
            },
            work_done_progress_params: Default::default(),
        };
        let res = harness.get_server().hover(params).await.unwrap();
        if res.is_some() {
            found_hover = res;
            break;
        }
    }
    assert!(
        found_hover.is_some(),
        "Expected hover result for relative path"
    );
    let hover = found_hover.unwrap();

    match hover.contents {
        HoverContents::Scalar(MarkedString::String(s)) => {
            let url = Url::parse(&s).unwrap();
            let expected = tokio::fs::canonicalize(harness.root_path().join("data/rel.txt"))
                .await
                .unwrap();
            assert_eq!(
                tokio::fs::canonicalize(&url.to_file_path().unwrap())
                    .await
                    .unwrap(),
                expected
            );
        }
        _ => panic!("Expected string hover content"),
    }
}

#[tokio::test]
async fn test_hover_absolute_integration() {
    let harness = TestHarness::new_with_document_link(false).await;

    harness.create_file("abs_dir/target.txt");
    harness.create_file("src/main.rs");

    let abs_path = harness.root_path().join("abs_dir/target.txt");
    let abs_display = format!("{}", abs_path.display());
    let content = format!("let s = \"{}\";", abs_display);
    let uri = harness.open_doc("src/main.rs", &content).await;

    // scan across the line to find hover result inside absolute path
    let mut found_hover: Option<Hover> = None;
    for c in 0..(content.len() as u32 + 1) {
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: c,
                },
            },
            work_done_progress_params: Default::default(),
        };
        let res = harness.get_server().hover(params).await.unwrap();
        if res.is_some() {
            found_hover = res;
            break;
        }
    }
    assert!(
        found_hover.is_some(),
        "Expected hover result for absolute path"
    );
    let hover = found_hover.unwrap();

    match hover.contents {
        HoverContents::Scalar(MarkedString::String(s)) => {
            let url = Url::parse(&s).unwrap();
            let expected = tokio::fs::canonicalize(abs_path).await.unwrap();
            assert_eq!(
                tokio::fs::canonicalize(&url.to_file_path().unwrap())
                    .await
                    .unwrap(),
                expected
            );
        }
        _ => panic!("Expected string hover content"),
    }
}

mod utils;
use crate::utils::*;

#[tokio::test]
async fn test_simple_relative_completion() {
    let harness = TestHarness::new().await;

    harness.create_file("data/config.json");
    harness.create_file("src/main.rs");

    let content = "let f = \"./da";
    let uri = harness.open_doc("src/main.rs", content).await;

    harness
        .assert_completion_contains(&uri, 0, content.len() as u32, "data")
        .await;
}

#[tokio::test]
async fn test_sibling_file_completion() {
    let harness = TestHarness::new().await;

    harness.create_file("images/logo.png");
    harness.create_file("src/abcd.md");
    harness.create_file("src/mod/info.rs");
    harness.create_file("README.md");

    let content = "Check ../ab";
    let uri = harness.open_doc("src/mod/info.rs", content).await;

    harness
        .assert_completion_contains(&uri, 0, content.len() as u32, "abcd.md")
        .await;
}

#[tokio::test]
async fn test_workspace_relative_completion() {
    let harness = TestHarness::new().await;

    harness.create_file("workspace/docs/readme.md");
    harness.create_file("src/main.rs");

    // reference workspace relative path
    let content = "let s = \"./workspace/do";
    let uri = harness.open_doc("src/main.rs", content).await;

    harness
        .assert_completion_contains(&uri, 0, content.len() as u32, "docs")
        .await;
}

#[tokio::test]
async fn test_document_relative_completion_current_dir() {
    let harness = TestHarness::new().await;

    harness.create_file("src/mod/data/config.toml");
    harness.create_file("src/mod/info.rs");

    let content = "let p = \"./da";
    let uri = harness.open_doc("src/mod/info.rs", content).await;

    harness
        .assert_completion_contains(&uri, 0, content.len() as u32, "data")
        .await;
}

#[tokio::test]
async fn test_document_relative_completion_parent_dir() {
    let harness = TestHarness::new().await;

    harness.create_file("src/abcd.md");
    harness.create_file("src/mod/info.rs");

    let content = "let r = \"../ab";
    let uri = harness.open_doc("src/mod/info.rs", content).await;

    harness
        .assert_completion_contains(&uri, 0, content.len() as u32, "abcd.md")
        .await;
}

#[tokio::test]
async fn test_absolute_path_completion() {
    let harness = TestHarness::new().await;

    // create a directory under the temp workspace root and a file inside it
    harness.create_file("abs_dir/config.json");
    let abs_dir = harness.root_path().join("abs_dir");
    let abs_prefix = format!("{}/", abs_dir.display());

    let content = format!("let a = \"{}con", abs_prefix);
    let uri = harness.open_doc("src/main.rs", &content).await;

    harness
        .assert_completion_contains(&uri, 0, content.len() as u32, "config.json")
        .await;
}

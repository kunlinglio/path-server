mod utils;
use path_server::{Completion, Config};
use utils::*;

use tower_lsp::lsp_types;

/// test config.completion.maxResults
#[tokio::test]
async fn test_config_max_results_limits_items() {
    let harness = TestHarness::new().await;

    for i in 0..20 {
        let path = format!("many/file{}.txt", i);
        harness.create_file(&path);
    }
    harness.create_file("src/main.rs");

    let completion_cfg = Completion {
        max_results: 5,
        show_hidden_files: true,
        exclude: vec![],
        base_path: vec!["${workspaceFolder}".into()],
        trigger_next_completion: true,
    };
    let cfg = Config {
        completion: completion_cfg,
    };
    harness.set_config(cfg).await;

    let content = "let f = \"./many/fi";
    let uri = harness.open_doc("src/main.rs", content).await;

    let items = harness
        .completion_items(&uri, 0, content.len() as u32)
        .await;

    assert_eq!(items.len(), 5);
}

/// test config.completion.showHiddenFiles
#[tokio::test]
async fn test_config_show_hidden_files_false() {
    let harness = TestHarness::new().await;

    harness.create_file("hidden_dir/.secret.txt");
    harness.create_file("src/main.rs");

    let completion_cfg = Completion {
        max_results: 0,
        show_hidden_files: false,
        exclude: vec![],
        base_path: vec!["${workspaceFolder}".into()],
        trigger_next_completion: true,
    };
    let cfg = Config {
        completion: completion_cfg,
    };
    harness.set_config(cfg).await;

    let content = "let p = \"./hidden_dir/.";
    let uri = harness.open_doc("src/main.rs", content).await;

    let items = harness
        .completion_items(&uri, 0, content.len() as u32)
        .await;

    // hidden file should not be returned when show_hidden_files is false
    assert!(items.is_empty());
}

/// test config.completion.exclude
#[tokio::test]
async fn test_config_exclude() {
    let harness = TestHarness::new().await;

    harness.create_file("exclude_dir/keep.txt");
    harness.create_file("exclude_dir/ignore.log");
    harness.create_file("src/main.rs");

    let completion_cfg = Completion {
        max_results: 0,
        show_hidden_files: true,
        exclude: vec!["*.log".into()],
        base_path: vec!["${workspaceFolder}".into()],
        trigger_next_completion: true,
    };
    let cfg = Config {
        completion: completion_cfg,
    };
    harness.set_config(cfg).await;

    let content = "let f = \"./exclude_dir/";
    let uri = harness.open_doc("src/main.rs", content).await;

    let items = harness
        .completion_items(&uri, 0, content.len() as u32)
        .await;
    let labels: Vec<String> = items.into_iter().map(|i| i.label).collect();

    assert!(labels.contains(&"keep.txt".to_string()));
    assert!(!labels.contains(&"ignore.log".to_string()));
}

/// test config.completion.basePath
#[tokio::test]
async fn test_config_base_path() {
    let harness = TestHarness::new().await;

    harness.create_file("alt/data/config.json");
    harness.create_file("src/main.rs");

    let completion_cfg = Completion {
        max_results: 0,
        show_hidden_files: true,
        exclude: vec![],
        base_path: vec!["${workspaceFolder}/alt".into()],
        trigger_next_completion: true,
    };
    let cfg = Config {
        completion: completion_cfg,
    };
    harness.set_config(cfg).await;

    let content = "let p = \"./data/co";
    let uri = harness.open_doc("src/main.rs", content).await;

    let items = harness
        .completion_items(&uri, 0, content.len() as u32)
        .await;
    let labels: Vec<String> = items.into_iter().map(|i| i.label).collect();

    assert!(labels.contains(&"config.json".to_string()));
}

/// test config.completion.triggerNextCompletion
#[tokio::test]
async fn test_config_trigger_next_completion() {
    let harness = TestHarness::new().await;

    // test trigger_next_completion == true
    harness.create_file("dir/file1");
    harness.create_file("dir/file2");

    let cfg_with = Completion {
        max_results: 0,
        show_hidden_files: true,
        exclude: vec![],
        base_path: vec!["${workspaceFolder}".into()],
        trigger_next_completion: true,
    };
    harness
        .set_config(Config {
            completion: cfg_with,
        })
        .await;

    let content = "let f = ./d";
    let uri = harness.open_doc("src/main.rs", content).await;

    let items = harness
        .completion_items(&uri, 0, content.len() as u32)
        .await;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].insert_text.clone().unwrap(), "dir/");
    assert_eq!(
        items[0].command,
        Some(lsp_types::Command {
            title: "triggerSuggest".to_string(),
            command: "editor.action.triggerSuggest".to_string(),
            arguments: None,
        })
    );

    // test trigger_next_completion == false
    let cfg_without = Completion {
        max_results: 0,
        show_hidden_files: true,
        exclude: vec![],
        base_path: vec!["${workspaceFolder}".into()],
        trigger_next_completion: false,
    };
    harness
        .set_config(Config {
            completion: cfg_without,
        })
        .await;

    let content = "let f = ./d";
    let uri = harness.open_doc("src/main.rs", content).await;

    let items = harness
        .completion_items(&uri, 0, content.len() as u32)
        .await;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].insert_text.clone().unwrap(), "dir");
    assert_eq!(items[0].command, None);
}

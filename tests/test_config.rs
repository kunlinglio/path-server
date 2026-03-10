mod utils;
use path_server::{Completion, Config, Highlight};
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
        trigger_next_completion: true,
    };
    let cfg = Config {
        base_path: vec!["${workspaceFolder}".into()],
        completion: completion_cfg,
        highlight: Highlight { enable: true },
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
        trigger_next_completion: true,
    };
    let cfg = Config {
        base_path: vec!["${workspaceFolder}".into()],
        completion: completion_cfg,
        highlight: Highlight { enable: true },
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
        trigger_next_completion: true,
    };
    let cfg = Config {
        base_path: vec!["${workspaceFolder}".into()],
        completion: completion_cfg,
        highlight: Highlight { enable: true },
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
        trigger_next_completion: true,
    };
    let cfg = Config {
        base_path: vec!["${workspaceFolder}/alt".into()],
        completion: completion_cfg,
        highlight: Highlight { enable: true },
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
        trigger_next_completion: true,
    };
    harness
        .set_config(Config {
            base_path: vec!["${workspaceFolder}".into()],
            completion: cfg_with,
            highlight: Highlight { enable: true },
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
        trigger_next_completion: false,
    };
    harness
        .set_config(Config {
            base_path: vec!["${workspaceFolder}".into()],
            completion: cfg_without,
            highlight: Highlight { enable: true },
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

/// test config.highlight.enable
#[tokio::test]
async fn test_config_highlight_enable() {
    let harness = TestHarness::new().await;

    harness.create_file("existing_file.txt");
    harness.create_file("src/main.rs");

    // test highlight.enable == true
    let cfg_with = Config {
        base_path: vec!["${workspaceFolder}".into()],
        completion: path_server::Completion {
            max_results: 0,
            show_hidden_files: true,
            exclude: vec![],
            trigger_next_completion: true,
        },
        highlight: Highlight { enable: true },
    };
    harness.set_config(cfg_with).await;

    let content = "let p = \"./existing_file.txt\"";
    let uri = harness.open_doc("src/main.rs", content).await;

    let links = harness.document_links(&uri).await;
    // Current behavior: should be non-empty when enabled
    assert!(
        !links.is_empty(),
        "Document links should be present when highlight is enabled"
    );

    // test highlight.enable == false
    let cfg_without = Config {
        base_path: vec!["${workspaceFolder}".into()],
        completion: path_server::Completion {
            max_results: 0,
            show_hidden_files: true,
            exclude: vec![],
            trigger_next_completion: true,
        },
        highlight: Highlight { enable: false },
    };
    harness.set_config(cfg_without).await;

    let links_disabled = harness.document_links(&uri).await;
    // Current behavior: should be empty when disabled
    assert!(
        links_disabled.is_empty(),
        "Document links should be empty when highlight is disabled"
    );
}

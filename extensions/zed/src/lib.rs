mod config;
use crate::config::*;
use zed::settings::LspSettings;
use zed_extension_api::{self as zed, serde_json};

const EXECUTABLE_DIR: &str = "bin/";

fn version_gt(a: &str, b: &str) -> bool {
    let a = if let Some(a) = a.strip_prefix('v') {
        a
    } else {
        a
    };
    let b = if let Some(b) = b.strip_prefix('v') {
        b
    } else {
        b
    };
    let a_parts: Vec<u32> = a.split('.').map(|s| s.parse().unwrap_or(0)).collect();
    let b_parts: Vec<u32> = b.split('.').map(|s| s.parse().unwrap_or(0)).collect();

    for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
        if a_part > b_part {
            return true;
        }
        if a_part < b_part {
            return false;
        }
    }

    false
}

#[derive(Default)]
struct PathServerExtension {}

impl PathServerExtension {
    /// Returns downloaded version or error_msg
    fn fetch_new_version(
        language_server_id: &zed::LanguageServerId,
        current_version: Option<&str>,
    ) -> Result<Option<String>, String> {
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        // 1. fetch latest releases
        let release = zed::latest_github_release(
            PATH_SERVER_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )
        .map_err(|e| format!("Failed to fetch latest release: {}", e))?;

        // 2. check if version matched release
        // TODO: support finding highest compatible version
        let mut compatible = false;
        for comp_major_version in COMPATIBLE_MAJOR_VERSIONS {
            if release
                .version
                .starts_with(&format!("v{}.", comp_major_version))
            {
                compatible = true;
                break;
            }
        }
        if !compatible {
            return Err(format!("Incompatible version: {}", release.version));
        }

        // 3. check if version higher than current
        if let Some(current_version) = current_version
            && !version_gt(&release.version, current_version)
        {
            return Ok(None);
        }

        // executable naming: path-server-{version}-{target}
        let (platform, arch) = zed::current_platform();
        let target = match (platform, arch) {
            (zed::Os::Mac, zed::Architecture::X8664) => "x86_64-apple-darwin",
            (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
            (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
            (zed::Os::Linux, zed::Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
            (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-msvc.exe",
            (zed::Os::Windows, zed::Architecture::Aarch64) => "aarch64-pc-windows-msvc.exe",
            _ => {
                return Err("Unsupported platform or architecture.".to_string());
            }
        };
        let asset_name = format!("path-server_{}_{}", release.version, target);
        let Some(asset) = release.assets.iter().find(|asset| asset.name == asset_name) else {
            return Err("No asset found with name.".to_string());
        };

        // 3. create dir: path-server-vx.x.x
        let Ok(_) = std::fs::create_dir_all(EXECUTABLE_DIR) else {
            return Err("Failed to create executable directory.".to_string());
        };

        // 4. download executable
        let binary_path = format!("{EXECUTABLE_DIR}/{}", asset.name);
        if !std::fs::metadata(&binary_path).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            let Ok(_) = zed::download_file(
                &asset.download_url,
                &binary_path,
                zed::DownloadedFileType::Uncompressed,
            ) else {
                return Err("Failed to download executable.".to_string());
            };
            let Ok(_) = zed::make_file_executable(&binary_path) else {
                return Err("Failed to make executable.".to_string());
            };

            // 5. clean up old versions
            let Ok(entries) = std::fs::read_dir(EXECUTABLE_DIR) else {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &zed::LanguageServerInstallationStatus::Failed(
                        "Failed to read executable directory.".to_string(),
                    ),
                );
                return Err("Failed to read executable directory.".to_string());
            };
            for entry in entries {
                let Ok(entry) = entry else {
                    continue;
                };
                let entry_path = entry.path();
                if !entry_path.is_file() {
                    continue;
                };
                let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.starts_with("path-server_") && name != asset.name {
                    std::fs::remove_file(&entry_path).ok();
                }
            }
        };
        Ok(Some(release.version))
    }

    /// Find the highest version exists in EXECUTABLE_DIR
    fn get_installed_version() -> Option<String> {
        let entries = std::fs::read_dir(EXECUTABLE_DIR).ok()?;
        let mut max_version: Option<String> = None;
        for entry in entries.flatten() {
            let name = entry.file_name().to_str()?.to_string();
            if name.starts_with("path-server_") {
                // extract v0.1.0 from path-server_v0.1.0_target
                let parts: Vec<&str> = name.split('_').collect();
                if parts.len() >= 2 {
                    let version = parts[1].to_string();
                    if max_version
                        .as_ref()
                        .is_none_or(|max| version_gt(&version, max))
                    {
                        max_version = Some(version);
                    }
                }
            }
        }
        max_version
    }

    fn get_binary_path(language_server_id: &zed::LanguageServerId) -> zed::Result<String> {
        let current_version = Self::get_installed_version();

        let version_to_run =
            match Self::fetch_new_version(language_server_id, current_version.as_deref()) {
                Ok(Some(new_version)) => {
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::None,
                    );
                    new_version
                }
                Ok(None) => {
                    // No need to upgrade
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::None,
                    );
                    current_version
                        .ok_or("No cached executable found and no update available".to_string())?
                }
                Err(e) => {
                    // Upgrade failed
                    zed::set_language_server_installation_status(
                        language_server_id,
                        &zed::LanguageServerInstallationStatus::Failed(e.clone()),
                    );
                    current_version.ok_or(format!(
                        "No cached executable found and update failed: {}",
                        e
                    ))?
                }
            };

        let entries = std::fs::read_dir(EXECUTABLE_DIR).map_err(|e| e.to_string())?;
        let mut binary_path = None;
        for entry in entries.flatten() {
            let name = entry.file_name().to_str().unwrap_or("").to_string();
            if name.contains(&version_to_run) {
                binary_path = Some(entry.path().to_string_lossy().to_string());
                break;
            }
        }
        binary_path.ok_or("Binary not found.".to_string())
    }
}

impl zed::Extension for PathServerExtension {
    fn new() -> Self {
        Self::default()
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let binary_path = Self::get_binary_path(language_server_id)?;
        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: Default::default(),
        })
    }

    fn language_server_workspace_configuration(
        &mut self,
        _server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree("path-server", worktree)
            .ok()
            .and_then(|lsp_settings| lsp_settings.settings)
            .unwrap_or_default();
        Ok(Some(serde_json::json!({
            "path-server": settings
        })))
    }
}

zed::register_extension!(PathServerExtension);

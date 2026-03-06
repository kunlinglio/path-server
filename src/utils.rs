use std::path::PathBuf;

use crate::common::*;
use tower_lsp::lsp_types;

pub fn url_to_path(url: &lsp_types::Url) -> PathServerResult<PathBuf> {
    if url.scheme() != "file" {
        return Err(PathServerError::Unsupported(format!(
            "Non-local url is not supported: {}",
            url
        )));
    }
    url.to_file_path().map_err(|_| {
        PathServerError::Unknown(format!("Failed to convert URL to file path: {}", url))
    })
}

// pub fn path_to_url(path: &PathBuf) -> PathServerResult<lsp_types::Url> {
//     lsp_types::Url::from_file_path(path).map_err(|_| {
//         PathServerError::Unknown(format!(
//             "Failed to convert file path to URL: {}",
//             path.display()
//         ))
//     })
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_path() {
        // valid file url
        #[cfg(not(windows))]
        let url_str = "file:///tmp";
        #[cfg(windows)]
        let url_str = "file:///C:/tmp";
        let url = lsp_types::Url::parse(url_str).unwrap();
        let path = url_to_path(&url).unwrap();
        assert!(path.ends_with("tmp"));

        // non-file scheme should error
        let url = lsp_types::Url::parse("http://example.com").unwrap();
        let err = url_to_path(&url).unwrap_err();
        match err {
            PathServerError::Unsupported(_) => {}
            _ => assert!(false, "expected Unsupported error, got: {}", err),
        }
    }
}

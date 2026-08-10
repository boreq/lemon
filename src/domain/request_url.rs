use http::Uri;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RequestUrl {
    uri: Uri,
}

impl RequestUrl {
    pub fn from_uri(uri: &Uri) -> Self {
        Self { uri: uri.clone() }
    }

    pub fn parse(path_and_query: &str) -> Self {
        let uri = path_and_query
            .parse::<Uri>()
            .unwrap_or_else(|_| Uri::from_static("/"));
        Self { uri }
    }

    pub fn path(&self) -> &str {
        self.uri.path()
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.path().split('/').filter(|s| !s.is_empty())
    }

    pub fn file_name(&self) -> Option<&str> {
        self.segments().last()
    }

    pub fn extension(&self) -> Option<&str> {
        self.file_name()
            .and_then(|name| Path::new(name).extension())
            .and_then(|ext| ext.to_str())
    }

    pub fn query(&self) -> Option<&str> {
        self.uri.query()
    }

    pub fn query_mentions(&self, needle: &str) -> bool {
        let needle = needle.to_ascii_lowercase();
        self.query()
            .map(|q| q.to_ascii_lowercase().contains(&needle))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_path_segments_and_query() {
        let url = RequestUrl::parse("/app/config/.env?raw=1");
        assert_eq!(url.path(), "/app/config/.env");
        assert_eq!(
            url.segments().collect::<Vec<_>>(),
            ["app", "config", ".env"]
        );
        assert_eq!(url.file_name(), Some(".env"));
        assert_eq!(url.extension(), None);
        assert_eq!(url.query(), Some("raw=1"));
    }

    #[test]
    fn parses_extension() {
        let url = RequestUrl::parse("/phpinfo.php");
        assert_eq!(url.file_name(), Some("phpinfo.php"));
        assert_eq!(url.extension(), Some("php"));
        assert_eq!(url.query(), None);
    }

    #[test]
    fn empty_path_becomes_root() {
        let url = RequestUrl::parse("/");
        assert_eq!(url.path(), "/");
        assert!(url.segments().next().is_none());
        assert_eq!(url.file_name(), None);
    }

    #[test]
    fn query_mentions_is_case_insensitive() {
        let url = RequestUrl::parse("/?PHPInfo=1");
        assert!(url.query_mentions("phpinfo"));
        assert!(!url.query_mentions("env"));
    }
}

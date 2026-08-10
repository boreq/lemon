use http::Uri;
use percent_encoding::percent_decode_str;
use std::path::Path;

/// Scanners hide the interesting part of a path behind percent encoding, sometimes
/// applied more than once. Decoding is repeated until it stops changing anything so
/// that `/%2561uth.json` reaches the same generator as `/auth.json`.
const MAX_DECODE_PASSES: usize = 3;

#[derive(Debug, Clone)]
pub struct RequestUrl {
    path: String,
    query: Option<String>,
}

impl RequestUrl {
    pub fn from_uri(uri: &Uri) -> Self {
        Self {
            path: normalize(uri.path()),
            query: uri.query().map(normalize),
        }
    }

    pub fn parse(path_and_query: &str) -> Self {
        let uri = path_and_query
            .parse::<Uri>()
            .unwrap_or_else(|_| Uri::from_static("/"));
        Self::from_uri(&uri)
    }

    pub fn path(&self) -> &str {
        &self.path
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
        self.query.as_deref()
    }

    pub fn query_mentions(&self, needle: &str) -> bool {
        let needle = needle.to_ascii_lowercase();
        self.query()
            .map(|q| q.to_ascii_lowercase().contains(&needle))
            .unwrap_or(false)
    }
}

/// Percent decodes and then drops control characters, so null byte tricks like
/// `/%00.env.swp` are matched as `/.env.swp`.
fn normalize(raw: &str) -> String {
    let mut current = raw.to_string();

    for _ in 0..MAX_DECODE_PASSES {
        let decoded = percent_decode_str(&current)
            .decode_utf8_lossy()
            .into_owned();
        if decoded == current {
            break;
        }
        current = decoded;
    }

    current.retain(|c| !c.is_control());
    current
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

    #[test]
    fn decodes_percent_encoded_path() {
        let url = RequestUrl::parse("/%61%75%74%68.%6a%73%6f%6e");
        assert_eq!(url.path(), "/auth.json");
        assert_eq!(url.file_name(), Some("auth.json"));
        assert_eq!(url.extension(), Some("json"));
    }

    #[test]
    fn decodes_encoded_slashes_into_segments() {
        let url = RequestUrl::parse("/app%2Fconfig%2F.env");
        assert_eq!(url.path(), "/app/config/.env");
        assert_eq!(url.file_name(), Some(".env"));
    }

    #[test]
    fn decodes_repeatedly_encoded_path() {
        let url = RequestUrl::parse("/%2561uth.json");
        assert_eq!(url.path(), "/auth.json");
    }

    #[test]
    fn strips_null_bytes_and_other_control_characters() {
        let url = RequestUrl::parse("/%00.env.swp");
        assert_eq!(url.path(), "/.env.swp");
        assert_eq!(url.file_name(), Some(".env.swp"));

        let url = RequestUrl::parse("/%00auth.json");
        assert_eq!(url.path(), "/auth.json");

        let url = RequestUrl::parse("/aws-config%0d%0a.js");
        assert_eq!(url.path(), "/aws-config.js");
    }

    #[test]
    fn decodes_query() {
        let url = RequestUrl::parse("/?%70%68%70info=1");
        assert!(url.query_mentions("phpinfo"));
    }

    #[test]
    fn invalid_utf8_does_not_panic() {
        let url = RequestUrl::parse("/%ff%fe.env");
        assert!(url.file_name().is_some());
    }
}

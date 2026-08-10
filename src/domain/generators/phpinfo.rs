use crate::domain::{Payload, PayloadGenerator, RequestUrl};
use askama::Template;

pub struct PhpInfoGenerator;

const PHP_EXTENSIONS: &[&str] = &["php", "php3", "php4", "php5", "phtml", "phps"];
const PHPINFO_STEMS: &[&str] = &["phpinfo", "php-info", "php_info", "phpversion"];
const GENERIC_PHP_STEMS: &[&str] = &["info", "test", "i", "infophp"];

impl PhpInfoGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PhpInfoGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadGenerator for PhpInfoGenerator {
    fn name(&self) -> &'static str {
        "phpinfo"
    }

    fn supports(&self, request: &RequestUrl) -> bool {
        if let Some(name) = request.file_name() {
            let name = name.to_ascii_lowercase();
            let (stem, is_php_ext) = split_php_extension(&name);

            if PHPINFO_STEMS.contains(&stem) {
                return true;
            }
            if is_php_ext && GENERIC_PHP_STEMS.contains(&stem) {
                return true;
            }
        }

        request.query_mentions("phpinfo")
    }

    fn generate(&self, request: &RequestUrl) -> Payload {
        let now = chrono::Utc::now();
        let template = PhpInfoTemplate {
            script_path: request.path().to_string(),
            script_filename: format!("/var/www/html{}", request.path()),
            request_time: now.timestamp().to_string(),
            request_time_float: format!("{}.{:03}", now.timestamp(), now.timestamp_subsec_millis()),
        };
        Payload::html(template.render().unwrap_or_default())
    }
}

fn split_php_extension(name: &str) -> (&str, bool) {
    match name.rsplit_once('.') {
        Some((stem, ext)) if PHP_EXTENSIONS.contains(&ext) => (stem, true),
        _ => (name, false),
    }
}

#[derive(Template)]
#[template(path = "phpinfo.html")]
struct PhpInfoTemplate {
    script_path: String,
    script_filename: String,
    request_time: String,
    request_time_float: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supports(path: &str) -> bool {
        PhpInfoGenerator::new().supports(&RequestUrl::parse(path))
    }

    #[test]
    fn supports_all_filename_variants() {
        for name in [
            "php-info",
            "php-info.php",
            "php_info",
            "php_info.php",
            "phpinfo",
            "phpinfo.php",
            "phpinfo.php3",
        ] {
            assert!(supports(&format!("/{name}")), "should support /{name}");
            assert!(
                supports(&format!("/deep/nested/{name}")),
                "should support nested /{name}"
            );
            assert!(
                supports(&format!("/{}", name.to_uppercase())),
                "should support uppercase /{name}"
            );
        }
    }

    #[test]
    fn supports_generic_and_query_probes() {
        assert!(supports("/info.php"));
        assert!(supports("/test.php"));
        assert!(supports("/index.php?phpinfo=1"));
        assert!(supports("/?page=phpinfo"));
    }

    #[test]
    fn ignores_unrelated_paths() {
        assert!(!supports("/index.php"));
        assert!(!supports("/wp-login.php"));
        assert!(!supports("/.env"));
        assert!(!supports("/info"));
        assert!(!supports("/test"));
        assert!(!supports("/"));
    }

    #[test]
    fn generates_real_phpinfo_page() {
        let payload = PhpInfoGenerator::new().generate(&RequestUrl::parse("/phpinfo.php"));
        let body = String::from_utf8(payload.into_body()).unwrap();
        assert!(body.contains("<title>phpinfo()</title>"));
        assert!(body.contains("PHP Version 5.6.40"));
        assert!(body.contains("<td class=\"v\">/phpinfo.php</td>"));
        assert!(body.contains("/var/www/html/phpinfo.php"));
    }
}

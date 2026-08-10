use crate::domain::{Payload, PayloadGenerator, Request};
use askama::Template;
use rand::Rng;
use std::time::Duration;

const SITE_TITLE: &str = "My Site";
const MIN_DELAY_MS: u64 = 2000;
const MAX_DELAY_MS: u64 = 6000;

pub struct WpLoginGenerator;

impl WpLoginGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WpLoginGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadGenerator for WpLoginGenerator {
    fn name(&self) -> &'static str {
        "wp_login"
    }

    fn supports(&self, request: &Request) -> bool {
        request
            .url()
            .file_name()
            .map(|name| name.eq_ignore_ascii_case("wp-login.php"))
            .unwrap_or(false)
    }

    fn generate(&self, request: &Request) -> Payload {
        if request.is_post() {
            reject(request)
        } else {
            login_form()
        }
    }
}

fn login_form() -> Payload {
    let template = WpLoginTemplate {
        site_title: SITE_TITLE.to_string(),
        nonce: nonce(),
        username: String::new(),
        redirect_to: "/wp-admin/".to_string(),
        has_error: false,
        empty_username: false,
        empty_password: false,
    };
    Payload::html(template.render().unwrap_or_default())
}

fn reject(request: &Request) -> Payload {
    let username = request.form_field("log").unwrap_or_default();
    let password = request.form_field("pwd").unwrap_or_default();
    let redirect_to = request.form_field("redirect_to").unwrap_or_default();

    let empty_username = username.trim().is_empty();
    let empty_password = !empty_username && password.is_empty();

    let template = WpLoginTemplate {
        site_title: SITE_TITLE.to_string(),
        nonce: nonce(),
        username,
        redirect_to,
        has_error: true,
        empty_username,
        empty_password,
    };

    let mut rng = rand::thread_rng();
    let delay = Duration::from_millis(rng.gen_range(MIN_DELAY_MS..=MAX_DELAY_MS));

    Payload::html(template.render().unwrap_or_default()).with_delay(delay)
}

fn nonce() -> String {
    const CHARS: &[u8] = b"0123456789abcdef";
    let mut rng = rand::thread_rng();
    (0..10)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

#[derive(Template)]
#[template(path = "wp_login.html")]
struct WpLoginTemplate {
    site_title: String,
    nonce: String,
    username: String,
    redirect_to: String,
    has_error: bool,
    empty_username: bool,
    empty_password: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supports(request: &Request) -> bool {
        WpLoginGenerator::new().supports(request)
    }

    fn render(request: &Request) -> String {
        String::from_utf8(WpLoginGenerator::new().generate(request).into_body()).unwrap()
    }

    #[test]
    fn supports_wp_login_anywhere_and_ignores_others() {
        assert!(supports(&Request::get("/wp-login.php")));
        assert!(supports(&Request::get("/WP-LOGIN.PHP")));
        assert!(supports(&Request::post("/wp/wp-login.php", "")));
        assert!(!supports(&Request::get("/wp-admin/")));
        assert!(!supports(&Request::get("/xmlrpc.php")));
        assert!(!supports(&Request::get("/")));
    }

    #[test]
    fn get_serves_the_login_form_without_an_error_or_delay() {
        let payload = WpLoginGenerator::new().generate(&Request::get("/wp-login.php"));
        assert!(payload.delay().is_none());

        let body = String::from_utf8(payload.into_body()).unwrap();
        assert!(body.contains("<form name=\"loginform\""));
        assert!(!body.contains("login_error"));
        assert!(body.contains("value=\"/wp-admin/\""));
        assert!(body.contains("getElementById( \"user_login\" )"));
    }

    #[test]
    fn post_rejects_bad_credentials_like_wordpress_does() {
        let request = Request::post("/wp-login.php", "log=admin&pwd=hunter2&redirect_to=");
        let payload = WpLoginGenerator::new().generate(&request);

        let delay = payload.delay().expect("credential submissions are delayed");
        assert!(delay >= Duration::from_millis(MIN_DELAY_MS));
        assert!(delay <= Duration::from_millis(MAX_DELAY_MS));

        let body = String::from_utf8(payload.into_body()).unwrap();
        assert!(body.contains(
            "The password you entered for the username <strong>admin</strong> is incorrect."
        ));
        assert!(body.contains("value=\"admin\""));
        assert!(body.contains("classList.add('shake')"));
        assert!(body.contains("getElementById( \"user_pass\" )"));
    }

    #[test]
    fn post_reports_an_empty_username() {
        let body = render(&Request::post("/wp-login.php", "log=&pwd=x"));
        assert!(body.contains("The username field is empty."));
    }

    #[test]
    fn post_reports_an_empty_password() {
        let body = render(&Request::post("/wp-login.php", "log=admin&pwd="));
        assert!(body.contains("The password field is empty."));
    }

    #[test]
    fn reflected_username_is_html_escaped() {
        let body = render(&Request::post(
            "/wp-login.php",
            "log=%3Cscript%3E&pwd=x",
        ));
        assert!(!body.contains("<script>x"));
        assert!(body.contains("&#60;script&#62;") || body.contains("&lt;script&gt;"));
    }
}

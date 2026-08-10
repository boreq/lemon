use crate::domain::RequestUrl;
use http::Method;
use percent_encoding::percent_decode_str;

#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    url: RequestUrl,
    body: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, url: RequestUrl, body: Vec<u8>) -> Self {
        Self { method, url, body }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn url(&self) -> &RequestUrl {
        &self.url
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn is_post(&self) -> bool {
        self.method == Method::POST
    }

    pub fn form_field(&self, name: &str) -> Option<String> {
        let body = std::str::from_utf8(&self.body).ok()?;
        form_pairs(body)
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    }

    #[cfg(test)]
    pub fn get(path: &str) -> Self {
        Self::new(Method::GET, RequestUrl::parse(path), Vec::new())
    }

    #[cfg(test)]
    pub fn post(path: &str, body: &str) -> Self {
        Self::new(
            Method::POST,
            RequestUrl::parse(path),
            body.as_bytes().to_vec(),
        )
    }
}

fn form_pairs(body: &str) -> impl Iterator<Item = (String, String)> + '_ {
    body.split('&').filter(|pair| !pair.is_empty()).map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        (decode_form(key), decode_form(value))
    })
}

fn decode_form(raw: &str) -> String {
    let plus_decoded = raw.replace('+', " ");
    percent_decode_str(&plus_decoded)
        .decode_utf8_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_form_fields_out_of_the_body() {
        let request = Request::post("/wp-login.php", "log=admin&pwd=s3cret&wp-submit=Log+In");
        assert_eq!(request.form_field("log").as_deref(), Some("admin"));
        assert_eq!(request.form_field("pwd").as_deref(), Some("s3cret"));
        assert_eq!(request.form_field("wp-submit").as_deref(), Some("Log In"));
        assert_eq!(request.form_field("missing"), None);
    }

    #[test]
    fn decodes_percent_and_plus_in_form_values() {
        let request = Request::post("/wp-login.php", "log=a%40b.com&pwd=p%20%26q");
        assert_eq!(request.form_field("log").as_deref(), Some("a@b.com"));
        assert_eq!(request.form_field("pwd").as_deref(), Some("p &q"));
    }

    #[test]
    fn get_has_no_body() {
        let request = Request::get("/wp-login.php");
        assert!(!request.is_post());
        assert_eq!(request.form_field("log"), None);
    }
}

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Payload {
    status: u16,
    content_type: String,
    body: Vec<u8>,
    delay: Option<Duration>,
}

impl Payload {
    pub fn new(status: u16, content_type: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type: content_type.into(),
            body: body.into(),
            delay: None,
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn html(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, "text/html; charset=utf-8", body)
    }

    pub fn text(body: impl Into<Vec<u8>>) -> Self {
        Self::new(200, "text/plain; charset=utf-8", body)
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn delay(&self) -> Option<Duration> {
        self.delay
    }

    pub fn into_body(self) -> Vec<u8> {
        self.body
    }
}

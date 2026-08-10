pub mod generators;
mod payload;
mod request_url;

pub use payload::Payload;
pub use request_url::RequestUrl;

pub trait PayloadGenerator: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, request: &RequestUrl) -> bool;
    fn generate(&self, request: &RequestUrl) -> Payload;
}

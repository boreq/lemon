pub mod generators;
mod payload;
mod request;
mod request_url;

pub use payload::Payload;
pub use request::Request;
pub use request_url::RequestUrl;

pub trait PayloadGenerator: Send + Sync {
    fn name(&self) -> &'static str;
    fn supports(&self, request: &Request) -> bool;
    fn generate(&self, request: &Request) -> Payload;
}

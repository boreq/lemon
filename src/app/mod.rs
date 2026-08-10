pub mod dispatch;

use crate::domain::{Payload, Request};

pub trait Dispatch: Send + Sync {
    fn dispatch(&self, request: &Request) -> Option<Payload>;
}

pub trait Metrics: Send + Sync {
    fn record_served(&self, generator: &str, method: &str);
    fn record_miss(&self, method: &str);
}

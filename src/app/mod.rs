pub mod dispatch;

use crate::domain::{Payload, RequestUrl};

pub trait Dispatch: Send + Sync {
    fn dispatch(&self, request: &RequestUrl) -> Option<Payload>;
}

pub trait Metrics: Send + Sync {
    fn record_served(&self, generator: &str);
    fn record_miss(&self);
}

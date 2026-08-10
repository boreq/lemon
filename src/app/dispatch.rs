use crate::app::{Dispatch, Metrics};
use crate::domain::{Payload, PayloadGenerator, RequestUrl};
use std::sync::Arc;

#[derive(Clone)]
pub struct DispatchHandler<M> {
    generators: Arc<Vec<Box<dyn PayloadGenerator>>>,
    metrics: M,
}

impl<M> DispatchHandler<M>
where
    M: Metrics,
{
    pub fn new(generators: Vec<Box<dyn PayloadGenerator>>, metrics: M) -> Self {
        Self {
            generators: Arc::new(generators),
            metrics,
        }
    }
}

impl<M> Dispatch for DispatchHandler<M>
where
    M: Metrics,
{
    fn dispatch(&self, request: &RequestUrl) -> Option<Payload> {
        for generator in self.generators.iter() {
            if generator.supports(request) {
                self.metrics.record_served(generator.name());
                return Some(generator.generate(request));
            }
        }
        self.metrics.record_miss();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::generators::{DotEnvGenerator, PhpInfoGenerator};
    use std::sync::Mutex;

    #[derive(Clone, Default)]
    struct SpyMetrics {
        served: Arc<Mutex<Vec<String>>>,
        misses: Arc<Mutex<usize>>,
    }

    impl Metrics for SpyMetrics {
        fn record_served(&self, generator: &str) {
            self.served.lock().unwrap().push(generator.to_string());
        }
        fn record_miss(&self) {
            *self.misses.lock().unwrap() += 1;
        }
    }

    fn handler(metrics: SpyMetrics) -> DispatchHandler<SpyMetrics> {
        DispatchHandler::new(
            vec![
                Box::new(PhpInfoGenerator::new()),
                Box::new(DotEnvGenerator::new()),
            ],
            metrics,
        )
    }

    #[test]
    fn dispatches_to_first_supporting_generator() {
        let metrics = SpyMetrics::default();
        let h = handler(metrics.clone());

        assert!(h.dispatch(&RequestUrl::parse("/phpinfo.php")).is_some());
        assert!(h.dispatch(&RequestUrl::parse("/app/.env")).is_some());

        assert_eq!(*metrics.served.lock().unwrap(), ["phpinfo", "dotenv"]);
        assert_eq!(*metrics.misses.lock().unwrap(), 0);
    }

    #[test]
    fn records_miss_when_nothing_matches() {
        let metrics = SpyMetrics::default();
        let h = handler(metrics.clone());

        assert!(h.dispatch(&RequestUrl::parse("/index.html")).is_none());

        assert!(metrics.served.lock().unwrap().is_empty());
        assert_eq!(*metrics.misses.lock().unwrap(), 1);
    }
}

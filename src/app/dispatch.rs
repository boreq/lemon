use crate::app::{Dispatch, Metrics};
use crate::domain::{Payload, PayloadGenerator, Request};
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
    fn dispatch(&self, request: &Request) -> Option<Payload> {
        let method = request.method().as_str();
        for generator in self.generators.iter() {
            if generator.supports(request) {
                self.metrics.record_served(generator.name(), method);
                return Some(generator.generate(request));
            }
        }
        self.metrics.record_miss(method);
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
        misses: Arc<Mutex<Vec<String>>>,
    }

    impl Metrics for SpyMetrics {
        fn record_served(&self, generator: &str, method: &str) {
            self.served
                .lock()
                .unwrap()
                .push(format!("{generator}:{method}"));
        }
        fn record_miss(&self, method: &str) {
            self.misses.lock().unwrap().push(method.to_string());
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

        assert!(h.dispatch(&Request::get("/phpinfo.php")).is_some());
        assert!(
            h.dispatch(&Request::post("/app/.env", "raw=1"))
                .is_some()
        );

        assert_eq!(
            *metrics.served.lock().unwrap(),
            ["phpinfo:GET", "dotenv:POST"]
        );
        assert!(metrics.misses.lock().unwrap().is_empty());
    }

    #[test]
    fn records_miss_with_method_when_nothing_matches() {
        let metrics = SpyMetrics::default();
        let h = handler(metrics.clone());

        assert!(h.dispatch(&Request::get("/index.html")).is_none());

        assert!(metrics.served.lock().unwrap().is_empty());
        assert_eq!(*metrics.misses.lock().unwrap(), ["GET"]);
    }
}

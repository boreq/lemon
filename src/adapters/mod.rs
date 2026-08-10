use crate::app;
use crate::config::{Config, Environment};
use crate::errors::Result;
use anyhow::anyhow;
use prometheus::{CounterVec, Opts, Registry, labels};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

pub struct ConfigLoader {
    path: PathBuf,
}

impl ConfigLoader {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Config> {
        let content = fs::read_to_string(&self.path)?;
        let transport: TomlConfig = toml::from_str(&content)?;
        Config::try_from(transport)
    }
}

#[derive(Deserialize)]
struct TomlConfig {
    http_address: String,
    metrics_address: String,
    environment: String,
}

impl TryFrom<TomlConfig> for Config {
    type Error = crate::errors::Error;

    fn try_from(value: TomlConfig) -> std::result::Result<Self, Self::Error> {
        Config::new(
            value.http_address,
            value.metrics_address,
            value.environment.try_into()?,
        )
    }
}

impl TryFrom<String> for Environment {
    type Error = crate::errors::Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            "production" => Ok(Environment::Production),
            "development" => Ok(Environment::Development),
            other => Err(anyhow!("invalid environment: {}", other).into()),
        }
    }
}

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    metric_requests: CounterVec,
}

impl Metrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new_custom(Some("lemon".into()), None)?;

        let metric_requests = CounterVec::new(
            Opts::new("requests_total", "trap requests grouped by generator"),
            &["generator"],
        )?;
        registry.register(Box::new(metric_requests.clone()))?;

        Ok(Self {
            registry,
            metric_requests,
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl app::Metrics for Metrics {
    fn record_served(&self, generator: &str) {
        self.metric_requests
            .with(&labels! { "generator" => generator })
            .inc();
    }

    fn record_miss(&self) {
        self.metric_requests
            .with(&labels! { "generator" => "none" })
            .inc();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::Result;
    use crate::fixtures;

    #[test]
    fn loads_config_from_file_successfully() -> Result<()> {
        let expected = Config::new("0.0.0.0:8080", "127.0.0.1:9090", Environment::Development)?;
        let loader = ConfigLoader::new(fixtures::test_file_path("src/adapters/testdata/config.toml"));
        let config = loader.load()?;
        assert_eq!(expected, config);
        Ok(())
    }
}

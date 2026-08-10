use crate::errors::Result;
use anyhow::anyhow;

#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    http_address: String,
    metrics_address: String,
    environment: Environment,
}

impl Config {
    pub fn new(
        http_address: impl Into<String>,
        metrics_address: impl Into<String>,
        environment: Environment,
    ) -> Result<Self> {
        let http_address = http_address.into();
        if http_address.is_empty() {
            return Err(anyhow!("http_address can't be empty").into());
        }
        let metrics_address = metrics_address.into();
        if metrics_address.is_empty() {
            return Err(anyhow!("metrics_address can't be empty").into());
        }
        Ok(Self {
            http_address,
            metrics_address,
            environment,
        })
    }

    pub fn http_address(&self) -> &str {
        &self.http_address
    }

    pub fn metrics_address(&self) -> &str {
        &self.metrics_address
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Environment {
    Production,
    Development,
}

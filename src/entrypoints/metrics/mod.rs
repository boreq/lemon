use crate::config;
use crate::errors::Result;
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use prometheus::{Registry, TextEncoder};

pub struct Server<'a> {
    config: &'a config::Config,
    registry: Registry,
}

impl<'a> Server<'a> {
    pub fn new(config: &'a config::Config, registry: Registry) -> Self {
        Self { config, registry }
    }

    pub async fn run(&self) -> Result<()> {
        let router = Router::new()
            .route("/metrics", get(handle_metrics))
            .with_state(self.registry.clone());

        let listener = tokio::net::TcpListener::bind(self.config.metrics_address()).await?;
        axum::serve(listener, router.into_make_service()).await?;
        Ok(())
    }
}

async fn handle_metrics(State(registry): State<Registry>) -> std::result::Result<String, StatusCode> {
    let encoder = TextEncoder::new();
    encoder
        .encode_to_string(&registry.gather())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

use crate::app::{self, Dispatch};
use crate::config;
use crate::domain::RequestUrl;
use crate::errors::Result;
use axum::Router;
use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use http::header;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub trait Deps: Clone + Send + Sync + 'static {
    fn dispatch(&self) -> &impl app::Dispatch;
}

pub struct Server<'a, D> {
    config: &'a config::Config,
    deps: D,
}

impl<'a, D> Server<'a, D>
where
    D: Deps,
{
    pub fn new(config: &'a config::Config, deps: D) -> Self {
        Self { config, deps }
    }

    pub async fn run(&self) -> Result<()> {
        let trace = TraceLayer::new_for_http();

        let router = Router::new()
            .fallback(handle_trap::<D>)
            .layer(ServiceBuilder::new().layer(trace))
            .with_state(self.deps.clone());

        let listener = tokio::net::TcpListener::bind(self.config.http_address()).await?;
        axum::serve(listener, router.into_make_service()).await?;
        Ok(())
    }
}

async fn handle_trap<D>(State(deps): State<D>, uri: Uri) -> Response
where
    D: Deps,
{
    let request = RequestUrl::from_uri(&uri);

    match deps.dispatch().dispatch(&request) {
        Some(payload) => {
            let status =
                StatusCode::from_u16(payload.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let content_type = payload.content_type().to_string();
            (
                status,
                [(header::CONTENT_TYPE, content_type)],
                payload.into_body(),
            )
                .into_response()
        }
        None => not_found(),
    }
}

fn not_found() -> Response {
    let body = "<html>\r\n<head><title>404 Not Found</title></head>\r\n\
<body>\r\n<center><h1>404 Not Found</h1></center>\r\n\
<hr><center>nginx</center>\r\n</body>\r\n</html>\r\n";
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/html")],
        body,
    )
        .into_response()
}

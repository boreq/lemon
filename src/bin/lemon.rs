use clap::{Command, arg};
use env_logger::Env;
use lemon::adapters::{self, ConfigLoader};
use lemon::app;
use lemon::app::dispatch::DispatchHandler;
use lemon::config::Config;
use lemon::domain::PayloadGenerator;
use lemon::domain::generators::{
    AwsCredentialsGenerator, DotEnvGenerator, GitConfigGenerator, PhpInfoGenerator,
};
use lemon::entrypoints::{http, metrics};
use lemon::errors::Result;
use log::error;

fn cli() -> Command {
    Command::new("lemon")
        .about("Serves deceptive payloads to malicious crawlers.")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("run")
                .about("Runs the program")
                .arg(arg!(<CONFIG> "Path to the configuration file")),
        )
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().filter_or("RUST_LOG", "info")).init();

    let matches = cli().get_matches();
    match matches.subcommand() {
        Some(("run", sub_matches)) => {
            let config_file_path = sub_matches.try_get_one::<String>("CONFIG")?.unwrap();
            run(config_file_path).await?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn run(config_file_path: &str) -> Result<()> {
    let config = ConfigLoader::new(config_file_path).load()?;
    let service = Service::new(&config)?;

    tokio::join!(
        http_server_loop(&service.http_server),
        metrics_server_loop(&service.metrics_server),
    );
    Ok(())
}

async fn http_server_loop<D>(server: &http::Server<'_, D>)
where
    D: http::Deps,
{
    loop {
        match server.run().await {
            Ok(_) => error!("the http server exited without returning any errors"),
            Err(err) => error!("the http server exited with an error: {err}"),
        }
    }
}

async fn metrics_server_loop(server: &metrics::Server<'_>) {
    loop {
        match server.run().await {
            Ok(_) => error!("the metrics server exited without returning any errors"),
            Err(err) => error!("the metrics server exited with an error: {err}"),
        }
    }
}

#[derive(Clone)]
struct HttpDeps<DH> {
    dispatch_handler: DH,
}

impl<DH> HttpDeps<DH> {
    fn new(dispatch_handler: DH) -> Self {
        Self { dispatch_handler }
    }
}

impl<DH> http::Deps for HttpDeps<DH>
where
    DH: app::Dispatch + Clone + 'static,
{
    fn dispatch(&self) -> &impl app::Dispatch {
        &self.dispatch_handler
    }
}

type DispatchHandlerImpl = DispatchHandler<adapters::Metrics>;
type HttpDepsImpl = HttpDeps<DispatchHandlerImpl>;

struct Service<'a> {
    http_server: http::Server<'a, HttpDepsImpl>,
    metrics_server: metrics::Server<'a>,
}

impl<'a> Service<'a> {
    fn new(config: &'a Config) -> Result<Self> {
        let metrics = adapters::Metrics::new()?;

        let generators: Vec<Box<dyn PayloadGenerator>> = vec![
            Box::new(PhpInfoGenerator::new()),
            Box::new(DotEnvGenerator::new()),
            Box::new(GitConfigGenerator::new()),
            Box::new(AwsCredentialsGenerator::new()),
        ];

        let dispatch_handler = DispatchHandler::new(generators, metrics.clone());
        let http_deps = HttpDeps::new(dispatch_handler);

        let http_server = http::Server::new(config, http_deps);
        let metrics_server = metrics::Server::new(config, metrics.registry().clone());

        Ok(Self {
            http_server,
            metrics_server,
        })
    }
}

use crate::domain::{Payload, PayloadGenerator, RequestUrl};
use askama::Template;
use rand::Rng;
use rand::distributions::Alphanumeric;
use rand::seq::SliceRandom;

pub struct DotEnvGenerator;

impl DotEnvGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DotEnvGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadGenerator for DotEnvGenerator {
    fn name(&self) -> &'static str {
        "dotenv"
    }

    fn supports(&self, request: &RequestUrl) -> bool {
        request
            .file_name()
            .map(|name| name.starts_with(".env"))
            .unwrap_or(false)
    }

    fn generate(&self, _request: &RequestUrl) -> Payload {
        Payload::text(build_template().render().unwrap_or_default())
    }
}

#[derive(Template)]
#[template(path = "dotenv.env", escape = "none")]
struct DotEnvTemplate {
    app: String,
    env: String,
    app_key: String,
    db_host: String,
    db_name: String,
    db_user: String,
    db_pass: String,
    redis_pass: String,
    aws_key: String,
    aws_secret: String,
    mail_pass: String,
    jwt: String,
    stripe_pub: String,
    stripe_secret: String,
}

fn build_template() -> DotEnvTemplate {
    let mut rng = rand::thread_rng();

    let apps = ["acme-api", "shop-backend", "billing", "dashboard", "gateway"];
    let envs = ["production", "staging", "prod"];
    let app = apps.choose(&mut rng).unwrap().to_string();
    let env = envs.choose(&mut rng).unwrap().to_string();

    let db_host = format!("10.0.{}.{}", rng.gen_range(0..=254), rng.gen_range(1..=254));
    let db_name = format!("{}_{}", app.replace('-', "_"), env);
    let db_user = format!("{}_user", app.replace('-', "_"));
    let aws_key = format!("AKIA{}", rand_upper_alnum(&mut rng, 16));

    DotEnvTemplate {
        app,
        env,
        app_key: rand_base64(&mut rng, 32),
        db_host,
        db_name,
        db_user,
        db_pass: rand_alnum(&mut rng, 24),
        redis_pass: rand_alnum(&mut rng, 24),
        aws_key,
        aws_secret: rand_base64(&mut rng, 30),
        mail_pass: rand_alnum(&mut rng, 20),
        jwt: rand_base64(&mut rng, 48),
        stripe_pub: rand_alnum(&mut rng, 24),
        stripe_secret: rand_alnum(&mut rng, 24),
    }
}

fn rand_alnum<R: Rng>(rng: &mut R, len: usize) -> String {
    (0..len)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}

fn rand_upper_alnum<R: Rng>(rng: &mut R, len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

fn rand_base64<R: Rng>(rng: &mut R, bytes: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s: String = (0..bytes)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect();
    while !s.len().is_multiple_of(4) {
        s.push('=');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supports(path: &str) -> bool {
        DotEnvGenerator::new().supports(&RequestUrl::parse(path))
    }

    #[test]
    fn supports_dotenv_in_any_dir() {
        assert!(supports("/.env"));
        assert!(supports("/app/.env"));
        assert!(supports("/deep/nested/dir/.env"));
        assert!(supports("/.env.production"));
        assert!(supports("/config/.env.local"));
        assert!(supports("/.env.bak"));
    }

    #[test]
    fn ignores_unrelated_paths() {
        assert!(!supports("/index.html"));
        assert!(!supports("/environment"));
        assert!(!supports("/"));
    }

    #[test]
    fn generates_plausible_env() {
        let payload = DotEnvGenerator::new().generate(&RequestUrl::parse("/.env"));
        let body = String::from_utf8(payload.into_body()).unwrap();
        assert!(body.contains("DB_PASSWORD="));
        assert!(body.contains("AWS_ACCESS_KEY_ID=AKIA"));
    }
}

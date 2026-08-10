use crate::domain::{Payload, PayloadGenerator, RequestUrl};
use askama::Template;
use rand::Rng;
use rand::distributions::Alphanumeric;
use rand::seq::SliceRandom;

pub struct GitConfigGenerator;

impl GitConfigGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GitConfigGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadGenerator for GitConfigGenerator {
    fn name(&self) -> &'static str {
        "gitconfig"
    }

    fn supports(&self, request: &RequestUrl) -> bool {
        let segments: Vec<&str> = request.segments().collect();
        matches!(segments.as_slice(), [.., ".git", "config"])
    }

    fn generate(&self, _request: &RequestUrl) -> Payload {
        Payload::text(build_template().render().unwrap_or_default())
    }
}

#[derive(Template)]
#[template(path = "gitconfig.ini", escape = "none")]
struct GitConfigTemplate {
    user: String,
    token: String,
    org: String,
    repo: String,
}

fn build_template() -> GitConfigTemplate {
    let mut rng = rand::thread_rng();

    let orgs = ["acme", "globex", "initech", "umbrella", "hooli"];
    let repos = ["backend", "api", "webapp", "infra", "platform"];
    let users = ["deploy", "ci-bot", "jenkins", "gitlab-runner", "svc-build"];

    GitConfigTemplate {
        user: users.choose(&mut rng).unwrap().to_string(),
        token: format!("ghp_{}", rand_alnum(&mut rng, 36)),
        org: orgs.choose(&mut rng).unwrap().to_string(),
        repo: repos.choose(&mut rng).unwrap().to_string(),
    }
}

fn rand_alnum<R: Rng>(rng: &mut R, len: usize) -> String {
    (0..len).map(|_| rng.sample(Alphanumeric) as char).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supports(path: &str) -> bool {
        GitConfigGenerator::new().supports(&RequestUrl::parse(path))
    }

    #[test]
    fn supports_git_config_in_any_dir() {
        assert!(supports("/.git/config"));
        assert!(supports("/app/.git/config"));
        assert!(supports("/deep/nested/dir/.git/config"));
    }

    #[test]
    fn ignores_unrelated_paths() {
        assert!(!supports("/.git/HEAD"));
        assert!(!supports("/config"));
        assert!(!supports("/.git/"));
        assert!(!supports("/gitconfig"));
        assert!(!supports("/"));
    }

    #[test]
    fn generates_plausible_git_config() {
        let payload = GitConfigGenerator::new().generate(&RequestUrl::parse("/.git/config"));
        let body = String::from_utf8(payload.into_body()).unwrap();
        assert!(body.contains("[remote \"origin\"]"));
        assert!(body.contains("ghp_"));
    }
}

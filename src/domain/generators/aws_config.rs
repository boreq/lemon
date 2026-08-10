use crate::domain::{Payload, PayloadGenerator, Request};
use askama::Template;
use rand::Rng;
use rand::seq::SliceRandom;

pub struct AwsConfigGenerator;

impl AwsConfigGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AwsConfigGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadGenerator for AwsConfigGenerator {
    fn name(&self) -> &'static str {
        "aws_config"
    }

    fn supports(&self, request: &Request) -> bool {
        let segments: Vec<&str> = request.url().segments().collect();
        matches!(segments.as_slice(), [.., ".aws", "config"])
    }

    fn generate(&self, _request: &Request) -> Payload {
        Payload::text(build_template().render().unwrap_or_default())
    }
}

#[derive(Template)]
#[template(path = "aws_config.ini", escape = "none")]
struct AwsConfigTemplate {
    sections: Vec<Section>,
}

struct Section {
    header: String,
    settings: Vec<Setting>,
}

impl Section {
    fn new(header: impl Into<String>) -> Self {
        Self {
            header: header.into(),
            settings: Vec::new(),
        }
    }

    fn with(mut self, key: &str, value: impl Into<String>) -> Self {
        self.settings.push(Setting {
            key: key.to_string(),
            value: value.into(),
        });
        self
    }
}

struct Setting {
    key: String,
    value: String,
}

const REGIONS: [&str; 6] = [
    "us-east-1",
    "us-east-2",
    "us-west-2",
    "eu-west-1",
    "eu-central-1",
    "ap-southeast-1",
];

const PROFILES: [&str; 8] = [
    "prod",
    "production",
    "staging",
    "dev",
    "terraform",
    "deploy",
    "backup",
    "s3-uploader",
];

const ROLES: [&str; 5] = [
    "OrganizationAccountAccessRole",
    "AdministratorAccess",
    "PowerUserAccess",
    "TerraformExecutionRole",
    "DeploymentRole",
];

const ORGS: [&str; 5] = ["acme", "globex", "initech", "umbrella", "hooli"];

const USERS: [&str; 5] = ["jenkins", "svc-build", "m.novak", "a.schmidt", "ops"];

fn build_template() -> AwsConfigTemplate {
    let mut rng = rand::thread_rng();

    let org = ORGS.choose(&mut rng).unwrap();
    let home_region = REGIONS.choose(&mut rng).unwrap().to_string();

    let mut sections = vec![
        Section::new("default")
            .with("region", home_region.clone())
            .with("output", "json"),
    ];

    // AWS Identity Center pushes everything through a single shared session so
    // the session section is emitted once and referenced by every profile
    // belonging to it.
    let sso_session = rng.gen_bool(0.5).then(|| {
        let name = format!("{org}-sso");
        sections.push(
            Section::new(format!("sso-session {name}"))
                .with(
                    "sso_start_url",
                    format!("https://d-{}.awsapps.com/start", rand_hex(&mut rng, 10)),
                )
                .with("sso_region", home_region.clone())
                .with("sso_registration_scopes", "sso:account:access"),
        );
        name
    });

    let profile_count = rng.gen_range(1..=3);
    for name in PROFILES.choose_multiple(&mut rng, profile_count) {
        let region = REGIONS.choose(&mut rng).unwrap().to_string();
        let account_id = rand_account_id(&mut rng);
        let section = Section::new(format!("profile {name}"));

        let section = match &sso_session {
            Some(session) => section
                .with("sso_session", session.clone())
                .with("sso_account_id", account_id.to_string())
                .with("sso_role_name", *ROLES.choose(&mut rng).unwrap()),
            None => {
                let section = section
                    .with(
                        "role_arn",
                        format!(
                            "arn:aws:iam::{account_id}:role/{}",
                            ROLES.choose(&mut rng).unwrap()
                        ),
                    )
                    .with("source_profile", "default");

                if rng.gen_bool(0.4) {
                    section.with(
                        "mfa_serial",
                        format!(
                            "arn:aws:iam::{account_id}:mfa/{}",
                            USERS.choose(&mut rng).unwrap()
                        ),
                    )
                } else {
                    section
                }
            }
        };

        sections.push(section.with("region", region).with("output", "json"));
    }

    AwsConfigTemplate { sections }
}

fn rand_account_id<R: Rng>(rng: &mut R) -> u64 {
    rng.gen_range(100_000_000_000..=999_999_999_999)
}

fn rand_hex<R: Rng>(rng: &mut R, len: usize) -> String {
    const CHARS: &[u8] = b"0123456789abcdef";
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supports(path: &str) -> bool {
        AwsConfigGenerator::new().supports(&Request::get(path))
    }

    fn generate() -> String {
        let payload = AwsConfigGenerator::new().generate(&Request::get("/.aws/config"));
        String::from_utf8(payload.into_body()).unwrap()
    }

    fn headers(body: &str) -> Vec<&str> {
        body.lines()
            .filter_map(|line| line.strip_prefix('['))
            .filter_map(|line| line.strip_suffix(']'))
            .collect()
    }

    #[test]
    fn supports_aws_config_in_any_dir() {
        assert!(supports("/.aws/config"));
        assert!(supports("/home/ubuntu/.aws/config"));
        assert!(supports("/deep/nested/dir/.aws/config"));
    }

    #[test]
    fn ignores_unrelated_paths() {
        assert!(!supports("/.aws/credentials"));
        assert!(!supports("/config"));
        assert!(!supports("/.aws/"));
        assert!(!supports("/aws/config"));
        assert!(!supports("/"));
    }

    #[test]
    fn generates_plausible_config() {
        let body = generate();
        assert!(body.contains("[default]"));
        assert!(body.contains("region = "));
        assert!(body.contains("output = json"));
    }

    #[test]
    fn names_non_default_profiles_the_way_the_config_file_does() {
        for _ in 0..100 {
            let body = generate();

            for header in headers(&body) {
                assert!(
                    header == "default"
                        || header.starts_with("profile ")
                        || header.starts_with("sso-session "),
                    "unexpected section header: {header}"
                );
            }
        }
    }

    #[test]
    fn references_only_declared_sso_sessions() {
        for _ in 0..100 {
            let body = generate();

            let declared: Vec<String> = headers(&body)
                .iter()
                .filter_map(|header| header.strip_prefix("sso-session "))
                .map(|name| name.to_string())
                .collect();

            for line in body.lines() {
                if let Some(session) = line.strip_prefix("sso_session = ") {
                    assert!(
                        declared.contains(&session.to_string()),
                        "{session} is referenced but never declared"
                    );
                }
            }
        }
    }

    #[test]
    fn generates_valid_arns_and_account_ids() {
        for _ in 0..100 {
            let body = generate();

            for line in body.lines() {
                let Some((key, value)) = line.split_once(" = ") else {
                    continue;
                };

                match key {
                    "role_arn" | "mfa_serial" => {
                        let parts: Vec<&str> = value.split(':').collect();
                        assert_eq!(parts.len(), 6, "unexpected arn: {value}");
                        assert_eq!(&parts[..3], ["arn", "aws", "iam"]);
                        assert!(parts[3].is_empty(), "iam arns have no region: {value}");
                        assert_account_id(parts[4]);
                    }
                    "sso_account_id" => assert_account_id(value),
                    _ => {}
                }
            }
        }
    }

    fn assert_account_id(value: &str) {
        assert_eq!(value.len(), 12, "unexpected account id: {value}");
        assert!(value.chars().all(|c| c.is_ascii_digit()));
    }
}

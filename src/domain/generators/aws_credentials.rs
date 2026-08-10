use crate::domain::{Payload, PayloadGenerator, RequestUrl};
use askama::Template;
use rand::Rng;
use rand::seq::SliceRandom;

pub struct AwsCredentialsGenerator;

impl AwsCredentialsGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AwsCredentialsGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PayloadGenerator for AwsCredentialsGenerator {
    fn name(&self) -> &'static str {
        "aws_credentials"
    }

    fn supports(&self, request: &RequestUrl) -> bool {
        let segments: Vec<&str> = request.segments().collect();
        matches!(segments.as_slice(), [.., ".aws", "credentials"])
    }

    fn generate(&self, _request: &RequestUrl) -> Payload {
        Payload::text(build_template().render().unwrap_or_default())
    }
}

#[derive(Template)]
#[template(path = "aws_credentials.ini", escape = "none")]
struct AwsCredentialsTemplate {
    profiles: Vec<Profile>,
}

struct Profile {
    name: String,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
    region: String,
}

const REGIONS: [&str; 6] = [
    "us-east-1",
    "us-east-2",
    "us-west-2",
    "eu-west-1",
    "eu-central-1",
    "ap-southeast-1",
];

const EXTRA_PROFILES: [&str; 8] = [
    "prod",
    "production",
    "staging",
    "dev",
    "terraform",
    "deploy",
    "backup",
    "s3-uploader",
];

fn build_template() -> AwsCredentialsTemplate {
    let mut rng = rand::thread_rng();

    let mut names = vec!["default".to_string()];
    let extra_count = rng.gen_range(1..=2);
    let extra = EXTRA_PROFILES
        .choose_multiple(&mut rng, extra_count)
        .map(|name| name.to_string());
    names.extend(extra);

    let profiles = names
        .into_iter()
        .map(|name| {
            // Temporary STS credentials are short lived so only a minority of the
            // profiles carry a session token, the rest are long lived IAM users.
            let temporary = rng.gen_bool(0.25);
            Profile {
                name,
                access_key_id: format!(
                    "{}{}",
                    if temporary { "ASIA" } else { "AKIA" },
                    rand_key_id_suffix(&mut rng, 16)
                ),
                secret_access_key: rand_base64(&mut rng, 40),
                session_token: temporary.then(|| rand_base64(&mut rng, 356)),
                region: REGIONS.choose(&mut rng).unwrap().to_string(),
            }
        })
        .collect();

    AwsCredentialsTemplate { profiles }
}

fn rand_key_id_suffix<R: Rng>(rng: &mut R, len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

fn rand_base64<R: Rng>(rng: &mut R, len: usize) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supports(path: &str) -> bool {
        AwsCredentialsGenerator::new().supports(&RequestUrl::parse(path))
    }

    fn generate() -> String {
        let payload =
            AwsCredentialsGenerator::new().generate(&RequestUrl::parse("/.aws/credentials"));
        String::from_utf8(payload.into_body()).unwrap()
    }

    #[test]
    fn supports_aws_credentials_in_any_dir() {
        assert!(supports("/.aws/credentials"));
        assert!(supports("/home/ubuntu/.aws/credentials"));
        assert!(supports("/deep/nested/dir/.aws/credentials"));
    }

    #[test]
    fn ignores_unrelated_paths() {
        assert!(!supports("/.aws/config"));
        assert!(!supports("/credentials"));
        assert!(!supports("/.aws/"));
        assert!(!supports("/aws/credentials"));
        assert!(!supports("/"));
    }

    #[test]
    fn generates_plausible_credentials() {
        let body = generate();
        assert!(body.contains("[default]"));
        assert!(body.contains("aws_access_key_id = "));
        assert!(body.contains("aws_secret_access_key = "));
        assert!(body.contains("region = "));
    }

    #[test]
    fn generates_keys_with_realistic_shapes() {
        for _ in 0..100 {
            let body = generate();

            for line in body.lines() {
                if let Some(key_id) = line.strip_prefix("aws_access_key_id = ") {
                    assert_eq!(key_id.len(), 20, "unexpected key id: {key_id}");
                    assert!(key_id.starts_with("AKIA") || key_id.starts_with("ASIA"));
                }
                if let Some(secret) = line.strip_prefix("aws_secret_access_key = ") {
                    assert_eq!(secret.len(), 40, "unexpected secret: {secret}");
                }
            }
        }
    }

    #[test]
    fn generates_more_than_one_profile() {
        let body = generate();
        assert!(body.matches('[').count() >= 2);
    }

    #[test]
    fn session_tokens_only_accompany_temporary_credentials() {
        for _ in 0..100 {
            let body = generate();

            for profile in body.split("[").skip(1) {
                if profile.contains("aws_session_token = ") {
                    assert!(profile.contains("aws_access_key_id = ASIA"));
                } else {
                    assert!(profile.contains("aws_access_key_id = AKIA"));
                }
            }
        }
    }
}

#[cfg(test)]
mod sample {
    use super::*;
    #[test]
    fn print_sample() {
        for _ in 0..3 {
            println!("=====\n{}", build_template().render().unwrap());
        }
    }
}

use crate::domain::{Payload, PayloadGenerator, Request};
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

    fn supports(&self, request: &Request) -> bool {
        let segments: Vec<&str> = request.url().segments().collect();
        matches!(segments.as_slice(), [.., ".aws", "credentials"])
    }

    fn generate(&self, _request: &Request) -> Payload {
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
    region: Option<String>,
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
                access_key_id: access_key_id(&mut rng, temporary),
                secret_access_key: secret_access_key(&mut rng),
                session_token: temporary.then(|| session_token(&mut rng)),
                // Regions belong in the config file, they only show up in some
                // credentials files.
                region: rng
                    .gen_bool(0.5)
                    .then(|| REGIONS.choose(&mut rng).unwrap().to_string()),
            }
        })
        .collect();

    AwsCredentialsTemplate { profiles }
}

/// Builds an access key identifier which decodes the way the real ones do.
///
/// A key is a four character prefix followed by ten base32 encoded bytes. The
/// first six of those bytes carry the account identifier so that it can be
/// recovered without calling AWS: `account_id = (bytes[0..6] & 0x7fffffffff80) >> 7`.
/// The top bit is always set and the low seven bits are not part of the
/// account identifier.
fn access_key_id<R: Rng>(rng: &mut R, temporary: bool) -> String {
    let account_id: u64 = rng.gen_range(100_000_000_000..=999_999_999_999);
    let header: u64 = (1 << 47) | (account_id << 7) | rng.gen_range(0..=0x7f);

    let mut bytes = [0u8; 10];
    bytes[..6].copy_from_slice(&header.to_be_bytes()[2..]);
    rng.fill(&mut bytes[6..]);

    let prefix = if temporary { "ASIA" } else { "AKIA" };
    format!("{prefix}{}", base32_encode(&bytes))
}

/// Secrets are thirty random bytes in base64, which is always forty characters
/// without any padding.
fn secret_access_key<R: Rng>(rng: &mut R) -> String {
    let mut bytes = [0u8; 30];
    rng.fill(&mut bytes[..]);
    base64_encode(&bytes)
}

/// Builds a session token which decodes into something with the shape of the
/// real ones: a version byte, a protobuf style `origin_ec` field and then an
/// opaque encrypted blob of a few hundred bytes.
fn session_token<R: Rng>(rng: &mut R) -> String {
    let mut bytes = vec![0x21, 0x0a, 0x09];
    bytes.extend_from_slice(b"origin_ec");

    let blob_len = rng.gen_range(300..=700);
    bytes.push(0x12);
    push_varint(&mut bytes, blob_len as u64);

    let blob_start = bytes.len();
    bytes.resize(blob_start + blob_len, 0);
    rng.fill(&mut bytes[blob_start..]);

    base64_encode(&bytes)
}

fn push_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Encodes bytes using the standard base32 alphabet. The input length has to be
/// a multiple of five bytes so that no padding is needed.
fn base32_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    assert!(bytes.len().is_multiple_of(5));

    bytes
        .chunks(5)
        .flat_map(|chunk| {
            let group = chunk
                .iter()
                .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte));
            (0..8).map(move |i| ALPHABET[((group >> (35 - 5 * i)) & 0x1f) as usize] as char)
        })
        .collect()
}

/// Encodes bytes using the standard base64 alphabet, padding with `=`.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let group = chunk
            .iter()
            .fold(0u32, |acc, byte| (acc << 8) | u32::from(*byte))
            << (8 * (3 - chunk.len()));

        for i in 0..chunk.len() + 1 {
            out.push(ALPHABET[((group >> (18 - 6 * i)) & 0x3f) as usize] as char);
        }
        for _ in 0..3 - chunk.len() {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supports(path: &str) -> bool {
        AwsCredentialsGenerator::new().supports(&Request::get(path))
    }

    fn generate() -> String {
        let payload = AwsCredentialsGenerator::new().generate(&Request::get("/.aws/credentials"));
        String::from_utf8(payload.into_body()).unwrap()
    }

    fn values_of<'a>(body: &'a str, setting: &str) -> Vec<&'a str> {
        let prefix = format!("{setting} = ");
        body.lines()
            .filter_map(|line| line.strip_prefix(prefix.as_str()))
            .collect()
    }

    fn base32_decode(s: &str) -> Vec<u8> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        assert!(s.len().is_multiple_of(8));

        s.as_bytes()
            .chunks(8)
            .flat_map(|chunk| {
                let group = chunk.iter().fold(0u64, |acc, c| {
                    (acc << 5) | ALPHABET.iter().position(|a| a == c).unwrap() as u64
                });
                (0..5).map(move |i| (group >> (32 - 8 * i)) as u8)
            })
            .collect()
    }

    fn base64_decode(s: &str) -> Vec<u8> {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        assert!(s.len().is_multiple_of(4));

        let padding = s.chars().rev().take_while(|c| *c == '=').count();
        let decoded: Vec<u8> = s
            .trim_end_matches('=')
            .as_bytes()
            .chunks(4)
            .flat_map(|chunk| {
                let group = chunk.iter().fold(0u32, |acc, c| {
                    (acc << 6) | ALPHABET.iter().position(|a| a == c).unwrap() as u32
                });
                let group = group << (6 * (4 - chunk.len()));
                (0..3).map(move |i| (group >> (16 - 8 * i)) as u8)
            })
            .collect();

        decoded[..decoded.len() - padding].to_vec()
    }

    /// The account identifier recovery used by `sts:GetAccessKeyInfo` and by
    /// every offline decoder out there.
    fn account_id_of(access_key_id: &str) -> u64 {
        let bytes = base32_decode(&access_key_id[4..]);
        let header = bytes[..6]
            .iter()
            .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte));
        (header & 0x7fffffffff80) >> 7
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
        assert!(!values_of(&body, "aws_access_key_id").is_empty());
        assert!(!values_of(&body, "aws_secret_access_key").is_empty());
    }

    #[test]
    fn generates_more_than_one_profile() {
        let body = generate();
        assert!(body.matches('[').count() >= 2);
    }

    #[test]
    fn access_key_ids_decode_to_valid_account_ids() {
        for _ in 0..100 {
            for key_id in values_of(&generate(), "aws_access_key_id") {
                assert_eq!(key_id.len(), 20, "unexpected key id: {key_id}");
                assert!(key_id.starts_with("AKIA") || key_id.starts_with("ASIA"));

                let account_id = account_id_of(key_id);
                assert!(
                    (100_000_000_000..=999_999_999_999).contains(&account_id),
                    "{key_id} decoded to {account_id}, which is not a 12 digit account id"
                );
            }
        }
    }

    #[test]
    fn secret_access_keys_are_forty_unpadded_base64_characters() {
        for _ in 0..100 {
            for secret in values_of(&generate(), "aws_secret_access_key") {
                assert_eq!(secret.len(), 40, "unexpected secret: {secret}");
                assert_eq!(base64_decode(secret).len(), 30);
            }
        }
    }

    #[test]
    fn session_tokens_decode_to_a_token_shaped_blob() {
        let mut seen = 0;
        for _ in 0..100 {
            for token in values_of(&generate(), "aws_session_token") {
                seen += 1;
                assert!(token.starts_with("IQoJb3JpZ2luX2Vj"), "unexpected token");

                let decoded = base64_decode(token);
                assert_eq!(decoded[0], 0x21);
                assert_eq!(&decoded[1..12], b"\x0a\x09origin_ec");
                assert!(decoded.len() > 300);
            }
        }
        assert!(seen > 0, "no temporary credentials were generated");
    }

    #[test]
    fn session_tokens_only_accompany_temporary_credentials() {
        for _ in 0..100 {
            let body = generate();

            for profile in body.split('[').skip(1) {
                if profile.contains("aws_session_token = ") {
                    assert!(profile.contains("aws_access_key_id = ASIA"));
                } else {
                    assert!(profile.contains("aws_access_key_id = AKIA"));
                }
            }
        }
    }
}

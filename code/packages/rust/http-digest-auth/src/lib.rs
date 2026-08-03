//! Bounded RFC 7616 HTTP Digest authentication primitives.

#![forbid(unsafe_code)]

use coding_adventures_md5::hex_string as md5_hex;
use coding_adventures_sha256::sha256_hex;
use coding_adventures_zeroize::Zeroizing;
use std::collections::BTreeMap;
use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const MAX_CHALLENGE_BYTES: usize = 8 * 1024;
pub const MAX_DIRECTIVES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestAlgorithm {
    Md5,
    Md5Sess,
    Sha256,
    Sha256Sess,
}

impl DigestAlgorithm {
    fn parse(value: Option<&str>) -> Result<Self, DigestAuthError> {
        match value.unwrap_or("MD5").to_ascii_lowercase().as_str() {
            "md5" => Ok(Self::Md5),
            "md5-sess" => Ok(Self::Md5Sess),
            "sha-256" => Ok(Self::Sha256),
            "sha-256-sess" => Ok(Self::Sha256Sess),
            other => Err(DigestAuthError::UnsupportedAlgorithm(other.to_string())),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Md5Sess => "MD5-sess",
            Self::Sha256 => "SHA-256",
            Self::Sha256Sess => "SHA-256-sess",
        }
    }

    fn session(self) -> bool {
        matches!(self, Self::Md5Sess | Self::Sha256Sess)
    }

    fn hash(self, bytes: &[u8]) -> Zeroizing<String> {
        Zeroizing::new(match self {
            Self::Md5 | Self::Md5Sess => md5_hex(bytes),
            Self::Sha256 | Self::Sha256Sess => sha256_hex(bytes),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestQop {
    Auth,
    Legacy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigestChallenge {
    realm: String,
    nonce: String,
    opaque: Option<String>,
    algorithm: DigestAlgorithm,
    qop: DigestQop,
    stale: bool,
}

impl DigestChallenge {
    pub fn parse(header_value: &str) -> Result<Self, DigestAuthError> {
        if header_value.len() > MAX_CHALLENGE_BYTES {
            return Err(DigestAuthError::ChallengeTooLarge {
                limit: MAX_CHALLENGE_BYTES,
            });
        }
        reject_unsafe("challenge", header_value)?;
        let (scheme, directives) = header_value.trim().split_once(char::is_whitespace).ok_or(
            DigestAuthError::MalformedChallenge("missing Digest directives"),
        )?;
        if !scheme.eq_ignore_ascii_case("Digest") {
            return Err(DigestAuthError::WrongScheme);
        }
        let directives = parse_directives(directives)?;
        let realm = required(&directives, "realm")?.to_string();
        let nonce = required(&directives, "nonce")?.to_string();
        let algorithm = DigestAlgorithm::parse(directives.get("algorithm").map(String::as_str))?;
        let qop = match directives.get("qop") {
            Some(value)
                if value
                    .split(',')
                    .any(|item| item.trim().eq_ignore_ascii_case("auth")) =>
            {
                DigestQop::Auth
            }
            Some(_) => return Err(DigestAuthError::UnsupportedQop),
            None => DigestQop::Legacy,
        };
        if directives
            .get("charset")
            .is_some_and(|value| !value.eq_ignore_ascii_case("UTF-8"))
        {
            return Err(DigestAuthError::UnsupportedCharset);
        }
        if directives
            .get("userhash")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        {
            return Err(DigestAuthError::UnsupportedUserhash);
        }
        let stale = match directives.get("stale") {
            None => false,
            Some(value) if value.eq_ignore_ascii_case("false") => false,
            Some(value) if value.eq_ignore_ascii_case("true") => true,
            Some(_) => return Err(DigestAuthError::MalformedChallenge("invalid stale value")),
        };
        Ok(Self {
            realm,
            nonce,
            opaque: directives.get("opaque").cloned(),
            algorithm,
            qop,
            stale,
        })
    }

    pub fn algorithm(&self) -> DigestAlgorithm {
        self.algorithm
    }

    pub fn qop(&self) -> DigestQop {
        self.qop
    }

    pub fn stale(&self) -> bool {
        self.stale
    }

    pub fn authorization(
        &self,
        username: &str,
        password: &str,
        method: &str,
        uri: &str,
        client_nonce: &str,
        nonce_count: u32,
    ) -> Result<Zeroizing<String>, DigestAuthError> {
        reject_unsafe("username", username)?;
        reject_unsafe("password", password)?;
        reject_unsafe("method", method)?;
        reject_unsafe("uri", uri)?;
        reject_unsafe("client nonce", client_nonce)?;
        if username.is_empty()
            || username.contains(':')
            || method.is_empty()
            || !method.bytes().all(is_token)
            || !uri.starts_with('/')
            || uri.bytes().any(|byte| !byte.is_ascii_graphic())
        {
            return Err(DigestAuthError::InvalidInput(
                "username must be non-empty without colons, method must be an HTTP token, and uri must be ASCII origin-form",
            ));
        }
        if client_nonce.is_empty() || client_nonce.bytes().any(|byte| !byte.is_ascii_graphic()) {
            return Err(DigestAuthError::InvalidInput(
                "client nonce must contain visible ASCII",
            ));
        }
        if self.qop == DigestQop::Auth && nonce_count == 0 {
            return Err(DigestAuthError::InvalidInput(
                "nonce count must be positive for qop=auth",
            ));
        }

        let a1 = Zeroizing::new(format!("{username}:{}:{password}", self.realm));
        let base_ha1 = self.algorithm.hash(a1.as_bytes());
        let ha1 = if self.algorithm.session() {
            let session = Zeroizing::new(format!(
                "{}:{}:{client_nonce}",
                base_ha1.as_str(),
                self.nonce
            ));
            self.algorithm.hash(session.as_bytes())
        } else {
            base_ha1
        };
        let a2 = Zeroizing::new(format!("{method}:{uri}"));
        let ha2 = self.algorithm.hash(a2.as_bytes());
        let nc = format!("{nonce_count:08x}");
        let response_input = match self.qop {
            DigestQop::Auth => Zeroizing::new(format!(
                "{}:{}:{nc}:{client_nonce}:auth:{}",
                ha1.as_str(),
                self.nonce,
                ha2.as_str()
            )),
            DigestQop::Legacy => {
                Zeroizing::new(format!("{}:{}:{}", ha1.as_str(), self.nonce, ha2.as_str()))
            }
        };
        let response = self.algorithm.hash(response_input.as_bytes());

        let quoted_username = Zeroizing::new(quote(username));
        let mut header = format!(
            "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", algorithm={}, response=\"{}\"",
            quoted_username.as_str(),
            quote(&self.realm),
            quote(&self.nonce),
            quote(uri),
            self.algorithm.label(),
            response.as_str(),
        );
        if let Some(opaque) = &self.opaque {
            header.push_str(&format!(", opaque=\"{}\"", quote(opaque)));
        }
        if self.qop == DigestQop::Auth {
            header.push_str(&format!(
                ", qop=auth, nc={nc}, cnonce=\"{}\"",
                quote(client_nonce)
            ));
        }
        Ok(Zeroizing::new(header))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestAuthError {
    WrongScheme,
    ChallengeTooLarge { limit: usize },
    MalformedChallenge(&'static str),
    MissingDirective(&'static str),
    DuplicateDirective(String),
    TooManyDirectives { limit: usize },
    UnsupportedAlgorithm(String),
    UnsupportedQop,
    UnsupportedCharset,
    UnsupportedUserhash,
    UnsafeText(&'static str),
    InvalidInput(&'static str),
}

impl fmt::Display for DigestAuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongScheme => formatter.write_str("authentication challenge is not Digest"),
            Self::ChallengeTooLarge { limit } => {
                write!(formatter, "Digest challenge exceeds {limit} bytes")
            }
            Self::MalformedChallenge(message) => {
                write!(formatter, "malformed Digest challenge: {message}")
            }
            Self::MissingDirective(name) => write!(formatter, "Digest challenge is missing {name}"),
            Self::DuplicateDirective(name) => write!(formatter, "Digest challenge repeats {name}"),
            Self::TooManyDirectives { limit } => {
                write!(formatter, "Digest challenge exceeds {limit} directives")
            }
            Self::UnsupportedAlgorithm(name) => {
                write!(formatter, "unsupported Digest algorithm {name}")
            }
            Self::UnsupportedQop => formatter.write_str("Digest challenge does not offer qop=auth"),
            Self::UnsupportedCharset => {
                formatter.write_str("Digest challenge uses an unsupported charset")
            }
            Self::UnsupportedUserhash => {
                formatter.write_str("Digest userhash=true is not supported")
            }
            Self::UnsafeText(field) => write!(formatter, "{field} contains unsafe HTTP text"),
            Self::InvalidInput(message) => write!(formatter, "invalid Digest input: {message}"),
        }
    }
}

impl std::error::Error for DigestAuthError {}

fn parse_directives(input: &str) -> Result<BTreeMap<String, String>, DigestAuthError> {
    let bytes = input.as_bytes();
    let mut cursor = 0usize;
    let mut directives = BTreeMap::new();
    while cursor < bytes.len() {
        while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b',')
        {
            cursor += 1;
        }
        if cursor == bytes.len() {
            break;
        }
        let name_start = cursor;
        while cursor < bytes.len() && is_token(bytes[cursor]) {
            cursor += 1;
        }
        if cursor == name_start {
            return Err(DigestAuthError::MalformedChallenge(
                "invalid directive name",
            ));
        }
        let name = input[name_start..cursor].to_ascii_lowercase();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            return Err(DigestAuthError::MalformedChallenge(
                "directive is missing equals",
            ));
        }
        cursor += 1;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let value = if bytes.get(cursor) == Some(&b'\"') {
            cursor += 1;
            let mut value = String::new();
            let mut closed = false;
            while cursor < bytes.len() {
                match bytes[cursor] {
                    b'\\' => {
                        cursor += 1;
                        let escaped =
                            *bytes
                                .get(cursor)
                                .ok_or(DigestAuthError::MalformedChallenge(
                                    "truncated quoted escape",
                                ))?;
                        value.push(char::from(escaped));
                        cursor += 1;
                    }
                    b'\"' => {
                        cursor += 1;
                        closed = true;
                        break;
                    }
                    byte if byte.is_ascii() && !byte.is_ascii_control() => {
                        value.push(char::from(byte));
                        cursor += 1;
                    }
                    _ => return Err(DigestAuthError::MalformedChallenge("invalid quoted value")),
                }
            }
            if !closed {
                return Err(DigestAuthError::MalformedChallenge(
                    "unterminated quoted value",
                ));
            }
            value
        } else {
            let value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b',' {
                cursor += 1;
            }
            input[value_start..cursor].trim().to_string()
        };
        if value.is_empty() {
            return Err(DigestAuthError::MalformedChallenge("empty directive value"));
        }
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor] != b',' {
            return Err(DigestAuthError::MalformedChallenge(
                "missing directive separator",
            ));
        }
        if directives.insert(name.clone(), value).is_some() {
            return Err(DigestAuthError::DuplicateDirective(name));
        }
        if directives.len() > MAX_DIRECTIVES {
            return Err(DigestAuthError::TooManyDirectives {
                limit: MAX_DIRECTIVES,
            });
        }
    }
    Ok(directives)
}

fn required<'a>(
    directives: &'a BTreeMap<String, String>,
    name: &'static str,
) -> Result<&'a str, DigestAuthError> {
    directives
        .get(name)
        .map(String::as_str)
        .ok_or(DigestAuthError::MissingDirective(name))
}

fn reject_unsafe(field: &'static str, value: &str) -> Result<(), DigestAuthError> {
    if value.contains(['\r', '\n', '\0']) {
        Err(DigestAuthError::UnsafeText(field))
    } else {
        Ok(())
    }
}

fn is_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

fn quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_md5_example_matches() {
        let challenge = DigestChallenge::parse(
            r#"Digest realm="testrealm@host.com", qop="auth,auth-int", nonce="dcd98b7102dd2f0e8b11d0f600bfb0c093", opaque="5ccc069c403ebaf9f0171e9517f40e41""#,
        )
        .unwrap();
        let header = challenge
            .authorization(
                "Mufasa",
                "Circle Of Life",
                "GET",
                "/dir/index.html",
                "0a4f113b",
                1,
            )
            .unwrap();
        assert!(header.contains("response=\"6629fae49393a05397450978507c4ef1\""));
        assert!(header.contains("qop=auth, nc=00000001, cnonce=\"0a4f113b\""));
    }

    #[test]
    fn published_sha256_example_matches() {
        let challenge = DigestChallenge::parse(
            r#"Digest realm="http-auth@example.org", qop="auth, auth-int", algorithm=SHA-256, nonce="7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v", opaque="FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS""#,
        )
        .unwrap();
        let header = challenge
            .authorization(
                "Mufasa",
                "Circle of Life",
                "GET",
                "/dir/index.html",
                "f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ",
                1,
            )
            .unwrap();
        assert!(header.contains(
            "response=\"753927fa0e85d155564e2e272a28d1802ca10daf4496794697cf8db5856cb6c1\""
        ));
        assert_eq!(challenge.algorithm(), DigestAlgorithm::Sha256);
    }

    #[test]
    fn parses_session_and_legacy_variants() {
        let challenge = DigestChallenge::parse(
            r#"Digest realm="camera", nonce="n\"once", algorithm=MD5-sess, stale=true"#,
        )
        .unwrap();
        assert_eq!(challenge.algorithm(), DigestAlgorithm::Md5Sess);
        assert_eq!(challenge.qop(), DigestQop::Legacy);
        assert!(challenge.stale());
        let header = challenge
            .authorization("root", "secret", "GET", "/status", "client", 0)
            .unwrap();
        assert!(!header.contains("qop="));
        assert!(header.contains("nonce=\"n\\\"once\""));
    }

    #[test]
    fn rejects_ambiguous_or_unsafe_challenges() {
        assert!(matches!(
            DigestChallenge::parse(r#"Digest realm="a", realm="b", nonce="n""#),
            Err(DigestAuthError::DuplicateDirective(name)) if name == "realm"
        ));
        assert!(matches!(
            DigestChallenge::parse(r#"Digest realm="a", nonce="n", qop="auth-int""#),
            Err(DigestAuthError::UnsupportedQop)
        ));
        assert!(matches!(
            DigestChallenge::parse("Digest realm=\"a\r\nb\", nonce=\"n\""),
            Err(DigestAuthError::UnsafeText("challenge"))
        ));
        assert!(matches!(
            DigestChallenge::parse(r#"Basic realm="a""#),
            Err(DigestAuthError::WrongScheme)
        ));
    }

    #[test]
    fn authorization_rejects_injection_and_zero_nonce_count() {
        let challenge =
            DigestChallenge::parse(r#"Digest realm="camera", nonce="nonce", qop="auth""#).unwrap();
        assert!(challenge
            .authorization("root", "secret", "GET\r\nInjected", "/", "c", 1)
            .is_err());
        assert!(challenge
            .authorization("root", "secret", "GET", "/", "c", 0)
            .is_err());
    }
}

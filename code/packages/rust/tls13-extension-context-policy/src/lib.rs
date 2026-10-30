//! Allocation-free TLS 1.3 extension-context policy for a future client.
//!
//! TLS extension syntax is intentionally separate from TLS extension policy.
//! [`tls13_extension_block`] already proves that one borrowed extension vector
//! is structurally exact and duplicate-free. This package consumes that typed
//! result and answers the next two questions:
//!
//! 1. May each recognized extension appear in this handshake message?
//! 2. When a server extension is a response, did the client offer it first?
//!
//! The RFC 8446 Table 1 registry below is ordinary data. Callers may instead
//! construct a narrower policy from their own bounded rule slice. Neither path
//! decodes extension data or enables an extension's semantics.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_extension_block::{ExtensionBlock, ExtensionType};

/// Maximum number of caller-owned context rules inspected by one policy.
pub const MAX_EXTENSION_RULES: usize = 64;

/// RFC 8446 `pre_shared_key` extension type.
pub const PRE_SHARED_KEY_EXTENSION_TYPE: u16 = 41;

/// RFC 8446 `cookie` extension type.
pub const COOKIE_EXTENSION_TYPE: u16 = 44;

/// One handshake-message context from RFC 8446 Table 1.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ExtensionContext {
    /// ClientHello (`CH`). This package uses it only for the future client's
    /// outbound offer.
    ClientHello = 0,
    /// ServerHello (`SH`).
    ServerHello = 1,
    /// EncryptedExtensions (`EE`).
    EncryptedExtensions = 2,
    /// Server Certificate entry extensions (`CT`).
    Certificate = 3,
    /// Server CertificateRequest (`CR`).
    CertificateRequest = 4,
    /// Server NewSessionTicket (`NST`).
    NewSessionTicket = 5,
    /// HelloRetryRequest (`HRR`), which uses the ServerHello wire shape.
    HelloRetryRequest = 6,
}

/// A compact non-empty-or-empty set of extension contexts.
///
/// Construction exposes only named contexts, so callers cannot manufacture
/// unknown bits. An empty set is representable so [`ExtensionContextPolicy`]
/// can reject an incomplete caller-owned rule with a stable error.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ExtensionContextSet(u8);

impl ExtensionContextSet {
    /// Construct the empty context set.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Construct a set containing one context.
    #[must_use]
    pub const fn of(context: ExtensionContext) -> Self {
        Self(1_u8 << context as u8)
    }

    /// Return a copy of this set with one context inserted.
    #[must_use]
    pub const fn with(self, context: ExtensionContext) -> Self {
        Self(self.0 | (1_u8 << context as u8))
    }

    /// Return the union of two context sets.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Return whether this set contains one context.
    #[must_use]
    pub const fn contains(self, context: ExtensionContext) -> bool {
        self.0 & (1_u8 << context as u8) != 0
    }

    /// Return whether this set contains no contexts.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for ExtensionContextSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExtensionContextSet")
            .field(
                "client_hello",
                &self.contains(ExtensionContext::ClientHello),
            )
            .field(
                "server_hello",
                &self.contains(ExtensionContext::ServerHello),
            )
            .field(
                "encrypted_extensions",
                &self.contains(ExtensionContext::EncryptedExtensions),
            )
            .field("certificate", &self.contains(ExtensionContext::Certificate))
            .field(
                "certificate_request",
                &self.contains(ExtensionContext::CertificateRequest),
            )
            .field(
                "new_session_ticket",
                &self.contains(ExtensionContext::NewSessionTicket),
            )
            .field(
                "hello_retry_request",
                &self.contains(ExtensionContext::HelloRetryRequest),
            )
            .finish()
    }
}

/// One data-driven extension-type-to-context rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtensionContextRule {
    extension_type: ExtensionType,
    contexts: ExtensionContextSet,
}

impl ExtensionContextRule {
    /// Construct one rule from an exact extension-type wire value and context
    /// set. The enclosing policy rejects an empty context set.
    #[must_use]
    pub const fn new(extension_type: u16, contexts: ExtensionContextSet) -> Self {
        Self {
            extension_type: ExtensionType::from_wire_value(extension_type),
            contexts,
        }
    }

    /// Return the exact extension type governed by this rule.
    #[must_use]
    pub const fn extension_type(self) -> ExtensionType {
        self.extension_type
    }

    /// Return the contexts authorized by this rule.
    #[must_use]
    pub const fn contexts(self) -> ExtensionContextSet {
        self.contexts
    }

    /// Return whether this rule authorizes one context.
    #[must_use]
    pub const fn allows(self, context: ExtensionContext) -> bool {
        self.contexts.contains(context)
    }
}

const CH: ExtensionContextSet = ExtensionContextSet::of(ExtensionContext::ClientHello);
const SH: ExtensionContextSet = ExtensionContextSet::of(ExtensionContext::ServerHello);
const EE: ExtensionContextSet = ExtensionContextSet::of(ExtensionContext::EncryptedExtensions);
const CT: ExtensionContextSet = ExtensionContextSet::of(ExtensionContext::Certificate);
const CR: ExtensionContextSet = ExtensionContextSet::of(ExtensionContext::CertificateRequest);
const NST: ExtensionContextSet = ExtensionContextSet::of(ExtensionContext::NewSessionTicket);
const HRR: ExtensionContextSet = ExtensionContextSet::of(ExtensionContext::HelloRetryRequest);

/// The complete 22-row TLS 1.3 placement table from RFC 8446 section 4.2.
///
/// Recognition here proves only that a type has a placement rule. It does not
/// mean this repository implements, enables, or semantically validates that
/// extension.
pub const STANDARD_TLS13_EXTENSION_RULES: [ExtensionContextRule; 22] = [
    ExtensionContextRule::new(0, CH.union(EE)),
    ExtensionContextRule::new(1, CH.union(EE)),
    ExtensionContextRule::new(5, CH.union(CR).union(CT)),
    ExtensionContextRule::new(10, CH.union(EE)),
    ExtensionContextRule::new(13, CH.union(CR)),
    ExtensionContextRule::new(14, CH.union(EE)),
    ExtensionContextRule::new(15, CH.union(EE)),
    ExtensionContextRule::new(16, CH.union(EE)),
    ExtensionContextRule::new(18, CH.union(CR).union(CT)),
    ExtensionContextRule::new(19, CH.union(EE)),
    ExtensionContextRule::new(20, CH.union(EE)),
    ExtensionContextRule::new(21, CH),
    ExtensionContextRule::new(51, CH.union(SH).union(HRR)),
    ExtensionContextRule::new(41, CH.union(SH)),
    ExtensionContextRule::new(45, CH),
    ExtensionContextRule::new(42, CH.union(EE).union(NST)),
    ExtensionContextRule::new(44, CH.union(HRR)),
    ExtensionContextRule::new(43, CH.union(SH).union(HRR)),
    ExtensionContextRule::new(47, CH.union(CR)),
    ExtensionContextRule::new(48, CR),
    ExtensionContextRule::new(49, CH),
    ExtensionContextRule::new(50, CH.union(CR)),
];

/// Server-originated message contexts handled by a future TLS client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServerExtensionContext {
    /// ServerHello extension block.
    ServerHello,
    /// EncryptedExtensions extension block.
    EncryptedExtensions,
    /// HelloRetryRequest extension block.
    HelloRetryRequest,
    /// Extensions on entries in the server Certificate message.
    Certificate,
    /// Server CertificateRequest extension block.
    CertificateRequest,
    /// Server NewSessionTicket extension block.
    NewSessionTicket,
}

impl ServerExtensionContext {
    const fn placement(self) -> ExtensionContext {
        match self {
            Self::ServerHello => ExtensionContext::ServerHello,
            Self::EncryptedExtensions => ExtensionContext::EncryptedExtensions,
            Self::HelloRetryRequest => ExtensionContext::HelloRetryRequest,
            Self::Certificate => ExtensionContext::Certificate,
            Self::CertificateRequest => ExtensionContext::CertificateRequest,
            Self::NewSessionTicket => ExtensionContext::NewSessionTicket,
        }
    }

    const fn is_response(self) -> bool {
        matches!(
            self,
            Self::ServerHello
                | Self::EncryptedExtensions
                | Self::HelloRetryRequest
                | Self::Certificate
        )
    }
}

/// Stable payload-free failures for rule construction and context validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensionContextPolicyError {
    /// The caller supplied more than [`MAX_EXTENSION_RULES`] rules.
    TooManyRules,
    /// One rule authorized no message context.
    EmptyRuleContexts,
    /// More than one rule governed the same extension type.
    DuplicateRule,
    /// The selected policy had no rule for one extension type.
    UnrecognizedExtension,
    /// A recognized extension appeared in a message its rule did not allow.
    ForbiddenContext,
    /// A response extension did not match a ClientHello offer.
    UnsolicitedExtension,
    /// ClientHello carried `pre_shared_key` before another extension.
    PreSharedKeyNotLast,
}

impl fmt::Display for ExtensionContextPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooManyRules => "TLS extension context policy has too many rules",
            Self::EmptyRuleContexts => "TLS extension context rule has no contexts",
            Self::DuplicateRule => "TLS extension context policy has a duplicate rule",
            Self::UnrecognizedExtension => "unrecognized TLS extension",
            Self::ForbiddenContext => "TLS extension is forbidden in this message context",
            Self::UnsolicitedExtension => "TLS server extension was not offered by the client",
            Self::PreSharedKeyNotLast => "TLS pre-shared-key extension is not last",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ExtensionContextPolicyError {}

/// A validated borrowed extension-context rule table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExtensionContextPolicy<'rules> {
    rules: &'rules [ExtensionContextRule],
}

impl<'rules> ExtensionContextPolicy<'rules> {
    /// Validate caller-owned rules without allocation.
    pub fn new(rules: &'rules [ExtensionContextRule]) -> Result<Self, ExtensionContextPolicyError> {
        if rules.len() > MAX_EXTENSION_RULES {
            return Err(ExtensionContextPolicyError::TooManyRules);
        }

        for (index, rule) in rules.iter().copied().enumerate() {
            if rule.contexts.is_empty() {
                return Err(ExtensionContextPolicyError::EmptyRuleContexts);
            }
            if rules[..index]
                .iter()
                .any(|earlier| earlier.extension_type == rule.extension_type)
            {
                return Err(ExtensionContextPolicyError::DuplicateRule);
            }
        }

        Ok(Self { rules })
    }

    /// Return the number of validated rules.
    #[must_use]
    pub const fn len(self) -> usize {
        self.rules.len()
    }

    /// Return whether this fail-closed policy recognizes no extensions.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.rules.is_empty()
    }

    /// Validate the future client's outbound ClientHello extension block.
    ///
    /// The structural decoder has already rejected duplicates. This layer
    /// additionally applies the selected context table and the TLS 1.3 rule
    /// that `pre_shared_key`, if present, is the final ClientHello extension.
    pub fn validate_client_hello(
        self,
        block: &ExtensionBlock<'_>,
    ) -> Result<(), ExtensionContextPolicyError> {
        self.validate_placement(block, ExtensionContext::ClientHello)?;

        let pre_shared_key = ExtensionType::from_wire_value(PRE_SHARED_KEY_EXTENSION_TYPE);
        if block.find(pre_shared_key).is_some()
            && block
                .entries()
                .last()
                .is_some_and(|entry| entry.extension_type() != pre_shared_key)
        {
            return Err(ExtensionContextPolicyError::PreSharedKeyNotLast);
        }
        Ok(())
    }

    /// Validate one server-originated extension block for a future TLS client.
    ///
    /// Response contexts bind every extension type back to the already
    /// validated ClientHello offer. RFC 8446's sole generic exception is an
    /// unsolicited `cookie` in HelloRetryRequest. CertificateRequest is a
    /// server request and NewSessionTicket is unsolicited by definition, so
    /// those contexts apply placement rules without offer binding.
    pub fn validate_server_message(
        self,
        context: ServerExtensionContext,
        server_block: &ExtensionBlock<'_>,
        client_hello: &ExtensionBlock<'_>,
    ) -> Result<(), ExtensionContextPolicyError> {
        self.validate_client_hello(client_hello)?;
        self.validate_placement(server_block, context.placement())?;

        if context.is_response() {
            let cookie = ExtensionType::from_wire_value(COOKIE_EXTENSION_TYPE);
            for entry in server_block.entries() {
                let extension_type = entry.extension_type();
                let is_retry_cookie = context == ServerExtensionContext::HelloRetryRequest
                    && extension_type == cookie;
                if !is_retry_cookie && client_hello.find(extension_type).is_none() {
                    return Err(ExtensionContextPolicyError::UnsolicitedExtension);
                }
            }
        }
        Ok(())
    }

    fn validate_placement(
        self,
        block: &ExtensionBlock<'_>,
        context: ExtensionContext,
    ) -> Result<(), ExtensionContextPolicyError> {
        for entry in block.entries() {
            let Some(rule) = self
                .rules
                .iter()
                .copied()
                .find(|rule| rule.extension_type == entry.extension_type())
            else {
                return Err(ExtensionContextPolicyError::UnrecognizedExtension);
            };
            if !rule.allows(context) {
                return Err(ExtensionContextPolicyError::ForbiddenContext);
            }
        }
        Ok(())
    }
}

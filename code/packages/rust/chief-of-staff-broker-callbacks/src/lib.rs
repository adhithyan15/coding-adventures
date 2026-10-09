//! # The daemon's side of a broker's callbacks (D18S S-K7, step 6 P2.6d-1)
//!
//! A broker holds one agent's channel keys. Storage stays in the daemon. While
//! the broker serves one of its agent's requests, it asks the daemon for
//! exactly the storage it needs, through the eight callbacks of
//! `chief-of-staff-broker-protocol`. This crate answers them. It holds no key,
//! and it treats every callback as hostile: the broker is the process this
//! design expects to be compromised, and a callback is its only reach into
//! shared state.
//!
//! ## Two kinds of "no"
//!
//! ```text
//!   callback ──► [1] fits the request in flight?  ── no ──► Violation: end the broker
//!                    right request, same channel,
//!                    allowed op, budget left,
//!                    one reserve, then one commit
//!                    or abandon
//!                        │ yes
//!                        ▼
//!                [2] authorized right now?         ── no ──► Refused(code): an answer
//!                    binding re-resolved: channel
//!                    bound, right direction; the
//!                    definition is current, active,
//!                    and names this agent
//!                        │ yes
//!                        ▼
//!                [3] the operation, with key-free checks on what it stores
//! ```
//!
//! A **violation** is something an honest broker never does. The caller ends
//! the broker, and with it the agent. A **refusal** is an ordinary answer,
//! such as a definition that changed or an append already pending, and goes
//! back to the broker, which maps it to the host's failure code.
//!
//! ## Identity comes from the pipe
//!
//! No callback says who is asking. The [`BindingResolver`] the caller
//! supplies is tied to the one broker it serves, and is asked again on every
//! callback, so unwiring a pipeline revokes a running broker at its next
//! callback (S-K2).
//!
//! ## What is checked without keys
//!
//! - **Grants:** the signature under the definition's originator key; that
//!   the originator is this agent; that the epoch is the definition's; that
//!   the receiver is in the definition. Storage is write-once, so a stored
//!   grant is never replaced.
//! - **Messages:** the signature under the definition's originator key, and
//!   that the header is byte for byte the one this daemon reserved, with the
//!   message id, timestamp, originator and epoch it filled in itself.
//!
//! Not checked, because it needs the channel key: whether the ciphertext
//! decrypts. D18S P2.6d records what follows from that.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use chief_of_staff_broker_protocol::{
    definition_digest, Callback, CallbackOp, CallbackOutcome, CallbackReply, Refusal,
    MAX_CALLBACKS_PER_REQUEST, MAX_PAGE_BYTES, MAX_PAGE_MESSAGES, MAX_PAGE_MESSAGE_BYTES,
};
use chief_of_staff_channel_crypto::wire::{encode_key_grant, encode_message, ChannelWireError};
use chief_of_staff_channel_crypto::{
    verify_channel_key_grant_signature, ChannelCryptoError, ChannelId, SealedChannelKeyGrant,
    Sequence,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, ChannelEndpointError, ChannelLifecycle,
    MessageMetadataSource,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelStore, ChannelStoreError};
use chief_of_staff_host_control_protocol::{ChannelBindingAccess, DataPlaneRequest, RequestId};
use chief_of_staff_pipeline_bindings::HostPipelineBinding;
use storage_core::StorageBackend;

/// Where the server learns, on every callback, whom its broker serves.
///
/// One resolver per broker. `None` means the binding is gone (the pipeline
/// was unwired, or the host deregistered), and every callback is then refused
/// as `Unauthorized`.
pub trait BindingResolver {
    /// The broker's current binding, freshly resolved.
    fn current_binding(&self) -> Option<HostPipelineBinding>;
}

/// A callback an honest broker never makes. The caller must end the broker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Violation {
    /// No channel request is in flight, or the callback names another.
    NotInFlight,
    /// The callback names a channel other than the request's.
    WrongChannel,
    /// The operation does not belong to this kind of request.
    OperationNotAllowed,
    /// More than [`MAX_CALLBACKS_PER_REQUEST`] callbacks.
    BudgetExhausted,
    /// A second reservation, a commit or abandon without one, or anything
    /// after the append was closed.
    AppendOutOfOrder,
    /// A page larger than the request asked for.
    PageTooLarge,
}

/// The channel request a broker is serving, and what it has done so far.
///
/// Built by [`InFlight::for_request`] when the supervisor relays a request,
/// and passed to every [`CallbackServer::serve`] for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InFlight {
    request_id: RequestId,
    kind: RequestKind,
    channel_id: ChannelId,
    callbacks: u32,
    append: Append,
    grants_loaded: bool,
    grants_saved: bool,
}

/// Where a Publish is in its one append:
///
/// ```text
///   None ──ReserveAppend──► Reserved(s) ──CommitAppend(s)──► Committing(s) ──ok──► Closed
///                               │                                │
///                               └──────AbandonAppend(s)──────────┴──────────────► Closed
/// ```
///
/// A refused commit leaves `Committing(s)`, from which the broker may still
/// give the reservation back. Otherwise the channel would stay stuck behind
/// it until the broker is replaced. Both arrows into `Closed` are taken only
/// once the store has done the work, so a refused abandon can be retried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Append {
    None,
    Reserved(Sequence),
    Committing(Sequence),
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestKind {
    Receive { limit: u16 },
    Publish,
    Acknowledge,
}

impl InFlight {
    /// The scope for one relayed request. `None` for a request that is not a
    /// channel operation: those make no callbacks at all.
    pub fn for_request(request: &DataPlaneRequest) -> Option<Self> {
        let (kind, channel) = match request {
            DataPlaneRequest::Receive {
                channel_id, limit, ..
            } => (RequestKind::Receive { limit: *limit }, channel_id),
            DataPlaneRequest::Publish { channel_id, .. } => (RequestKind::Publish, channel_id),
            DataPlaneRequest::Acknowledge { channel_id, .. } => {
                (RequestKind::Acknowledge, channel_id)
            }
            _ => return None,
        };
        Some(Self {
            request_id: request.id(),
            kind,
            channel_id: ChannelId(*channel),
            callbacks: 0,
            append: Append::None,
            grants_loaded: false,
            grants_saved: false,
        })
    }

    /// The request this scope belongs to.
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// The reservation a Publish made and neither committed nor abandoned.
    /// The caller abandons it if the broker ends mid-request.
    pub fn open_reservation(&self) -> Option<Sequence> {
        match self.append {
            Append::Reserved(sequence) | Append::Committing(sequence) => Some(sequence),
            Append::None | Append::Closed => None,
        }
    }

    /// Step [1]: does `call` fit this request at all?
    fn admit(&mut self, request_id: RequestId, call: &Callback) -> Result<(), Violation> {
        if request_id != self.request_id {
            return Err(Violation::NotInFlight);
        }
        if call.channel_id() != self.channel_id {
            return Err(Violation::WrongChannel);
        }
        let op = call.op();
        let allowed = match self.kind {
            RequestKind::Receive { .. } => {
                matches!(
                    op,
                    CallbackOp::LoadDefinition | CallbackOp::ReadReceiverPage
                )
            }
            RequestKind::Acknowledge => {
                matches!(op, CallbackOp::LoadDefinition | CallbackOp::Acknowledge)
            }
            RequestKind::Publish => {
                !matches!(op, CallbackOp::ReadReceiverPage | CallbackOp::Acknowledge)
            }
        };
        if !allowed {
            return Err(Violation::OperationNotAllowed);
        }
        if self.callbacks >= MAX_CALLBACKS_PER_REQUEST {
            return Err(Violation::BudgetExhausted);
        }
        self.callbacks += 1;
        if let (RequestKind::Receive { limit }, Callback::ReadReceiverPage { limit: asked, .. }) =
            (self.kind, call)
        {
            if *asked > limit {
                return Err(Violation::PageTooLarge);
            }
        }
        // The append: grant work first, each step at most once; then one
        // reservation; then one commit attempt and, at most once, giving
        // the reservation back.
        let next = match (call, self.append) {
            (Callback::LoadMissingGrants { .. }, Append::None) if !self.grants_loaded => {
                self.grants_loaded = true;
                self.append
            }
            (Callback::SaveGrants { .. }, Append::None) if !self.grants_saved => {
                self.grants_saved = true;
                self.append
            }
            (Callback::LoadMissingGrants { .. } | Callback::SaveGrants { .. }, _) => {
                return Err(Violation::AppendOutOfOrder)
            }
            // The server records the reservation once it is made.
            (Callback::ReserveAppend { .. }, Append::None) => self.append,
            (Callback::ReserveAppend { .. }, _) => return Err(Violation::AppendOutOfOrder),
            (Callback::CommitAppend { message, .. }, Append::Reserved(sequence))
                if message.header().fields().sequence() == sequence =>
            {
                Append::Committing(sequence)
            }
            (Callback::CommitAppend { .. }, _) => return Err(Violation::AppendOutOfOrder),
            // Closed only once the store has abandoned it; a refused
            // abandon may be retried, within the request's budget.
            (
                Callback::AbandonAppend { sequence, .. },
                Append::Reserved(reserved) | Append::Committing(reserved),
            ) if reserved.0 == *sequence => self.append,
            (Callback::AbandonAppend { .. }, _) => return Err(Violation::AppendOutOfOrder),
            _ => self.append,
        };
        self.append = next;
        Ok(())
    }
}

/// Answers one broker's callbacks against shared storage. Holds no keys.
pub struct CallbackServer<'a> {
    backend: &'a dyn StorageBackend,
    metadata: &'a dyn MessageMetadataSource,
    binding: &'a dyn BindingResolver,
}

/// Where a callback sits relative to the channel: as a receiver of it, or
/// as its originator.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Receiver,
    Originator,
}

impl<'a> CallbackServer<'a> {
    /// A server for the one broker `binding` resolves.
    pub fn new(
        backend: &'a dyn StorageBackend,
        metadata: &'a dyn MessageMetadataSource,
        binding: &'a dyn BindingResolver,
    ) -> Self {
        Self {
            backend,
            metadata,
            binding,
        }
    }

    /// Serve one callback for the request in `in_flight`.
    ///
    /// `Err` is a [`Violation`]: end the broker. `Ok` is the outcome to send
    /// back, an answer or a refusal.
    pub fn serve(
        &self,
        in_flight: &mut InFlight,
        request_id: RequestId,
        call: Callback,
    ) -> Result<CallbackOutcome, Violation> {
        in_flight.admit(request_id, &call)?;
        let op = call.op();
        let outcome = match self.authorized(&call) {
            Ok((agent, definition)) => self.perform(in_flight, &agent, &definition, call),
            Err(refusal) => Err(refusal),
        };
        Ok(match outcome {
            Ok(reply) => CallbackOutcome::Ok(reply),
            Err(refusal) => CallbackOutcome::Refused { op, refusal },
        })
    }

    /// Step [2]: is this broker's agent, right now, allowed this channel in
    /// this direction, under a current, active definition that names it?
    fn authorized(&self, call: &Callback) -> Result<(AgentId, ChannelDefinition), Refusal> {
        let role = match call.op() {
            CallbackOp::LoadDefinition => None,
            CallbackOp::ReadReceiverPage | CallbackOp::Acknowledge => Some(Role::Receiver),
            _ => Some(Role::Originator),
        };
        let binding = self
            .binding
            .current_binding()
            .ok_or(Refusal::Unauthorized)?;
        let channel = call.channel_id();
        let access = binding
            .launch_bindings()
            .channels()
            .iter()
            .find(|bound| bound.channel_id() == channel.0)
            .map(|bound| bound.access())
            .ok_or(Refusal::Unauthorized)?;
        let direction_ok = match role {
            None => true,
            Some(Role::Receiver) => access == ChannelBindingAccess::Read,
            Some(Role::Originator) => access == ChannelBindingAccess::Write,
        };
        if !direction_ok {
            return Err(Refusal::Unauthorized);
        }

        let definition = ChannelDefinitionStore::new(self.backend)
            .load(channel)
            .map_err(endpoint_refusal)?
            .ok_or(Refusal::NotFound)?;
        if definition.lifecycle() != ChannelLifecycle::Active {
            return Err(Refusal::ChannelDestroyed);
        }
        let agent = binding.agent_id().clone();
        let member = match access {
            ChannelBindingAccess::Read => definition.receiver(&agent).is_some(),
            ChannelBindingAccess::Write => definition.originator().agent_id == agent,
        };
        if !member {
            return Err(Refusal::Unauthorized);
        }
        if let Some(digest) = digest_of(call) {
            if digest != definition_digest(&definition) {
                return Err(Refusal::DefinitionChanged);
            }
        }
        Ok((agent, definition))
    }

    /// Step [3]: the operation itself.
    fn perform(
        &self,
        in_flight: &mut InFlight,
        agent: &AgentId,
        definition: &ChannelDefinition,
        call: Callback,
    ) -> Result<CallbackReply, Refusal> {
        let store = ChannelStore::new(self.backend, definition.channel_id());
        match call {
            Callback::LoadDefinition { .. } => Ok(CallbackReply::Definition(definition.clone())),
            Callback::ReadReceiverPage { limit, .. } => {
                self.read_page(&store, agent, usize::from(limit))
            }
            Callback::Acknowledge { sequence, .. } => {
                store.initialize().map_err(store_refusal)?;
                let first_unread = store
                    .acknowledge(agent.as_bytes(), Sequence(sequence))
                    .map_err(store_refusal)?;
                Ok(CallbackReply::Acknowledged {
                    first_unread: first_unread.0,
                })
            }
            Callback::LoadMissingGrants { .. } => {
                let epoch = definition.key_epoch();
                let mut receivers = Vec::new();
                for (index, receiver) in definition.receivers().iter().enumerate() {
                    if store
                        .key_grant(epoch, receiver.agent_id.as_bytes())
                        .map_err(store_refusal)?
                        .is_none()
                    {
                        receivers.push(u16::try_from(index).map_err(|_| Refusal::Corrupt)?);
                    }
                }
                Ok(CallbackReply::MissingGrants {
                    key_epoch: epoch.0,
                    receivers,
                })
            }
            Callback::SaveGrants { grants, .. } => {
                // Check every grant before storing any, so a bad one in the
                // middle leaves nothing half-saved.
                let mut seen = BTreeSet::new();
                for grant in &grants {
                    check_grant(grant, agent, definition)?;
                    if !seen.insert(grant.receiver_id.clone()) {
                        return Err(Refusal::InvalidRequest);
                    }
                }
                for grant in &grants {
                    store.save_key_grant(grant).map_err(store_refusal)?;
                }
                Ok(CallbackReply::GrantsSaved)
            }
            Callback::ReserveAppend {
                content_type,
                plaintext_hash,
                ..
            } => {
                let metadata = self
                    .metadata
                    .next_metadata()
                    .map_err(|_| Refusal::Unavailable)?;
                store.initialize().map_err(store_refusal)?;
                let header = store
                    .reserve_append_with_hash(
                        AppendRequest {
                            message_id: *metadata.message_id.as_bytes(),
                            timestamp_ns: metadata.timestamp_ns,
                            originator_id: agent.as_bytes().to_vec(),
                            key_epoch: definition.key_epoch(),
                            content_type,
                        },
                        plaintext_hash,
                    )
                    .map_err(store_refusal)?;
                in_flight.append = Append::Reserved(header.fields().sequence());
                Ok(CallbackReply::Reserved(header))
            }
            Callback::CommitAppend { message, .. } => {
                // The store refuses any header but the pending one, which
                // this daemon minted; the signature must be the
                // definition's originator's. Refused, the reservation stays
                // open for the broker to abandon.
                let committed = store
                    .commit_encrypted(&message, &definition.originator().public_key)
                    .map_err(store_refusal)?;
                in_flight.append = Append::Closed;
                Ok(CallbackReply::Committed {
                    sequence: committed.header().fields().sequence().0,
                })
            }
            Callback::AbandonAppend { sequence, .. } => {
                let abandoned = store
                    .abandon_pending_at(Sequence(sequence))
                    .map_err(store_refusal)?;
                in_flight.append = Append::Closed;
                Ok(CallbackReply::Abandoned { abandoned })
            }
        }
    }

    /// Whole messages after the receiver's cursor, up to `limit` and
    /// [`MAX_PAGE_BYTES`], with the receiver's grant for each epoch among
    /// them.
    fn read_page(
        &self,
        store: &ChannelStore<'_>,
        agent: &AgentId,
        limit: usize,
    ) -> Result<CallbackReply, Refusal> {
        // As a receiver's endpoint does today: a channel nobody has written
        // to yet reads as empty, not as missing.
        store.initialize().map_err(store_refusal)?;
        let first_unread = store
            .receiver_cursor(agent.as_bytes())
            .map_err(store_refusal)?;
        let page = store
            .read_for_receiver(agent.as_bytes(), limit.min(MAX_PAGE_MESSAGES))
            .map_err(store_refusal)?;
        // Whole messages, while both budgets hold: the messages alone must
        // decrypt into a response the host can take, and messages plus the
        // grants they need must fit one frame.
        let mut messages = Vec::new();
        let mut grants: Vec<SealedChannelKeyGrant> = Vec::new();
        let mut epochs = BTreeSet::new();
        let mut message_bytes = 0usize;
        let mut page_bytes = 0usize;
        for message in page.messages {
            let size = encode_message(&message).map_err(wire_refusal)?.len();
            let epoch = message.header().fields().key_epoch();
            let mut grant = None;
            let mut grant_size = 0;
            if !epochs.contains(&epoch) {
                grant = store
                    .key_grant(epoch, agent.as_bytes())
                    .map_err(store_refusal)?;
                if let Some(grant) = &grant {
                    grant_size = encode_key_grant(grant).map_err(wire_refusal)?.len();
                }
            }
            if message_bytes + size > MAX_PAGE_MESSAGE_BYTES
                || page_bytes + size + grant_size > MAX_PAGE_BYTES
            {
                if messages.is_empty() {
                    // One message larger than any page: it can never be
                    // delivered this way, and is refused, not skipped.
                    return Err(Refusal::TooLarge);
                }
                break;
            }
            message_bytes += size;
            page_bytes += size + grant_size;
            epochs.insert(epoch);
            grants.extend(grant);
            messages.push(message);
        }
        Ok(CallbackReply::ReceiverPage {
            first_unread: first_unread.0,
            messages,
            grants,
        })
    }
}

/// A grant may be stored only if this agent, as the definition's originator,
/// signed it, for this channel at the current epoch, for a receiver in the
/// definition.
fn check_grant(
    grant: &SealedChannelKeyGrant,
    agent: &AgentId,
    definition: &ChannelDefinition,
) -> Result<(), Refusal> {
    if grant.key_epoch != definition.key_epoch() {
        return Err(Refusal::InvalidRequest);
    }
    let receiver_id =
        AgentId::new(grant.receiver_id.clone()).map_err(|_| Refusal::InvalidRequest)?;
    if definition.receiver(&receiver_id).is_none() {
        return Err(Refusal::InvalidRequest);
    }
    verify_channel_key_grant_signature(
        grant,
        agent.as_bytes(),
        &grant.receiver_id,
        definition.channel_id(),
        &definition.originator().public_key,
    )
    .map_err(|_| Refusal::InvalidRequest)
}

fn digest_of(call: &Callback) -> Option<[u8; 32]> {
    match call {
        Callback::LoadDefinition { .. } | Callback::AbandonAppend { .. } => None,
        Callback::ReadReceiverPage {
            definition_digest, ..
        }
        | Callback::Acknowledge {
            definition_digest, ..
        }
        | Callback::LoadMissingGrants {
            definition_digest, ..
        }
        | Callback::SaveGrants {
            definition_digest, ..
        }
        | Callback::ReserveAppend {
            definition_digest, ..
        }
        | Callback::CommitAppend {
            definition_digest, ..
        } => Some(*definition_digest),
    }
}

fn endpoint_refusal(error: ChannelEndpointError) -> Refusal {
    match error {
        ChannelEndpointError::Storage(_) | ChannelEndpointError::ConcurrentUpdate => {
            Refusal::Unavailable
        }
        _ => Refusal::Corrupt,
    }
}

fn wire_refusal(_: ChannelWireError) -> Refusal {
    Refusal::Corrupt
}

fn store_refusal(error: ChannelStoreError) -> Refusal {
    match error {
        ChannelStoreError::Storage(_) | ChannelStoreError::ConcurrentUpdate => Refusal::Unavailable,
        ChannelStoreError::PendingAppend(_) => Refusal::PendingAppend,
        ChannelStoreError::ConflictingRecord(_) => Refusal::Conflict,
        ChannelStoreError::NotInitialized => Refusal::NotFound,
        ChannelStoreError::PendingHeaderMismatch
        | ChannelStoreError::NoPendingAppend
        | ChannelStoreError::InvalidReceiverId
        | ChannelStoreError::InvalidPageSize
        | ChannelStoreError::AcknowledgementRegression { .. }
        | ChannelStoreError::Crypto(ChannelCryptoError::InvalidMessageSignature) => {
            Refusal::InvalidRequest
        }
        _ => Refusal::Corrupt,
    }
}

//! # Actor -- Primitives for Concurrent Computation
//!
//! This crate implements the three foundational primitives of the Actor model,
//! the mathematical framework for concurrent computation invented by Carl Hewitt,
//! Peter Bishop, and Richard Steiger in 1973.
//!
//! ## The Actor Model
//!
//! The key insight: an actor is defined by what it **can do**, not what it **is**.
//! An actor can:
//!
//! 1. **Receive** a message
//! 2. **Send** messages to other actors it knows about
//! 3. **Create** new actors
//! 4. **Change its own internal state** in response to a message
//!
//! An actor **cannot**:
//!
//! 1. Access another actor's internal state
//! 2. Share memory with another actor
//! 3. Communicate except through messages
//!
//! ## Three Primitives
//!
//! ```text
//! Message   -- the atom of communication. Immutable, typed, serializable.
//! Channel   -- a one-way, append-only pipe for messages. Persistent, replayable.
//! Actor     -- an isolated unit of computation with a mailbox and private state.
//! ```
//!
//! Plus `ActorSystem`, the runtime that manages actor lifecycles, message
//! delivery, and channels.
//!
//! ## Quick Start
//!
//! ```
//! use actor::{Message, Channel, Actor, ActorSystem, ActorResult};
//!
//! let mut system = ActorSystem::new();
//!
//! // Create a counter actor
//! system.create_actor("counter", Box::new(0_u64), Box::new(|state, _msg| {
//!     let count = *state.downcast::<u64>().unwrap();
//!     ActorResult {
//!         new_state: Box::new(count + 1),
//!         messages_to_send: vec![],
//!         actors_to_create: vec![],
//!         stop: false,
//!     }
//! })).unwrap();
//!
//! // Send a message
//! system.send("counter", Message::text("user", "tick")).unwrap();
//!
//! // Process it
//! system.process_next("counter").unwrap();
//! ```

pub mod message;
pub mod channel;
pub mod actor;
pub mod actor_system;

// Re-export the main types at the crate root for convenient access.
pub use message::{ActorError, Message, WIRE_MAGIC, WIRE_VERSION, HEADER_SIZE};
pub use channel::Channel;
pub use actor::{Actor, ActorResult, ActorSpec, ActorStatus, Behavior};
pub use actor_system::ActorSystem;

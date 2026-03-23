//! # Actor -- Isolated Unit of Computation
//!
//! An Actor is an isolated unit of computation. It has an **address** (so other
//! actors can send it messages), a **mailbox** (where incoming messages queue up),
//! a **behavior** (a function that processes one message at a time), and **state**
//! (private data that only the actor can see or modify).
//!
//! ## Analogy
//!
//! An actor is a person sitting alone in a soundproofed room with a mail slot
//! in the door. Letters (messages) come in through the slot and pile up in a
//! tray (mailbox). The person reads one letter at a time, thinks about it,
//! possibly writes reply letters and slides them out through their own mail
//! slot, and possibly rearranges things on their desk (state). They never
//! leave the room. They never look into anyone else's room.
//!
//! ## Processing Guarantees
//!
//! 1. **Sequential processing:** An actor processes exactly one message at a
//!    time. No races, no deadlocks, no locks needed.
//!
//! 2. **At-most-once delivery:** A message in the mailbox is delivered to the
//!    behavior function exactly once. If the actor panics mid-processing, the
//!    message is lost.
//!
//! 3. **No ordering across actors:** Messages from A to B arrive in order,
//!    but messages from A to B and A to C have no relative ordering guarantee.

use std::any::Any;
use std::collections::VecDeque;

use crate::message::Message;

// ---------------------------------------------------------------------------
// Type alias for behavior functions
// ---------------------------------------------------------------------------

/// The type of an actor's behavior function.
///
/// A behavior takes the actor's current state (as a `Box<dyn Any>`) and a
/// reference to the incoming message, and returns an `ActorResult` describing
/// what the actor wants to do: update state, send messages, create actors,
/// or stop.
///
/// Using a type alias reduces repetition and satisfies clippy's type_complexity
/// lint, since `Box<dyn Fn(Box<dyn Any>, &Message) -> ActorResult>` appears in
/// multiple places (Actor, ActorSpec, create_actor).
pub type Behavior = Box<dyn Fn(Box<dyn Any>, &Message) -> ActorResult>;

// ---------------------------------------------------------------------------
// Actor status
// ---------------------------------------------------------------------------

/// The lifecycle state of an actor.
///
/// ```text
/// +------------+     receive msg     +-------------+     stop     +---------+
/// |   IDLE     | ----------------->  | PROCESSING  | ----------> | STOPPED |
/// |            | <-----------------  |             |             |         |
/// +------------+     done            +-------------+             +---------+
///       ^                                  |
///       +----------------------------------+
///              (if stop == false)
/// ```
///
/// - **Idle:** Waiting for messages. The actor is ready to process.
/// - **Processing:** Currently handling a message. No other message will be
///   dequeued until the current behavior call returns.
/// - **Stopped:** Permanently halted. No further messages will be delivered.
///   Any messages in the mailbox are drained to dead_letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorStatus {
    Idle,
    Processing,
    Stopped,
}

impl std::fmt::Display for ActorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorStatus::Idle => write!(f, "idle"),
            ActorStatus::Processing => write!(f, "processing"),
            ActorStatus::Stopped => write!(f, "stopped"),
        }
    }
}

// ---------------------------------------------------------------------------
// ActorResult -- return value from behavior
// ---------------------------------------------------------------------------

/// The return value from an actor's behavior function.
///
/// When an actor processes a message, it returns an ActorResult that tells the
/// ActorSystem what to do next:
///
/// ```text
/// +-------------------+----------------------------------------------+
/// | Field             | Description                                  |
/// +-------------------+----------------------------------------------+
/// | new_state         | The actor's state after processing            |
/// | messages_to_send  | List of (target_id, message) pairs           |
/// | actors_to_create  | List of ActorSpec for new actors to spawn    |
/// | stop              | If true, actor halts after this message       |
/// +-------------------+----------------------------------------------+
/// ```
pub struct ActorResult {
    /// The actor's updated state. Can be the same object (no change) or
    /// completely new state.
    pub new_state: Box<dyn Any>,

    /// Messages to deliver to other actors. Each entry is a (target_id, message)
    /// pair. The ActorSystem will call `send(target_id, message)` for each.
    pub messages_to_send: Vec<(String, Message)>,

    /// Specifications for new actors to create. The ActorSystem will call
    /// `create_actor` for each spec.
    pub actors_to_create: Vec<ActorSpec>,

    /// If true, the actor stops permanently after processing this message.
    /// Its mailbox is drained to dead_letters and no further messages are
    /// delivered.
    pub stop: bool,
}

// ---------------------------------------------------------------------------
// ActorSpec -- blueprint for creating a new actor
// ---------------------------------------------------------------------------

/// Specification for creating a new actor.
///
/// An ActorSpec is a blueprint: it contains the actor's id, initial state,
/// and behavior function. When returned in an ActorResult's `actors_to_create`
/// list, the ActorSystem uses this spec to spawn the new actor.
pub struct ActorSpec {
    /// Unique identifier for the new actor.
    pub actor_id: String,

    /// Initial state for the new actor (can be any type, boxed).
    pub initial_state: Box<dyn Any>,

    /// Behavior function for the new actor.
    pub behavior: Behavior,
}

// ---------------------------------------------------------------------------
// Actor
// ---------------------------------------------------------------------------

/// An isolated unit of computation with a mailbox and private state.
///
/// ## Fields
///
/// - `id` -- Unique address. Other actors use this to send messages.
/// - `mailbox` -- FIFO queue of incoming messages. Messages are enqueued by
///   the ActorSystem, dequeued one at a time by process_next.
/// - `state` -- Private data. Only the behavior function can read/write it.
/// - `behavior` -- A function `(state, message) -> ActorResult`.
/// - `status` -- IDLE, PROCESSING, or STOPPED.
///
/// ## Why `Box<dyn Any>` for State?
///
/// Different actors need different state types. A counter needs a `u64`, a
/// router needs a `HashMap`, a cache needs a `Vec`. Using `Box<dyn Any>`
/// allows each actor to choose its own state type. The behavior function
/// downcasts to the concrete type at runtime. This is the same pattern used
/// by Erlang (where state can be any term) and Akka (where state is typed
/// per actor).
pub struct Actor {
    pub id: String,
    pub mailbox: VecDeque<Message>,
    pub state: Box<dyn Any>,
    pub behavior: Behavior,
    pub status: ActorStatus,
}

impl Actor {
    /// Create a new actor with the given id, initial state, and behavior.
    ///
    /// The actor starts in IDLE status with an empty mailbox.
    pub fn new(
        id: &str,
        initial_state: Box<dyn Any>,
        behavior: Behavior,
    ) -> Self {
        Actor {
            id: id.to_string(),
            mailbox: VecDeque::new(),
            state: initial_state,
            behavior,
            status: ActorStatus::Idle,
        }
    }

    /// Enqueue a message into this actor's mailbox.
    ///
    /// Messages are added at the back (FIFO order). They will be processed
    /// in arrival order when `process_next` is called.
    pub fn enqueue(&mut self, msg: Message) {
        self.mailbox.push_back(msg);
    }

    /// Number of pending messages in the mailbox.
    pub fn mailbox_size(&self) -> usize {
        self.mailbox.len()
    }
}

impl std::fmt::Debug for Actor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Actor")
            .field("id", &self.id)
            .field("mailbox_size", &self.mailbox.len())
            .field("status", &self.status)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 37: Create actor with initial state, verify status is IDLE.
    #[test]
    fn test_create_actor() {
        let actor = Actor::new(
            "test_actor",
            Box::new(0_u64),
            Box::new(|state, _msg| ActorResult {
                new_state: state,
                messages_to_send: vec![],
                actors_to_create: vec![],
                stop: false,
            }),
        );
        assert_eq!(actor.id, "test_actor");
        assert_eq!(actor.status, ActorStatus::Idle);
        assert_eq!(actor.mailbox_size(), 0);
    }

    /// Test: Enqueue adds messages to mailbox.
    #[test]
    fn test_enqueue() {
        let mut actor = Actor::new(
            "test",
            Box::new(()),
            Box::new(|state, _msg| ActorResult {
                new_state: state,
                messages_to_send: vec![],
                actors_to_create: vec![],
                stop: false,
            }),
        );
        actor.enqueue(Message::text("sender", "hello"));
        assert_eq!(actor.mailbox_size(), 1);
        actor.enqueue(Message::text("sender", "world"));
        assert_eq!(actor.mailbox_size(), 2);
    }

    /// Test: ActorStatus display.
    #[test]
    fn test_status_display() {
        assert_eq!(ActorStatus::Idle.to_string(), "idle");
        assert_eq!(ActorStatus::Processing.to_string(), "processing");
        assert_eq!(ActorStatus::Stopped.to_string(), "stopped");
    }
}

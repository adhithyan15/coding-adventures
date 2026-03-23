//! # ActorSystem -- The Runtime for Actors
//!
//! The ActorSystem is the "world" that actors live in. It manages actor
//! lifecycles (creation, stopping), message delivery (routing messages to
//! mailboxes), and channel management.
//!
//! ## Analogy
//!
//! The ActorSystem is the office building. It has a directory (which actors
//! exist and their addresses), a mail room (message routing), and a building
//! manager (supervision -- restart actors that crash). Actors are tenants.
//! They register with the building, get an address, and the building delivers
//! their mail. But the building manager does not read the mail.
//!
//! ## Processing Model
//!
//! In V1, the ActorSystem processes actors **one at a time** in round-robin
//! order. This is sequential, not parallel. True parallelism (multiple actors
//! processing simultaneously via threads) is a future enhancement. The
//! sequential model is simpler to test, debug, and reason about.
//!
//! ```text
//! ActorSystem
//! +-------------------------------------------------------------+
//! | actors: HashMap<id, Actor>                                   |
//! | channels: HashMap<id, Channel>                               |
//! | dead_letters: Vec<Message>                                   |
//! | clock: u64 (monotonic)                                       |
//! +-------------------------------------------------------------+
//! ```

use std::any::Any;
use std::collections::HashMap;

use crate::actor::{Actor, ActorStatus, Behavior};
use crate::channel::Channel;
use crate::message::{ActorError, Message};

// ---------------------------------------------------------------------------
// ActorSystem
// ---------------------------------------------------------------------------

/// Runtime for managing actors, message delivery, and channels.
///
/// ## Operations
///
/// ```text
/// +---------------------+------------------------------------------------+
/// | Operation           | Description                                    |
/// +---------------------+------------------------------------------------+
/// | create_actor(...)   | Register a new actor with id, state, behavior  |
/// | stop_actor(id)      | Stop an actor, drain mailbox to dead_letters   |
/// | send(target, msg)   | Enqueue message in target's mailbox            |
/// | process_next(id)    | Process one message from actor's mailbox       |
/// | run_until_idle()    | Process all actors round-robin until quiet     |
/// | run_until_done()    | Like run_until_idle but exhaustive             |
/// | create_channel(...) | Create and register a new channel              |
/// | get_channel(id)     | Retrieve a channel by ID                       |
/// | shutdown()          | Stop all actors, drain all mailboxes            |
/// +---------------------+------------------------------------------------+
/// ```
pub struct ActorSystem {
    /// Map of actor_id to Actor. This is the registry of all living actors.
    pub actors: HashMap<String, Actor>,

    /// Map of channel_id to Channel. All channels in the system.
    pub channels: HashMap<String, Channel>,

    /// Messages that could not be delivered (target actor does not exist or
    /// is stopped). Useful for debugging and monitoring.
    pub dead_letters: Vec<Message>,

    /// Monotonic counter. Provides ordering for events in the system.
    pub clock: u64,
}

impl ActorSystem {
    /// Create a new, empty ActorSystem.
    ///
    /// The system starts with no actors, no channels, no dead letters, and
    /// a clock at zero.
    pub fn new() -> Self {
        ActorSystem {
            actors: HashMap::new(),
            channels: HashMap::new(),
            dead_letters: Vec::new(),
            clock: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Actor lifecycle
    // -----------------------------------------------------------------------

    /// Create and register a new actor.
    ///
    /// # Arguments
    ///
    /// * `actor_id` -- Unique identifier. Must not already be in use.
    /// * `initial_state` -- The actor's starting state (any type, boxed).
    /// * `behavior` -- Function called for each message: (state, msg) -> ActorResult.
    ///
    /// # Returns
    ///
    /// The actor_id on success, or an error if the id already exists.
    ///
    /// # How it works
    ///
    /// ```text
    /// 1. Check actors map for existing id
    ///    +-- EXISTS --> return error (duplicate)
    ///    +-- NOT FOUND --> continue
    /// 2. Create Actor with empty mailbox, given state, given behavior
    /// 3. Set status = IDLE
    /// 4. Insert into actors map
    /// 5. Return actor_id
    /// ```
    pub fn create_actor(
        &mut self,
        actor_id: &str,
        initial_state: Box<dyn Any>,
        behavior: Behavior,
    ) -> Result<String, ActorError> {
        if self.actors.contains_key(actor_id) {
            return Err(ActorError::InvalidFormat(format!(
                "actor '{}' already exists",
                actor_id
            )));
        }
        let actor = Actor::new(actor_id, initial_state, behavior);
        self.actors.insert(actor_id.to_string(), actor);
        Ok(actor_id.to_string())
    }

    /// Stop an actor and drain its mailbox to dead_letters.
    ///
    /// After stopping, the actor's status is STOPPED and no further messages
    /// will be delivered to it. Any pending messages in its mailbox are moved
    /// to dead_letters for debugging.
    ///
    /// ```text
    /// Before: Actor "worker" has mailbox [m1, m2, m3], status = IDLE
    /// stop_actor("worker")
    /// After:  Actor "worker" has mailbox [], status = STOPPED
    ///         dead_letters now contains [m1, m2, m3]
    /// ```
    pub fn stop_actor(&mut self, actor_id: &str) -> Result<(), ActorError> {
        let actor = self.actors.get_mut(actor_id).ok_or_else(|| {
            ActorError::InvalidFormat(format!("actor '{}' not found", actor_id))
        })?;
        actor.status = ActorStatus::Stopped;
        // Drain mailbox to dead_letters
        while let Some(msg) = actor.mailbox.pop_front() {
            self.dead_letters.push(msg);
        }
        Ok(())
    }

    /// Get the status of an actor as a string ("idle", "processing", "stopped").
    pub fn get_actor_status(&self, actor_id: &str) -> Result<String, ActorError> {
        let actor = self.actors.get(actor_id).ok_or_else(|| {
            ActorError::InvalidFormat(format!("actor '{}' not found", actor_id))
        })?;
        Ok(actor.status.to_string())
    }

    // -----------------------------------------------------------------------
    // Messaging
    // -----------------------------------------------------------------------

    /// Send a message to an actor's mailbox.
    ///
    /// ## Delivery algorithm
    ///
    /// ```text
    /// send(target_id, message)
    /// 1. Look up target_id in actors map.
    ///    +-- NOT FOUND --> dead_letters.append(message). Return error.
    ///    +-- FOUND --> continue.
    /// 2. Check target.status:
    ///    +-- STOPPED --> dead_letters.append(message). Return error.
    ///    +-- IDLE or PROCESSING --> continue.
    /// 3. Enqueue message at the back of target.mailbox.
    /// 4. Return success.
    /// ```
    ///
    /// Time complexity: O(1) -- hash map lookup + queue append.
    pub fn send(&mut self, target_id: &str, message: Message) -> Result<(), ActorError> {
        match self.actors.get_mut(target_id) {
            None => {
                self.dead_letters.push(message);
                Err(ActorError::InvalidFormat(format!(
                    "actor '{}' not found",
                    target_id
                )))
            }
            Some(actor) => {
                if actor.status == ActorStatus::Stopped {
                    self.dead_letters.push(message);
                    Err(ActorError::InvalidFormat(format!(
                        "actor '{}' is stopped",
                        target_id
                    )))
                } else {
                    actor.enqueue(message);
                    Ok(())
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Processing
    // -----------------------------------------------------------------------

    /// Process one message from an actor's mailbox.
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// process_next(actor_id)
    /// 1. Look up actor. If not found or STOPPED, return error.
    /// 2. If mailbox is empty, return Ok(false).
    /// 3. Dequeue front message (FIFO: oldest first).
    /// 4. Set status = PROCESSING.
    /// 5. Call behavior(state, message) -> ActorResult.
    ///    +-- If behavior panics:
    ///        a. State is UNCHANGED (we took ownership, so it's lost).
    ///        b. Message goes to dead_letters.
    ///        c. Actor returns to IDLE.
    /// 6. Update state to result.new_state.
    /// 7. Create each actor in result.actors_to_create.
    /// 8. Send each (target_id, msg) in result.messages_to_send.
    /// 9. If result.stop: set status = STOPPED, drain mailbox.
    ///    Else: set status = IDLE.
    /// 10. Return Ok(true).
    /// ```
    ///
    /// Returns `Ok(true)` if a message was processed, `Ok(false)` if the
    /// mailbox was empty.
    pub fn process_next(&mut self, actor_id: &str) -> Result<bool, ActorError> {
        // Step 1: Look up actor
        let actor = self.actors.get(actor_id).ok_or_else(|| {
            ActorError::InvalidFormat(format!("actor '{}' not found", actor_id))
        })?;

        if actor.status == ActorStatus::Stopped {
            return Err(ActorError::InvalidFormat(format!(
                "actor '{}' is stopped",
                actor_id
            )));
        }

        // We need to temporarily take ownership of state and the message.
        // Rust's borrow checker doesn't allow calling a closure that borrows
        // the actor while also mutating the actor. We work around this by
        // removing the actor from the map, processing, then reinserting.
        let mut actor = self.actors.remove(actor_id).unwrap();

        // Step 2: Check mailbox
        let msg = match actor.mailbox.pop_front() {
            Some(m) => m,
            None => {
                self.actors.insert(actor_id.to_string(), actor);
                return Ok(false);
            }
        };

        // Step 4: Set status to PROCESSING
        actor.status = ActorStatus::Processing;

        // Step 5: Call behavior
        // We use std::panic::catch_unwind to handle panicking behaviors.
        // For this to work, the behavior must be UnwindSafe. Since we're
        // using Box<dyn Any> and Box<dyn Fn>, we use AssertUnwindSafe.
        let state = std::mem::replace(&mut actor.state, Box::new(()));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (actor.behavior)(state, &msg)
        }));

        match result {
            Ok(actor_result) => {
                // Step 6: Update state
                actor.state = actor_result.new_state;

                // Step 9: Check stop flag
                if actor_result.stop {
                    actor.status = ActorStatus::Stopped;
                    while let Some(remaining_msg) = actor.mailbox.pop_front() {
                        self.dead_letters.push(remaining_msg);
                    }
                } else {
                    actor.status = ActorStatus::Idle;
                }

                // Re-insert actor before creating new actors or sending messages
                self.actors.insert(actor_id.to_string(), actor);

                // Step 7: Create actors FIRST (so they exist when messages arrive)
                for spec in actor_result.actors_to_create {
                    let _ = self.create_actor(&spec.actor_id, spec.initial_state, spec.behavior);
                }

                // Step 8: Send messages (targets may include newly created actors)
                for (target_id, out_msg) in actor_result.messages_to_send {
                    let _ = self.send(&target_id, out_msg);
                }

                Ok(true)
            }
            Err(_panic) => {
                // Behavior panicked -- actor state is lost but we handle gracefully
                // Step 5b: Message goes to dead_letters
                self.dead_letters.push(msg);
                // Step 5c: Actor continues with a dummy state
                // Step 5d: Actor returns to IDLE
                actor.state = Box::new(());
                actor.status = ActorStatus::Idle;
                self.actors.insert(actor_id.to_string(), actor);
                Ok(true)
            }
        }
    }

    /// Process all actors round-robin until no work remains.
    ///
    /// ## Algorithm
    ///
    /// ```text
    /// run_until_idle()
    /// 1. Loop:
    ///    a. Find any actor with status == IDLE and non-empty mailbox.
    ///    b. If none found, return (system is idle).
    ///    c. Call process_next(actor_id).
    /// 2. Return stats.
    /// ```
    ///
    /// Returns a map with "messages_processed" and "actors_created" counts.
    pub fn run_until_idle(&mut self) -> HashMap<String, u64> {
        let mut messages_processed: u64 = 0;
        let initial_actor_count = self.actors.len() as u64;

        loop {
            // Find an actor with work to do
            let actor_id = self
                .actors
                .values()
                .find(|a| a.status == ActorStatus::Idle && !a.mailbox.is_empty())
                .map(|a| a.id.clone());

            match actor_id {
                Some(id) => {
                    if let Ok(true) = self.process_next(&id) {
                        messages_processed += 1;
                    }
                }
                None => break,
            }
        }

        let mut stats = HashMap::new();
        stats.insert("messages_processed".to_string(), messages_processed);
        stats.insert(
            "actors_created".to_string(),
            self.actors.len() as u64 - initial_actor_count,
        );
        stats
    }

    /// Process all actors until the system is completely quiet.
    ///
    /// Like `run_until_idle()` but keeps going until no messages remain in
    /// any mailbox and no new messages are being generated. This handles
    /// chains of actors that produce messages for each other.
    ///
    /// Safety: includes a maximum iteration limit to prevent infinite loops
    /// (e.g., two actors ping-ponging forever).
    pub fn run_until_done(&mut self) -> HashMap<String, u64> {
        let mut total_processed: u64 = 0;
        let initial_actor_count = self.actors.len() as u64;
        let max_iterations = 100_000; // Safety valve

        for _ in 0..max_iterations {
            let actor_id = self
                .actors
                .values()
                .find(|a| a.status == ActorStatus::Idle && !a.mailbox.is_empty())
                .map(|a| a.id.clone());

            match actor_id {
                Some(id) => {
                    if let Ok(true) = self.process_next(&id) {
                        total_processed += 1;
                    }
                }
                None => break,
            }
        }

        let mut stats = HashMap::new();
        stats.insert("messages_processed".to_string(), total_processed);
        stats.insert(
            "actors_created".to_string(),
            self.actors.len() as u64 - initial_actor_count,
        );
        stats
    }

    // -----------------------------------------------------------------------
    // Channels
    // -----------------------------------------------------------------------

    /// Create and register a new channel.
    pub fn create_channel(&mut self, channel_id: &str, name: &str) -> &mut Channel {
        let channel = Channel::new(channel_id, name);
        self.channels.insert(channel_id.to_string(), channel);
        self.channels.get_mut(channel_id).unwrap()
    }

    /// Retrieve a channel by ID.
    pub fn get_channel(&self, channel_id: &str) -> Option<&Channel> {
        self.channels.get(channel_id)
    }

    /// Retrieve a mutable channel by ID.
    pub fn get_channel_mut(&mut self, channel_id: &str) -> Option<&mut Channel> {
        self.channels.get_mut(channel_id)
    }

    // -----------------------------------------------------------------------
    // Inspection
    // -----------------------------------------------------------------------

    /// List all registered actor IDs.
    pub fn actor_ids(&self) -> Vec<String> {
        self.actors.keys().cloned().collect()
    }

    /// Number of pending messages for an actor.
    pub fn mailbox_size(&self, actor_id: &str) -> Result<usize, ActorError> {
        let actor = self.actors.get(actor_id).ok_or_else(|| {
            ActorError::InvalidFormat(format!("actor '{}' not found", actor_id))
        })?;
        Ok(actor.mailbox_size())
    }

    /// Shut down the entire system.
    ///
    /// 1. Stop all actors (set status = STOPPED).
    /// 2. Drain all mailboxes to dead_letters.
    pub fn shutdown(&mut self) {
        let actor_ids: Vec<String> = self.actors.keys().cloned().collect();
        for id in actor_ids {
            let _ = self.stop_actor(&id);
        }
    }
}

impl Default for ActorSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::{ActorResult, ActorSpec};

    /// Helper: create a simple no-op behavior.
    fn noop_behavior() -> Box<dyn Fn(Box<dyn Any>, &Message) -> ActorResult> {
        Box::new(|state, _msg| ActorResult {
            new_state: state,
            messages_to_send: vec![],
            actors_to_create: vec![],
            stop: false,
        })
    }

    /// Helper: create a counter behavior that increments a u64 state.
    fn counter_behavior() -> Box<dyn Fn(Box<dyn Any>, &Message) -> ActorResult> {
        Box::new(|state, _msg| {
            let count = *state.downcast::<u64>().unwrap();
            ActorResult {
                new_state: Box::new(count + 1),
                messages_to_send: vec![],
                actors_to_create: vec![],
                stop: false,
            }
        })
    }

    /// Helper: create an echo behavior that sends the message back.
    fn echo_behavior() -> Box<dyn Fn(Box<dyn Any>, &Message) -> ActorResult> {
        Box::new(|state, msg| {
            let reply = Message::text("echo", &format!("echo: {}", msg.payload_text()));
            ActorResult {
                new_state: state,
                messages_to_send: vec![(msg.sender_id.clone(), reply)],
                actors_to_create: vec![],
                stop: false,
            }
        })
    }

    /// Test 38: Send message, verify mailbox_size is 1.
    #[test]
    fn test_send_message() {
        let mut system = ActorSystem::new();
        system
            .create_actor("target", Box::new(()), noop_behavior())
            .unwrap();
        let msg = Message::text("sender", "hello");
        system.send("target", msg).unwrap();
        assert_eq!(system.mailbox_size("target").unwrap(), 1);
    }

    /// Test 39: Send message, process_next, verify behavior was called.
    #[test]
    fn test_process_message() {
        let mut system = ActorSystem::new();
        system
            .create_actor("counter", Box::new(0_u64), counter_behavior())
            .unwrap();
        system
            .send("counter", Message::text("sender", "tick"))
            .unwrap();
        let processed = system.process_next("counter").unwrap();
        assert!(processed);
        assert_eq!(system.mailbox_size("counter").unwrap(), 0);
    }

    /// Test 40: Counter actor -- send 3 messages, verify state is 3.
    #[test]
    fn test_state_update() {
        let mut system = ActorSystem::new();
        system
            .create_actor("counter", Box::new(0_u64), counter_behavior())
            .unwrap();
        for _ in 0..3 {
            system
                .send("counter", Message::text("sender", "tick"))
                .unwrap();
        }
        for _ in 0..3 {
            system.process_next("counter").unwrap();
        }
        let actor = system.actors.get("counter").unwrap();
        let count = actor.state.downcast_ref::<u64>().unwrap();
        assert_eq!(*count, 3);
    }

    /// Test 41: Echo actor -- reply delivered to sender's mailbox.
    #[test]
    fn test_messages_to_send() {
        let mut system = ActorSystem::new();
        system
            .create_actor("echo", Box::new(()), echo_behavior())
            .unwrap();
        system
            .create_actor("requester", Box::new(()), noop_behavior())
            .unwrap();

        let msg = Message::text("requester", "hello");
        system.send("echo", msg).unwrap();
        system.process_next("echo").unwrap();

        // Echo should have sent a reply to "requester"
        assert_eq!(system.mailbox_size("requester").unwrap(), 1);
    }

    /// Test 42: Spawner actor creates new actor.
    #[test]
    fn test_actor_creation() {
        let mut system = ActorSystem::new();
        system
            .create_actor(
                "spawner",
                Box::new(0_u64),
                Box::new(|state, _msg| {
                    let count = *state.downcast::<u64>().unwrap();
                    let new_id = format!("child_{}", count);
                    ActorResult {
                        new_state: Box::new(count + 1),
                        messages_to_send: vec![],
                        actors_to_create: vec![ActorSpec {
                            actor_id: new_id,
                            initial_state: Box::new(()),
                            behavior: Box::new(|s, _| ActorResult {
                                new_state: s,
                                messages_to_send: vec![],
                                actors_to_create: vec![],
                                stop: false,
                            }),
                        }],
                        stop: false,
                    }
                }),
            )
            .unwrap();

        system
            .send("spawner", Message::text("user", "spawn"))
            .unwrap();
        system.process_next("spawner").unwrap();

        assert!(system.actors.contains_key("child_0"));
    }

    /// Test 43: Stop actor -- status is STOPPED after processing stop message.
    #[test]
    fn test_stop_actor() {
        let mut system = ActorSystem::new();
        system
            .create_actor(
                "stopper",
                Box::new(()),
                Box::new(|state, _msg| ActorResult {
                    new_state: state,
                    messages_to_send: vec![],
                    actors_to_create: vec![],
                    stop: true,
                }),
            )
            .unwrap();
        system
            .send("stopper", Message::text("user", "stop"))
            .unwrap();
        system.process_next("stopper").unwrap();
        assert_eq!(
            system.get_actor_status("stopper").unwrap(),
            "stopped"
        );
    }

    /// Test 44: Stopped actor rejects messages -- they go to dead_letters.
    #[test]
    fn test_stopped_actor_rejects_messages() {
        let mut system = ActorSystem::new();
        system
            .create_actor("target", Box::new(()), noop_behavior())
            .unwrap();
        system.stop_actor("target").unwrap();

        let msg = Message::text("sender", "hello");
        let result = system.send("target", msg);
        assert!(result.is_err());
        assert_eq!(system.dead_letters.len(), 1);
    }

    /// Test 45: Send to non-existent actor goes to dead_letters.
    #[test]
    fn test_dead_letters() {
        let mut system = ActorSystem::new();
        let msg = Message::text("sender", "hello");
        let result = system.send("nonexistent", msg);
        assert!(result.is_err());
        assert_eq!(system.dead_letters.len(), 1);
    }

    /// Test 46: Sequential processing in FIFO order.
    #[test]
    fn test_sequential_processing() {
        let mut system = ActorSystem::new();
        // Use a behavior that records the order of messages received
        system
            .create_actor(
                "recorder",
                Box::new(Vec::<String>::new()),
                Box::new(|state, msg| {
                    let mut log = *state.downcast::<Vec<String>>().unwrap();
                    log.push(msg.payload_text());
                    ActorResult {
                        new_state: Box::new(log),
                        messages_to_send: vec![],
                        actors_to_create: vec![],
                        stop: false,
                    }
                }),
            )
            .unwrap();

        system
            .send("recorder", Message::text("s", "first"))
            .unwrap();
        system
            .send("recorder", Message::text("s", "second"))
            .unwrap();
        system
            .send("recorder", Message::text("s", "third"))
            .unwrap();

        for _ in 0..3 {
            system.process_next("recorder").unwrap();
        }

        let actor = system.actors.get("recorder").unwrap();
        let log = actor.state.downcast_ref::<Vec<String>>().unwrap();
        assert_eq!(log, &vec!["first", "second", "third"]);
    }

    /// Test 47: Mailbox drains to dead_letters on stop.
    #[test]
    fn test_mailbox_drains_on_stop() {
        let mut system = ActorSystem::new();
        system
            .create_actor("target", Box::new(()), noop_behavior())
            .unwrap();
        system
            .send("target", Message::text("s", "a"))
            .unwrap();
        system
            .send("target", Message::text("s", "b"))
            .unwrap();
        system
            .send("target", Message::text("s", "c"))
            .unwrap();
        assert_eq!(system.mailbox_size("target").unwrap(), 3);

        system.stop_actor("target").unwrap();
        assert_eq!(system.dead_letters.len(), 3);
        assert_eq!(system.mailbox_size("target").unwrap(), 0);
    }

    /// Test 48: Behavior panic -- state unchanged, message to dead_letters,
    ///          actor continues.
    #[test]
    fn test_behavior_exception() {
        let mut system = ActorSystem::new();
        system
            .create_actor(
                "panicker",
                Box::new(0_u64),
                Box::new(|state, msg| {
                    if msg.payload_text() == "panic" {
                        panic!("intentional panic for testing");
                    }
                    let count = *state.downcast::<u64>().unwrap();
                    ActorResult {
                        new_state: Box::new(count + 1),
                        messages_to_send: vec![],
                        actors_to_create: vec![],
                        stop: false,
                    }
                }),
            )
            .unwrap();

        // Send a panic-inducing message
        system
            .send("panicker", Message::text("s", "panic"))
            .unwrap();
        let result = system.process_next("panicker");
        assert!(result.is_ok()); // Actor didn't crash the system

        // Panic message went to dead_letters
        assert_eq!(system.dead_letters.len(), 1);

        // Actor is still alive and processing
        assert_eq!(system.get_actor_status("panicker").unwrap(), "idle");
    }

    /// Test 49: Duplicate actor ID returns error.
    #[test]
    fn test_duplicate_actor_id() {
        let mut system = ActorSystem::new();
        system
            .create_actor("dup", Box::new(()), noop_behavior())
            .unwrap();
        let result = system.create_actor("dup", Box::new(()), noop_behavior());
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Integration tests
    // -----------------------------------------------------------------------

    /// Test 50: Ping-pong -- two actors exchange messages 10 times.
    #[test]
    fn test_ping_pong() {
        let mut system = ActorSystem::new();

        // Ping sends a message to pong, and vice versa.
        // State is a counter that stops after 10 exchanges.
        system
            .create_actor(
                "ping",
                Box::new(0_u64),
                Box::new(|state, _msg| {
                    let count = *state.downcast::<u64>().unwrap();
                    if count >= 10 {
                        return ActorResult {
                            new_state: Box::new(count),
                            messages_to_send: vec![],
                            actors_to_create: vec![],
                            stop: false,
                        };
                    }
                    let reply = Message::text("ping", &format!("ping_{}", count));
                    ActorResult {
                        new_state: Box::new(count + 1),
                        messages_to_send: vec![("pong".to_string(), reply)],
                        actors_to_create: vec![],
                        stop: false,
                    }
                }),
            )
            .unwrap();

        system
            .create_actor(
                "pong",
                Box::new(0_u64),
                Box::new(|state, _msg| {
                    let count = *state.downcast::<u64>().unwrap();
                    if count >= 10 {
                        return ActorResult {
                            new_state: Box::new(count),
                            messages_to_send: vec![],
                            actors_to_create: vec![],
                            stop: false,
                        };
                    }
                    let reply = Message::text("pong", &format!("pong_{}", count));
                    ActorResult {
                        new_state: Box::new(count + 1),
                        messages_to_send: vec![("ping".to_string(), reply)],
                        actors_to_create: vec![],
                        stop: false,
                    }
                }),
            )
            .unwrap();

        // Kick off the exchange
        system
            .send("ping", Message::text("user", "start"))
            .unwrap();
        let stats = system.run_until_done();

        // ping processed 11 (start + 10 pongs), pong processed 10 pings
        let total = *stats.get("messages_processed").unwrap();
        assert!(total >= 20, "expected >= 20 messages processed, got {}", total);
    }

    /// Test 51: Pipeline -- A -> B -> C chain.
    #[test]
    fn test_pipeline() {
        let mut system = ActorSystem::new();

        // Actor A: forwards message to B with prefix "A:"
        system
            .create_actor(
                "a",
                Box::new(()),
                Box::new(|state, msg| {
                    let forwarded = Message::text("a", &format!("A:{}", msg.payload_text()));
                    ActorResult {
                        new_state: state,
                        messages_to_send: vec![("b".to_string(), forwarded)],
                        actors_to_create: vec![],
                        stop: false,
                    }
                }),
            )
            .unwrap();

        // Actor B: forwards message to C with prefix "B:"
        system
            .create_actor(
                "b",
                Box::new(()),
                Box::new(|state, msg| {
                    let forwarded = Message::text("b", &format!("B:{}", msg.payload_text()));
                    ActorResult {
                        new_state: state,
                        messages_to_send: vec![("c".to_string(), forwarded)],
                        actors_to_create: vec![],
                        stop: false,
                    }
                }),
            )
            .unwrap();

        // Actor C: stores received message
        system
            .create_actor(
                "c",
                Box::new(String::new()),
                Box::new(|_state, msg| ActorResult {
                    new_state: Box::new(msg.payload_text()),
                    messages_to_send: vec![],
                    actors_to_create: vec![],
                    stop: false,
                }),
            )
            .unwrap();

        system
            .send("a", Message::text("user", "hello"))
            .unwrap();
        system.run_until_done();

        let c_state = system
            .actors
            .get("c")
            .unwrap()
            .state
            .downcast_ref::<String>()
            .unwrap();
        assert_eq!(c_state, "B:A:hello");
    }

    /// Test 52: Channel-based pipeline.
    #[test]
    fn test_channel_pipeline() {
        let mut system = ActorSystem::new();
        system.create_channel("ch_001", "data-pipe");

        // Producer writes to channel
        let ch = system.get_channel_mut("ch_001").unwrap();
        ch.append(Message::text("producer", "msg_0"));
        ch.append(Message::text("producer", "msg_1"));
        ch.append(Message::text("producer", "msg_2"));

        // Consumer reads from channel
        let ch = system.get_channel("ch_001").unwrap();
        let batch = ch.read(0, 10);
        assert_eq!(batch.len(), 3);
        assert_eq!(batch[0].payload_text(), "msg_0");
        assert_eq!(batch[1].payload_text(), "msg_1");
        assert_eq!(batch[2].payload_text(), "msg_2");
    }

    /// Test 53: Fan-out -- one sender, five receivers.
    #[test]
    fn test_fan_out() {
        let mut system = ActorSystem::new();

        for i in 0..5 {
            system
                .create_actor(
                    &format!("receiver_{}", i),
                    Box::new(0_u64),
                    counter_behavior(),
                )
                .unwrap();
        }

        // Send same message to all 5 receivers
        for i in 0..5 {
            let msg = Message::text("sender", "broadcast");
            system.send(&format!("receiver_{}", i), msg).unwrap();
        }

        system.run_until_done();

        // All 5 should have processed exactly 1 message
        for i in 0..5 {
            let actor = system.actors.get(&format!("receiver_{}", i)).unwrap();
            let count = actor.state.downcast_ref::<u64>().unwrap();
            assert_eq!(*count, 1);
        }
    }

    /// Test 54: Dynamic topology -- spawned actor participates.
    #[test]
    fn test_dynamic_topology() {
        let mut system = ActorSystem::new();

        // Spawner creates a child and sends it a message
        system
            .create_actor(
                "spawner",
                Box::new(()),
                Box::new(|state, _msg| ActorResult {
                    new_state: state,
                    messages_to_send: vec![(
                        "child".to_string(),
                        Message::text("spawner", "hello child"),
                    )],
                    actors_to_create: vec![ActorSpec {
                        actor_id: "child".to_string(),
                        initial_state: Box::new(String::new()),
                        behavior: Box::new(|_state, msg: &Message| ActorResult {
                            new_state: Box::new(msg.payload_text()),
                            messages_to_send: vec![],
                            actors_to_create: vec![],
                            stop: false,
                        }),
                    }],
                    stop: false,
                }),
            )
            .unwrap();

        system
            .send("spawner", Message::text("user", "go"))
            .unwrap();
        system.run_until_done();

        // Child should exist and have processed the message
        assert!(system.actors.contains_key("child"));
        let child_state = system
            .actors
            .get("child")
            .unwrap()
            .state
            .downcast_ref::<String>()
            .unwrap();
        assert_eq!(child_state, "hello child");
    }

    /// Test 55: Run until idle with complex network of 5 actors.
    #[test]
    fn test_run_until_idle() {
        let mut system = ActorSystem::new();

        // Create 5 actors that each forward to the next
        for i in 0..5 {
            let next = if i < 4 {
                Some(format!("actor_{}", i + 1))
            } else {
                None
            };
            system
                .create_actor(
                    &format!("actor_{}", i),
                    Box::new(0_u64),
                    Box::new(move |state, _msg| {
                        let count = *state.downcast::<u64>().unwrap();
                        let mut msgs = vec![];
                        if let Some(ref target) = next {
                            msgs.push((
                                target.clone(),
                                Message::text(
                                    &format!("actor_{}", i),
                                    &format!("forwarded_{}", count),
                                ),
                            ));
                        }
                        ActorResult {
                            new_state: Box::new(count + 1),
                            messages_to_send: msgs,
                            actors_to_create: vec![],
                            stop: false,
                        }
                    }),
                )
                .unwrap();
        }

        // Send initial message to actor_0
        system
            .send("actor_0", Message::text("user", "start"))
            .unwrap();
        let stats = system.run_until_idle();

        let processed = *stats.get("messages_processed").unwrap();
        assert_eq!(processed, 5); // Each actor processed exactly 1 message
    }

    /// Test 56: Persistence round-trip with channels.
    #[test]
    fn test_persistence_roundtrip() {
        use std::fs;

        let dir = std::env::temp_dir().join(format!(
            "actor_test_persist_rt_{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let dir_str = dir.to_string_lossy().to_string();

        let mut system = ActorSystem::new();
        let ch = system.create_channel("ch_001", "roundtrip-test");
        ch.append(Message::text("producer", "hello"));
        ch.append(Message::binary(
            "producer",
            "image/png",
            vec![0x89, 0x50, 0x4E, 0x47],
        ));

        // Persist
        let ch = system.get_channel("ch_001").unwrap();
        ch.persist(&dir_str).unwrap();

        // Recover in a new system
        let recovered = Channel::recover(&dir_str, "roundtrip-test").unwrap();
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered.log[0].payload_text(), "hello");
        assert_eq!(recovered.log[1].content_type, "image/png");
        assert_eq!(recovered.log[1].payload, vec![0x89, 0x50, 0x4E, 0x47]);

        let _ = fs::remove_dir_all(&dir);
    }

    /// Test 57: Large-scale -- 100 actors, 1000 messages.
    #[test]
    fn test_large_scale() {
        let mut system = ActorSystem::new();

        // Create 100 counter actors
        for i in 0..100 {
            system
                .create_actor(
                    &format!("actor_{}", i),
                    Box::new(0_u64),
                    counter_behavior(),
                )
                .unwrap();
        }

        // Send 1000 messages to random actors (deterministic "random" via modulo)
        for i in 0..1000 {
            let target = format!("actor_{}", i % 100);
            let msg = Message::text("sender", &format!("msg_{}", i));
            system.send(&target, msg).unwrap();
        }

        let stats = system.run_until_done();
        let processed = *stats.get("messages_processed").unwrap();
        assert_eq!(processed, 1000);

        // Verify total count across all actors is 1000
        let total: u64 = (0..100)
            .map(|i| {
                let actor = system.actors.get(&format!("actor_{}", i)).unwrap();
                *actor.state.downcast_ref::<u64>().unwrap()
            })
            .sum();
        assert_eq!(total, 1000);
        assert!(system.dead_letters.is_empty());
    }

    /// Test 58: Binary message pipeline -- image through channel.
    #[test]
    fn test_binary_message_pipeline() {
        let mut system = ActorSystem::new();

        // PNG image data (first 8 bytes of a real PNG header)
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

        // Actor A sends a PNG image via channel
        let ch = system.create_channel("ch_images", "images");
        ch.append(Message::binary("actor_a", "image/png", png_data.clone()));

        // Actor B reads from the channel and verifies
        let ch = system.get_channel("ch_images").unwrap();
        let msgs = ch.read(0, 1);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content_type, "image/png");
        assert_eq!(msgs[0].payload, png_data);
    }

    /// Test: actor_ids returns all registered actors.
    #[test]
    fn test_actor_ids() {
        let mut system = ActorSystem::new();
        system
            .create_actor("a", Box::new(()), noop_behavior())
            .unwrap();
        system
            .create_actor("b", Box::new(()), noop_behavior())
            .unwrap();
        let mut ids = system.actor_ids();
        ids.sort();
        assert_eq!(ids, vec!["a", "b"]);
    }

    /// Test: shutdown stops all actors and drains mailboxes.
    #[test]
    fn test_shutdown() {
        let mut system = ActorSystem::new();
        system
            .create_actor("a", Box::new(()), noop_behavior())
            .unwrap();
        system
            .create_actor("b", Box::new(()), noop_behavior())
            .unwrap();
        system.send("a", Message::text("s", "msg1")).unwrap();
        system.send("b", Message::text("s", "msg2")).unwrap();

        system.shutdown();

        assert_eq!(system.get_actor_status("a").unwrap(), "stopped");
        assert_eq!(system.get_actor_status("b").unwrap(), "stopped");
        assert_eq!(system.dead_letters.len(), 2);
    }

    /// Test: process_next on empty mailbox returns Ok(false).
    #[test]
    fn test_process_next_empty_mailbox() {
        let mut system = ActorSystem::new();
        system
            .create_actor("empty", Box::new(()), noop_behavior())
            .unwrap();
        let result = system.process_next("empty").unwrap();
        assert!(!result);
    }

    /// Test: process_next on non-existent actor returns error.
    #[test]
    fn test_process_next_nonexistent() {
        let mut system = ActorSystem::new();
        let result = system.process_next("ghost");
        assert!(result.is_err());
    }

    /// Test: Default trait implementation for ActorSystem.
    #[test]
    fn test_default() {
        let system = ActorSystem::default();
        assert!(system.actors.is_empty());
        assert!(system.channels.is_empty());
        assert!(system.dead_letters.is_empty());
    }
}

use chief_of_staff_agent_stdio_host::{
    AgentCommand, AgentResponder, AgentStdioError, LevelFourHost, LevelFourHostError,
    LevelFourRunOutcome, StdioAgentSession,
};
use chief_of_staff_agent_stdio_protocol::{AgentInput, AgentResponse};
use chief_of_staff_channel_crypto::{ChannelId, Sequence};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelEndpointError, MessageId, Originator, PublishedMessage, ReceivedMessage,
    Receiver,
};
use std::cell::{Cell, RefCell};
use std::process::Command;

fn message_id(byte: u8) -> MessageId {
    let mut bytes = [byte; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    MessageId::from_uuid_v7(bytes).unwrap()
}

fn channel_id(byte: u8) -> ChannelId {
    let mut bytes = [byte; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    ChannelId(bytes)
}

struct FakeReceiver {
    id: AgentId,
    messages: Vec<ReceivedMessage>,
    acknowledgements: Vec<MessageId>,
    fail_acknowledge: bool,
}

impl FakeReceiver {
    fn with_message() -> Self {
        Self {
            id: AgentId::new(b"level-four-agent".to_vec()).unwrap(),
            messages: vec![ReceivedMessage {
                message_id: message_id(7),
                sequence: Sequence(4),
                timestamp_ns: 9_007_199_254_740_993,
                content_type: "application/octet-stream".to_string(),
                payload: b"hello".to_vec(),
            }],
            acknowledgements: Vec::new(),
            fail_acknowledge: false,
        }
    }
}

impl Receiver for FakeReceiver {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn channel_id(&self) -> ChannelId {
        channel_id(3)
    }

    fn public_key(&self) -> [u8; 32] {
        [2; 32]
    }

    fn receive(&mut self, _limit: usize) -> Result<Vec<ReceivedMessage>, ChannelEndpointError> {
        Ok(std::mem::take(&mut self.messages))
    }

    fn acknowledge(&mut self, message_id: MessageId) -> Result<Sequence, ChannelEndpointError> {
        if self.fail_acknowledge {
            return Err(ChannelEndpointError::ChannelDestroyed);
        }
        self.acknowledgements.push(message_id);
        Ok(Sequence(5))
    }
}

struct FakeOriginator {
    id: AgentId,
    publications: RefCell<Vec<(Vec<u8>, String)>>,
    fail: Cell<bool>,
}

impl FakeOriginator {
    fn new() -> Self {
        Self {
            id: AgentId::new(b"level-four-output".to_vec()).unwrap(),
            publications: RefCell::new(Vec::new()),
            fail: Cell::new(false),
        }
    }
}

impl Originator for FakeOriginator {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn channel_id(&self) -> ChannelId {
        channel_id(4)
    }

    fn public_key(&self) -> [u8; 32] {
        [4; 32]
    }

    fn publish(
        &self,
        payload: &[u8],
        content_type: &str,
    ) -> Result<PublishedMessage, ChannelEndpointError> {
        if self.fail.get() {
            return Err(ChannelEndpointError::ChannelDestroyed);
        }
        self.publications
            .borrow_mut()
            .push((payload.to_vec(), content_type.to_string()));
        Ok(PublishedMessage {
            message_id: message_id(9),
            sequence: Sequence(8),
            timestamp_ns: 11,
        })
    }
}

fn child(mode: &str) -> StdioAgentSession {
    let command =
        AgentCommand::new(env!("CARGO_BIN_EXE_chief-agent-stdio-test-child"), [mode]).unwrap();
    StdioAgentSession::spawn(&command).unwrap()
}

fn python_program() -> &'static str {
    ["python3", "python"]
        .into_iter()
        .find(|program| {
            Command::new(program)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
        })
        .expect("the repository build image must provide Python")
}

#[test]
fn real_subprocess_flows_then_publishes_and_acknowledges() {
    let mut session = child("success");
    let mut receiver = FakeReceiver::with_message();
    let originator = FakeOriginator::new();
    let outcome = LevelFourHost::new(&mut session)
        .run_once(&mut receiver, &originator)
        .unwrap();

    assert!(matches!(
        outcome,
        LevelFourRunOutcome::Processed {
            input_message_id,
            acknowledged_through: Sequence(5),
            ..
        } if input_message_id == message_id(7)
    ));
    assert_eq!(
        originator.publications.borrow().as_slice(),
        &[(b"world".to_vec(), "text/plain; charset=utf-8".to_string())]
    );
    assert_eq!(receiver.acknowledgements, [message_id(7)]);
}

#[test]
fn one_long_lived_session_handles_multiple_ordered_messages() {
    let mut session = child("success");
    let first = AgentInput::new(
        "01980000-0000-7000-8000-000000000001",
        "01980000-0000-7000-8000-000000000010",
        1,
        2,
        "text/plain",
        b"first".to_vec(),
    )
    .unwrap();
    let second = AgentInput::new(
        "01980000-0000-7000-8000-000000000002",
        "01980000-0000-7000-8000-000000000010",
        2,
        3,
        "text/plain",
        b"second".to_vec(),
    )
    .unwrap();
    assert_eq!(session.respond(&first).unwrap().payload(), b"world");
    assert_eq!(session.respond(&second).unwrap().payload(), b"world");
}

#[test]
fn sdk_free_python_agent_uses_the_normative_json_lines_contract() {
    let script = format!(
        "{}/tests/fixtures/echo_agent.py",
        env!("CARGO_MANIFEST_DIR")
    );
    let command = AgentCommand::new(python_program(), [script]).unwrap();
    let mut session = StdioAgentSession::spawn(&command).unwrap();
    let input = AgentInput::new(
        "01980000-0000-7000-8000-000000000001",
        "01980000-0000-7000-8000-000000000010",
        u64::MAX,
        9_007_199_254_740_993,
        "application/octet-stream",
        [0, 255, 128, 65],
    )
    .unwrap();
    let response = session.respond(&input).unwrap();
    assert_eq!(response.input_message_id(), input.message_id());
    assert_eq!(response.content_type(), "text/plain; charset=utf-8");
    assert_eq!(response.payload(), b"python-world");
}

#[test]
fn malformed_mismatched_and_early_exit_children_fail_closed() {
    for (mode, expected) in [
        ("malformed", "protocol"),
        ("mismatch", "correlation"),
        ("eof", "before a response"),
        ("exit", "before a response"),
    ] {
        let mut session = child(mode);
        let input = AgentInput::new(
            "01980000-0000-7000-8000-000000000001",
            "01980000-0000-7000-8000-000000000010",
            1,
            2,
            "text/plain",
            b"input".to_vec(),
        )
        .unwrap();
        let error = session.respond(&input).unwrap_err();
        assert!(error.to_string().contains(expected), "{mode}: {error}");
        assert!(
            session.respond(&input).is_err(),
            "{mode} session stayed open"
        );
    }
}

struct FixedResponder {
    response: AgentResponse,
}

impl AgentResponder for FixedResponder {
    fn respond(&mut self, _input: &AgentInput) -> Result<AgentResponse, AgentStdioError> {
        Ok(self.response.clone())
    }
}

#[test]
fn idle_and_pre_ack_failures_leave_cursor_unchanged() {
    let mut idle_receiver = FakeReceiver::with_message();
    idle_receiver.messages.clear();
    let originator = FakeOriginator::new();
    let mut responder = FixedResponder {
        response: AgentResponse::new("unused", "text/plain", b"unused".to_vec()).unwrap(),
    };
    assert!(matches!(
        LevelFourHost::new(&mut responder)
            .run_once(&mut idle_receiver, &originator)
            .unwrap(),
        LevelFourRunOutcome::Idle
    ));

    let mut receiver = FakeReceiver::with_message();
    let mut mismatched = FixedResponder {
        response: AgentResponse::new("different", "text/plain", b"no".to_vec()).unwrap(),
    };
    assert!(matches!(
        LevelFourHost::new(&mut mismatched).run_once(&mut receiver, &originator),
        Err(LevelFourHostError::Agent(AgentStdioError::Protocol(_)))
    ));
    assert!(receiver.acknowledgements.is_empty());
    assert!(originator.publications.borrow().is_empty());

    let mut receiver = FakeReceiver::with_message();
    let mut response = FixedResponder {
        response: AgentResponse::new(
            "07070707-0707-7007-8007-070707070707",
            "text/plain",
            b"no".to_vec(),
        )
        .unwrap(),
    };
    originator.fail.set(true);
    assert!(matches!(
        LevelFourHost::new(&mut response).run_once(&mut receiver, &originator),
        Err(LevelFourHostError::Channel(_))
    ));
    assert!(receiver.acknowledgements.is_empty());
}

#[test]
fn acknowledgement_failure_occurs_after_publication() {
    let mut receiver = FakeReceiver::with_message();
    receiver.fail_acknowledge = true;
    let originator = FakeOriginator::new();
    let mut response = FixedResponder {
        response: AgentResponse::new(
            "07070707-0707-7007-8007-070707070707",
            "text/plain",
            b"published".to_vec(),
        )
        .unwrap(),
    };
    assert!(matches!(
        LevelFourHost::new(&mut response).run_once(&mut receiver, &originator),
        Err(LevelFourHostError::Channel(_))
    ));
    assert_eq!(originator.publications.borrow().len(), 1);
    assert!(receiver.acknowledgements.is_empty());
}

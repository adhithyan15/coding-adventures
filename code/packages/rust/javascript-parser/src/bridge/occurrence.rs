//! CLOC32 observed-entry API scaffold. Context attachment follows the committed
//! behavioral red; this delegation deliberately does not invoke the observer.
use super::{BridgeError, GrammarASTNode, Program};
use coding_adventures_javascript_tokens::EsVersion;

/// A caller-owned array identity policy, not a trusted source-span certificate.
/// The bridge may assign only the returned array CV; it cannot validate arbitrary
/// caller strings or bound arbitrary Rust callback work.
pub trait ArrayOccurrenceObserver: Send {
    fn array_constructed(&mut self, node: &GrammarASTNode) -> Result<Option<String>, String>;
}

/// Keep observed failures distinct without changing the legacy error enum.
#[derive(Debug)]
pub enum ObservedBridgeError {
    Bridge(BridgeError),
    Observer(String),
    Worker(String),
}

impl std::fmt::Display for ObservedBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bridge(error) => std::fmt::Display::fmt(error, f),
            Self::Observer(message) => write!(f, "array occurrence observer failed: {message}"),
            Self::Worker(message) => write!(f, "array occurrence bridge worker failed: {message}"),
        }
    }
}

impl std::error::Error for ObservedBridgeError {}

pub fn grammar_to_program_with_array_observer(
    node: &GrammarASTNode,
    version: EsVersion,
    _observer: &mut dyn ArrayOccurrenceObserver,
) -> Result<Program, ObservedBridgeError> {
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("javascript-array-bridge".into())
            .stack_size(128 * 1024 * 1024)
            .spawn_scoped(scope, move || {
                super::grammar_to_program(node, version).map_err(ObservedBridgeError::Bridge)
            })
            .map_err(|error| ObservedBridgeError::Worker(error.to_string()))?;
        worker.join().map_err(|_| ObservedBridgeError::Worker("worker panicked".into()))?
    })
}

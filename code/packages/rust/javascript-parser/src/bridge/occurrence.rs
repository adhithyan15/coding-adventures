//! Run-scoped identity observation at actual typed array construction.
//!
//! The grammar occurrence is borrowed rather than looked up by spelling/span.
//! Context travels with recursive conversion; no registry, second source walk
//! or global state can confuse repeated equal literals or independent runs.
//! A sink is an identity policy, not certified source/graph evidence. The future
//! trusted CLI adapter must provide those stronger checks separately.
use super::{BridgeError, GrammarASTNode, Program};
use coding_adventures_javascript_tokens::EsVersion;

/// A caller-owned array identity policy, not a trusted source-span certificate.
/// The bridge may assign only the returned array CV; it cannot validate arbitrary
/// caller strings or bound arbitrary Rust callback work.
pub trait ArrayOccurrenceObserver: Send {
    /// Called exactly once, after the array's elements have been converted.
    /// Return an optional caller-owned identity; a refusal rejects the observed
    /// entry with the original error and prevents subsequent callback calls.
    fn array_constructed(&mut self, node: &GrammarASTNode) -> Result<Option<String>, String>;
}

/// A single caller-owned sink for arrays and each actual typed member prefix.
/// The end index is inclusive in the borrowed owner's raw children, and is
/// evidence of construction rather than a certified physical source range.
pub trait CompositeOccurrenceObserver: ArrayOccurrenceObserver {
    fn member_constructed(
        &mut self,
        owner: &GrammarASTNode,
        end_child: usize,
        computed: bool,
        optional: bool,
    ) -> Result<Option<String>, String>;
}

/// CLOC34 delegating scaffold: exposes the API for an honest compiled red.
/// Member construction observation is deliberately not implemented yet.
pub fn grammar_to_program_with_composite_observer(
    node: &GrammarASTNode,
    version: EsVersion,
    observer: &mut dyn CompositeOccurrenceObserver,
) -> Result<Program, ObservedBridgeError> {
    grammar_to_program_with_array_observer(node, version, observer)
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

/// One invocation owns its first refusal. The outer boundary checks this latch
/// even if a future converter catches the internal propagation sentinel.
pub(super) struct Context<'a> {
    observer: Option<&'a mut dyn ArrayOccurrenceObserver>,
    first_error: Option<String>,
}

impl<'a> Context<'a> {
    pub(super) fn unobserved() -> Self {
        Self { observer: None, first_error: None }
    }

    fn observed(observer: &'a mut dyn ArrayOccurrenceObserver) -> Self {
        Self { observer: Some(observer), first_error: None }
    }

    pub(super) fn array_constructed(&mut self, node: &GrammarASTNode) -> Result<Option<String>, BridgeError> {
        if self.first_error.is_some() {
            return Err(super::internal(node, "array occurrence observer already refused construction"));
        }
        let Some(observer) = self.observer.as_deref_mut() else { return Ok(None); };
        match observer.array_constructed(node) {
            Ok(id) => Ok(id),
            Err(message) => {
                self.first_error = Some(message);
                Err(super::internal(node, "array occurrence observer refused construction"))
            }
        }
    }

    fn finish(self, result: Result<Program, BridgeError>) -> Result<Program, ObservedBridgeError> {
        if let Some(message) = self.first_error {
            if let Ok(program) = result {
                // This stays on the worker too; reuse the existing exhaustive
                // iterative disposer rather than introducing another AST walk.
                coding_adventures_javascript_ast::dispose_program(program);
            }
            return Err(ObservedBridgeError::Observer(message));
        }
        result.map_err(ObservedBridgeError::Bridge)
    }
}

/// Observe array construction without changing the legacy facade or its errors.
///
/// The scoped worker borrows the CST and Send sink. Partial candidates rejected
/// in a converter are dropped on its large stack, before an error returns to a
/// small caller. Worker creation/join failure never degrades to unobserved
/// success. Successful AST ownership and arbitrary caller CST depth retain
/// their existing caller responsibilities; arbitrary Rust sinks are not sandboxed.
pub fn grammar_to_program_with_array_observer(
    node: &GrammarASTNode,
    version: EsVersion,
    observer: &mut dyn ArrayOccurrenceObserver,
) -> Result<Program, ObservedBridgeError> {
    if node.rule_name != "program" {
        return Err(ObservedBridgeError::Bridge(super::internal(node,
            format!("expected root rule 'program', got '{}'", node.rule_name))));
    }
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("javascript-array-bridge".into())
            .stack_size(128 * 1024 * 1024)
            .spawn_scoped(scope, move || {
                let mut context = Context::observed(observer);
                let result = super::convert_program(&mut context, node, version);
                context.finish(result)
            })
            .map_err(|error| ObservedBridgeError::Worker(error.to_string()))?;
        worker.join().map_err(|_| ObservedBridgeError::Worker("worker panicked".into()))?
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_first_refusal_survives_caught_sentinel_and_blocks_both_methods() {
        struct Refuse { arrays: usize, members: usize }
        impl ArrayOccurrenceObserver for Refuse {
            fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> {
                self.arrays += 1; Err("first-array".into())
            }
        }
        impl CompositeOccurrenceObserver for Refuse {
            fn member_constructed(&mut self, _: &GrammarASTNode, _: usize, _: bool, _: bool) -> Result<Option<String>, String> {
                self.members += 1; Err("first-member".into())
            }
        }
        let node = crate::parse_javascript_typed("a.b;", crate::DEFAULT_ES_VERSION).unwrap();
        fn owner(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
            if node.rule_name == "member_expression" && node.children.len() == 3 { return Some(node); }
            node.children.iter().find_map(|c| match c {
                super::super::ASTNodeOrToken::Node(n) => owner(n), _ => None,
            })
        }
        let member = owner(&node).unwrap();
        for first_array in [false, true] {
            let mut sink = Refuse { arrays: 0, members: 0 };
            let mut context = Context::composite(&mut sink);
            if first_array { assert!(context.array_constructed(&node).is_err()); }
            else { assert!(context.member_constructed(member, 2, false, false).is_err()); }
            assert!(context.member_constructed(member, 2, false, false).is_err());
            assert!(context.array_constructed(&node).is_err());
            let candidate = super::super::grammar_to_program(&node, crate::DEFAULT_ES_VERSION).unwrap();
            assert!(matches!(context.finish(Ok(candidate)), Err(ObservedBridgeError::Observer(s))
                if s == if first_array { "first-array" } else { "first-member" }));
            assert_eq!((sink.arrays, sink.members), if first_array { (1, 0) } else { (0, 1) });
        }
    }

    #[test]
    fn invalid_composite_endpoint_latches_before_sink_and_suppresses_arrays() {
        struct Sink(usize);
        impl ArrayOccurrenceObserver for Sink {
            fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> {
                self.0 += 1; Ok(None)
            }
        }
        impl CompositeOccurrenceObserver for Sink {
            fn member_constructed(&mut self, _: &GrammarASTNode, _: usize, _: bool, _: bool) -> Result<Option<String>, String> {
                self.0 += 1; Ok(None)
            }
        }
        let node = crate::parse_javascript_typed("[];", crate::DEFAULT_ES_VERSION).unwrap();
        for (computed, expected) in [(true, "member occurrence endpoint is not a direct closing bracket"),
            (false, "member occurrence endpoint is not a direct property token")] {
            let mut sink = Sink(0);
            let mut context = Context::composite(&mut sink);
            assert!(context.member_constructed(&node, usize::MAX, computed, false).is_err());
            assert!(context.array_constructed(&node).is_err());
            let candidate = super::super::grammar_to_program(&node, crate::DEFAULT_ES_VERSION).unwrap();
            assert!(matches!(context.finish(Ok(candidate)), Err(ObservedBridgeError::Observer(s)) if s == expected));
            assert_eq!(sink.0, 0);
        }
    }

    #[test]
    fn first_refusal_survives_a_caught_converter_sentinel_and_blocks_later_calls() {
        struct Refuse(usize);
        impl ArrayOccurrenceObserver for Refuse {
            fn array_constructed(&mut self, _: &GrammarASTNode) -> Result<Option<String>, String> {
                self.0 += 1;
                Err(format!("first-error-{}", self.0))
            }
        }
        let node = crate::parse_javascript_typed("[];", crate::DEFAULT_ES_VERSION).unwrap();
        let mut observer = Refuse(0);
        let mut context = Context::observed(&mut observer);
        assert!(context.array_constructed(&node).is_err());
        assert!(context.array_constructed(&node).is_err());
        let candidate = super::super::grammar_to_program(&node, crate::DEFAULT_ES_VERSION).unwrap();
        assert!(matches!(context.finish(Ok(candidate)),
            Err(ObservedBridgeError::Observer(s)) if s == "first-error-1"));
        assert_eq!(observer.0, 1);
    }
}

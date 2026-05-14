#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

/// Three-state result produced by an operation callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationResult<T> {
    pub did_succeed: bool,
    pub did_fail_unexpectedly: bool,
    pub return_value: T,
    pub error: Option<String>,
}

impl<T> OperationResult<T> {
    pub fn success(value: T) -> Self {
        Self {
            did_succeed: true,
            did_fail_unexpectedly: false,
            return_value: value,
            error: None,
        }
    }

    pub fn expected_failure(value: T, error: impl Into<String>) -> Self {
        Self {
            did_succeed: false,
            did_fail_unexpectedly: false,
            return_value: value,
            error: Some(error.into()),
        }
    }

    pub fn unexpected_failure(value: T, error: impl Into<String>) -> Self {
        Self {
            did_succeed: false,
            did_fail_unexpectedly: true,
            return_value: value,
            error: Some(error.into()),
        }
    }

    pub fn from_parts(
        did_succeed: bool,
        did_fail_unexpectedly: bool,
        value: T,
        error: Option<String>,
    ) -> Self {
        Self {
            did_succeed,
            did_fail_unexpectedly,
            return_value: value,
            error,
        }
    }
}

/// Creates [`OperationResult`] values inside callbacks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResultFactory<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> ResultFactory<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn generate(
        &self,
        did_succeed: bool,
        did_fail_unexpectedly: bool,
        value: T,
    ) -> OperationResult<T> {
        OperationResult::from_parts(did_succeed, did_fail_unexpectedly, value, None)
    }

    pub fn succeed(&self, value: T) -> OperationResult<T> {
        OperationResult::success(value)
    }

    pub fn fail(&self, value: T, error: impl Into<String>) -> OperationResult<T> {
        OperationResult::expected_failure(value, error)
    }

    pub fn fail_unexpectedly(&self, value: T, error: impl Into<String>) -> OperationResult<T> {
        OperationResult::unexpected_failure(value, error)
    }
}

/// Mutable callback context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationScope {
    name: String,
    property_bag: BTreeMap<String, String>,
}

impl OperationScope {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            property_bag: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn add_property(&mut self, name: impl Into<String>, value: impl ToString) {
        self.property_bag.insert(name.into(), value.to_string());
    }

    pub fn properties(&self) -> &BTreeMap<String, String> {
        &self.property_bag
    }
}

/// Error kind for operation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationErrorKind {
    Expected,
    Unexpected,
}

/// Error returned when an operation does not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationError {
    pub name: String,
    pub kind: OperationErrorKind,
    pub message: String,
    pub properties: BTreeMap<String, String>,
}

impl OperationError {
    pub fn is_expected(&self) -> bool {
        self.kind == OperationErrorKind::Expected
    }

    pub fn is_unexpected(&self) -> bool {
        self.kind == OperationErrorKind::Unexpected
    }
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            OperationErrorKind::Expected => {
                write!(f, "operation {:?} failed: {}", self.name, self.message)
            }
            OperationErrorKind::Unexpected => write!(
                f,
                "operation {:?} failed unexpectedly: {}",
                self.name, self.message
            ),
        }
    }
}

impl std::error::Error for OperationError {}

/// Outcome preserving Go-style `(value, error)` semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationOutcome<T> {
    pub value: T,
    pub error: Option<OperationError>,
}

impl<T> OperationOutcome<T> {
    pub fn into_result(self) -> Result<T, OperationError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.value),
        }
    }
}

/// A named unit of work.
pub struct Operation<T, F>
where
    F: FnOnce(&mut OperationScope, &ResultFactory<T>) -> OperationResult<T>,
{
    name: String,
    fallback: T,
    callback: Option<F>,
    re_panic: bool,
}

impl<T, F> Operation<T, F>
where
    F: FnOnce(&mut OperationScope, &ResultFactory<T>) -> OperationResult<T>,
{
    pub fn panic_on_unexpected(mut self) -> Self {
        self.re_panic = true;
        self
    }

    pub fn get_outcome(mut self) -> OperationOutcome<T> {
        let name = self.name.clone();
        let mut scope = OperationScope::new(name.clone());
        let rf = ResultFactory::<T>::new();
        let callback = self
            .callback
            .take()
            .expect("operation callback should be present exactly once");

        let result = catch_unwind(AssertUnwindSafe(|| callback(&mut scope, &rf)));
        let properties = scope.property_bag;

        let operation_result = match result {
            Ok(operation_result) => operation_result,
            Err(panic_value) => {
                if self.re_panic {
                    resume_unwind(panic_value);
                }
                OperationResult::unexpected_failure(
                    self.fallback,
                    "callback panicked before producing an operation result",
                )
            }
        };

        if operation_result.did_succeed {
            return OperationOutcome {
                value: operation_result.return_value,
                error: None,
            };
        }

        let kind = if operation_result.did_fail_unexpectedly {
            OperationErrorKind::Unexpected
        } else {
            OperationErrorKind::Expected
        };
        let message = operation_result.error.unwrap_or_else(|| match kind {
            OperationErrorKind::Expected => "operation failed".to_string(),
            OperationErrorKind::Unexpected => "operation failed unexpectedly".to_string(),
        });

        OperationOutcome {
            value: operation_result.return_value,
            error: Some(OperationError {
                name,
                kind,
                message,
                properties,
            }),
        }
    }

    pub fn get_result(self) -> Result<T, OperationError> {
        self.get_outcome().into_result()
    }
}

/// Create an operation without executing it.
pub fn start_new<T, F>(name: impl Into<String>, fallback: T, callback: F) -> Operation<T, F>
where
    F: FnOnce(&mut OperationScope, &ResultFactory<T>) -> OperationResult<T>,
{
    Operation {
        name: name.into(),
        fallback,
        callback: Some(callback),
        re_panic: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_operation_returns_value() {
        let value = start_new("math.add", 0, |op, rf| {
            op.add_property("lhs", 2);
            op.add_property("rhs", 3);
            rf.succeed(5)
        })
        .get_result()
        .unwrap();

        assert_eq!(value, 5);
    }

    #[test]
    fn expected_failure_preserves_fallback_value_and_properties() {
        let outcome = start_new("fs.read", Vec::<u8>::new(), |op, rf| {
            op.add_property("path", "/etc/passwd");
            rf.fail(Vec::new(), "capability denied")
        })
        .get_outcome();

        assert_eq!(outcome.value, Vec::<u8>::new());
        let error = outcome.error.expect("expected failure should carry error");
        assert!(error.is_expected());
        assert_eq!(error.message, "capability denied");
        assert_eq!(
            error.properties.get("path").map(String::as_str),
            Some("/etc/passwd")
        );
    }

    #[test]
    fn unexpected_failure_can_be_returned_without_panic() {
        let error = start_new("planner.lower", "fallback".to_string(), |_op, rf| {
            rf.fail_unexpectedly("fallback".to_string(), "internal invariant broke")
        })
        .get_result()
        .unwrap_err();

        assert!(error.is_unexpected());
        assert_eq!(error.message, "internal invariant broke");
    }

    #[test]
    fn panic_becomes_unexpected_failure_by_default() {
        let outcome = start_new("panic.catcher", 42, |_op, _rf| -> OperationResult<i32> {
            panic!("boom")
        })
        .get_outcome();

        assert_eq!(outcome.value, 42);
        let error = outcome.error.expect("panic should become error");
        assert!(error.is_unexpected());
        assert!(error.message.contains("panicked"));
    }

    #[test]
    #[should_panic(expected = "boom")]
    fn panic_on_unexpected_rethrows_callback_panic() {
        let _ = start_new("panic.rethrow", 0, |_op, _rf| -> OperationResult<i32> {
            panic!("boom")
        })
        .panic_on_unexpected()
        .get_result();
    }
}

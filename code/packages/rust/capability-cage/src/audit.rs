//! Audit envelope types for capability-checked operations.

use std::fmt;

use crate::{Action, CapabilityViolationError, Category};

/// The cage decision for one attempted operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationDecision {
    Allowed,
    Denied,
}

impl OperationDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
        }
    }

    pub fn is_allowed(self) -> bool {
        self == Self::Allowed
    }
}

impl fmt::Display for OperationDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Metadata for a capability decision, safe to persist in audit trails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRecord {
    pub category: Category,
    pub action: Action,
    pub target: String,
    pub decision: OperationDecision,
    pub message: Option<String>,
}

impl OperationRecord {
    pub fn allowed(category: Category, action: Action, target: impl Into<String>) -> Self {
        Self {
            category,
            action,
            target: target.into(),
            decision: OperationDecision::Allowed,
            message: None,
        }
    }

    pub fn denied(violation: CapabilityViolationError) -> Self {
        Self {
            category: violation.category,
            action: violation.action,
            target: violation.target,
            decision: OperationDecision::Denied,
            message: Some(violation.message),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn is_allowed(&self) -> bool {
        self.decision.is_allowed()
    }

    pub fn capability_key(&self) -> String {
        format!("{}:{}:{}", self.category, self.action, self.target)
    }
}

/// Result envelope for one capability-checked operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation<T> {
    pub record: OperationRecord,
    pub output: Option<T>,
}

impl<T> Operation<T> {
    pub fn allowed(
        category: Category,
        action: Action,
        target: impl Into<String>,
        output: T,
    ) -> Self {
        Self {
            record: OperationRecord::allowed(category, action, target),
            output: Some(output),
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Operation<U> {
        Operation {
            record: self.record,
            output: self.output.map(f),
        }
    }

    pub fn is_allowed(&self) -> bool {
        self.record.is_allowed()
    }

    pub fn audit_record(&self) -> &OperationRecord {
        &self.record
    }

    pub fn record_to(&self, sink: &mut impl AuditSink) {
        sink.record(self.record.clone());
    }
}

impl Operation<()> {
    pub fn denied(violation: CapabilityViolationError) -> Self {
        Self {
            record: OperationRecord::denied(violation),
            output: None,
        }
    }
}

/// Minimal sink contract used by wrappers and host runtimes.
pub trait AuditSink {
    fn record(&mut self, record: OperationRecord);
}

/// Audit sink for callers that deliberately discard audit events.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn record(&mut self, _record: OperationRecord) {}
}

/// Deterministic in-memory sink for tests and embedded runtimes.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct VecAuditSink {
    records: Vec<OperationRecord>,
}

impl VecAuditSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> &[OperationRecord] {
        &self.records
    }

    pub fn into_records(self) -> Vec<OperationRecord> {
        self.records
    }
}

impl AuditSink for VecAuditSink {
    fn record(&mut self, record: OperationRecord) {
        self.records.push(record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_operation_records_capability_and_output() {
        let operation = Operation::allowed(Category::Fs, Action::Read, "./input.txt", b"hello");

        assert!(operation.is_allowed());
        assert_eq!(operation.output, Some(b"hello"));
        assert_eq!(
            operation.audit_record().capability_key(),
            "fs:read:./input.txt"
        );
        assert_eq!(operation.audit_record().decision.as_str(), "allowed");
    }

    #[test]
    fn denied_operation_carries_violation_message_without_output() {
        let violation = CapabilityViolationError {
            category: Category::Net,
            action: Action::Connect,
            target: "api.example.test:443".to_string(),
            message: "net connect denied".to_string(),
        };

        let operation = Operation::denied(violation);

        assert!(!operation.is_allowed());
        assert_eq!(operation.output, None);
        assert_eq!(
            operation.audit_record().capability_key(),
            "net:connect:api.example.test:443"
        );
        assert_eq!(
            operation.audit_record().message.as_deref(),
            Some("net connect denied")
        );
    }

    #[test]
    fn operation_output_can_be_mapped_without_changing_audit_record() {
        let operation = Operation::allowed(Category::Fs, Action::List, "./fixtures", vec![1, 2, 3])
            .map(|items| items.len());

        assert_eq!(operation.output, Some(3));
        assert_eq!(
            operation.audit_record().capability_key(),
            "fs:list:./fixtures"
        );
    }

    #[test]
    fn vec_audit_sink_records_ordered_operation_records() {
        let mut sink = VecAuditSink::new();
        Operation::allowed(Category::Time, Action::Read, "clock", 123).record_to(&mut sink);
        Operation::<()>::denied(CapabilityViolationError {
            category: Category::Stdout,
            action: Action::Write,
            target: "terminal".to_string(),
            message: "stdout denied".to_string(),
        })
        .record_to(&mut sink);

        let records = sink.records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].decision, OperationDecision::Allowed);
        assert_eq!(records[1].decision, OperationDecision::Denied);
        assert_eq!(records[1].message.as_deref(), Some("stdout denied"));
    }
}

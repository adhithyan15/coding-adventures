//! Observable event trace for microarchitectural side channel and constant-time verification (VER26 §8.1, CWE-208).

use crate::ast::{BlockId, ProvenanceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TraceEvent {
    BranchDecision {
        from_block: BlockId,
        target_block: BlockId,
    },
    MemoryAccess {
        provenance: ProvenanceId,
        offset: u64,
        kind: AccessKind,
        size: usize,
    },
    CapabilityInvocation {
        capability: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ObservableTrace {
    pub events: Vec<TraceEvent>,
}

impl ObservableTrace {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_branch(&mut self, from_block: BlockId, target_block: BlockId) {
        self.events.push(TraceEvent::BranchDecision {
            from_block,
            target_block,
        });
    }

    pub fn record_memory(
        &mut self,
        provenance: ProvenanceId,
        offset: u64,
        kind: AccessKind,
        size: usize,
    ) {
        self.events.push(TraceEvent::MemoryAccess {
            provenance,
            offset,
            kind,
            size,
        });
    }

    pub fn record_capability(&mut self, capability: &str) {
        self.events.push(TraceEvent::CapabilityInvocation {
            capability: capability.to_string(),
        });
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn is_identical_to(&self, other: &Self) -> bool {
        self.events == other.events
    }
}

//! Deterministic injection at the production allocation call sites.

use std::cell::Cell;

/// Exact checked allocation boundaries in the gate package.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationSite {
    InitialStorage,
    StateSnapshot,
    LoadImage,
    RestoreMemory,
    RawInstruction,
    Description,
    MemoryWrites,
    OracleState,
    TraceReservation,
    TraceRetention,
    FinalState,
}

thread_local! {
    static FAILURE: Cell<Option<(AllocationSite, usize)>> = const { Cell::new(None) };
}

/// Configure one deterministic failure after `skip` matching call sites.
///
/// This hook changes only whether the adjacent real `try_reserve_exact` call
/// is attempted. It never supplies architectural data or affects decode.
#[doc(hidden)]
pub fn fail_allocation(site: AllocationSite, skip: usize) {
    FAILURE.with(|failure| failure.set(Some((site, skip))));
}

/// Disable deterministic allocation failure injection.
#[doc(hidden)]
pub fn clear_allocation_failure() {
    FAILURE.with(|failure| failure.set(None));
}

pub(crate) fn checkpoint(site: AllocationSite) -> Result<(), ()> {
    FAILURE.with(|failure| match failure.get() {
        Some((configured, 0)) if configured == site => {
            failure.set(None);
            Err(())
        }
        Some((configured, remaining)) if configured == site => {
            failure.set(Some((configured, remaining - 1)));
            Ok(())
        }
        _ => Ok(()),
    })
}

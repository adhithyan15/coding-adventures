//! [`BreakpointManager`] — tracks user-set breakpoints per file.
//!
//! ## Model
//!
//! The editor sends `setBreakpoints { source.path, breakpoints: [{line}, …] }`
//! and we replace the entire breakpoint list for that file in one shot.
//!
//! For each requested source line we ask [`SidecarIndex`] for every reachable
//! [`VmLocation`] on that line and install a VM breakpoint at each.  This is
//! a "splatter" strategy: any path through the source line will trip.
//!
//! When a previously-set breakpoint no longer appears in a `setBreakpoints`
//! request, we remove its VM breakpoints.
//!
//! ## State
//!
//! ```text
//! file_breakpoints:  PathBuf → Vec<UserBreakpoint { line, vm_locs, verified }>
//! ```
//!
//! `vm_locs` is the snapshot of `SidecarIndex::source_to_locs(file, line)`
//! taken when the breakpoint was installed.  We re-use it on removal so we
//! don't have to query the sidecar twice.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::sidecar::SidecarIndex;
use crate::vm_conn::VmLocation;

/// VM-level changes the caller (DapServer) must apply after a
/// `set_breakpoints` call: clear the old set, install the new one.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BreakpointDiff {
    /// VM locations whose breakpoints should be cleared.
    pub to_clear:   Vec<VmLocation>,
    /// VM locations where new breakpoints should be installed.
    pub to_install: Vec<VmLocation>,
}

/// One user-set breakpoint on a specific source line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBreakpoint {
    /// 1-based line in the source file.
    pub line: u32,
    /// VM locations installed for this breakpoint.
    pub vm_locs: Vec<VmLocation>,
    /// `true` if the sidecar resolved the source line to ≥ 1 VM location;
    /// the editor uses this to grey-out unverified breakpoints.
    pub verified: bool,
}

/// Tracks the active breakpoint set across all files.
#[derive(Debug, Default)]
pub struct BreakpointManager {
    file_breakpoints: HashMap<PathBuf, Vec<UserBreakpoint>>,
}

impl BreakpointManager {
    /// Build an empty manager.
    pub fn new() -> Self {
        BreakpointManager::default()
    }

    /// Replace the breakpoints for `file` with `lines`.
    ///
    /// Returns the new [`UserBreakpoint`] list (used to build the
    /// `setBreakpoints` DAP response) plus a [`BreakpointDiff`] describing
    /// the VM-level changes the caller must apply.
    ///
    /// Decoupling the manager from `VmConnection` like this side-steps the
    /// `dyn` lifetime variance issue when the server holds the connection in
    /// `Option<Box<dyn VmConnection>>`, and makes this function trivially
    /// testable without a mock.
    pub fn set_breakpoints(
        &mut self,
        file: &PathBuf,
        lines: &[u32],
        sidecar: Option<&SidecarIndex>,
    ) -> (Vec<UserBreakpoint>, BreakpointDiff) {
        // ----- 1. Tear down existing breakpoints for this file ------------
        let prev = self.file_breakpoints.remove(file).unwrap_or_default();
        let to_clear: Vec<VmLocation> = prev.iter()
            .flat_map(|b| b.vm_locs.iter().cloned())
            .collect();

        // ----- 2. Resolve new breakpoints --------------------------------
        let file_str = file.to_string_lossy().to_string();
        let mut new_bps: Vec<UserBreakpoint> = Vec::with_capacity(lines.len());
        let mut to_install: Vec<VmLocation> = Vec::new();

        for &line in lines {
            let vm_locs: Vec<VmLocation> = sidecar
                .map(|sc| sc.source_to_locs(&file_str, line))
                .unwrap_or_default();
            let verified = !vm_locs.is_empty();
            to_install.extend(vm_locs.iter().cloned());
            new_bps.push(UserBreakpoint { line, vm_locs, verified });
        }

        if !new_bps.is_empty() {
            self.file_breakpoints.insert(file.clone(), new_bps.clone());
        }
        (new_bps, BreakpointDiff { to_clear, to_install })
    }

    /// Find the breakpoint (if any) that the VM hit at `loc`.
    ///
    /// Returns `Some((file, line))` when the location matches a stored user
    /// breakpoint, so the adapter can include the source position in the
    /// `stopped` event payload.
    pub fn find_hit(&self, loc: &VmLocation) -> Option<(PathBuf, u32)> {
        for (file, bps) in &self.file_breakpoints {
            for bp in bps {
                if bp.vm_locs.contains(loc) {
                    return Some((file.clone(), bp.line));
                }
            }
        }
        None
    }

    /// All currently-installed breakpoints (useful for tests and logging).
    pub fn all(&self) -> Vec<(PathBuf, UserBreakpoint)> {
        self.file_breakpoints
            .iter()
            .flat_map(|(f, bps)| bps.iter().map(move |b| (f.clone(), b.clone())))
            .collect()
    }

    /// Number of user breakpoints across all files.
    pub fn len(&self) -> usize {
        self.file_breakpoints.values().map(|v| v.len()).sum()
    }

    /// `true` if no breakpoints are installed.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use debug_sidecar::DebugSidecarWriter;

    fn small_sidecar() -> SidecarIndex {
        let mut w = DebugSidecarWriter::new();
        let fid = w.add_source_file("p.tw", b"");
        w.begin_function("main", 0, 0);
        w.record("main", 0, fid, 1, 1);
        w.record("main", 1, fid, 2, 1);
        w.record("main", 2, fid, 3, 1);
        w.end_function("main", 3);
        SidecarIndex::from_bytes(&w.finish()).unwrap()
    }

    #[test]
    fn empty_manager() {
        let m = BreakpointManager::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
    }

    #[test]
    fn set_breakpoints_returns_install_diff() {
        let mut m = BreakpointManager::new();
        let sc = small_sidecar();
        let (bps, diff) = m.set_breakpoints(
            &PathBuf::from("p.tw"),
            &[1, 2],
            Some(&sc),
        );
        assert_eq!(bps.len(), 2);
        assert!(bps[0].verified);
        assert!(bps[1].verified);
        assert!(diff.to_clear.is_empty());
        assert_eq!(diff.to_install.len(), 2);
    }

    #[test]
    fn unresolvable_line_marked_unverified_no_install() {
        let mut m = BreakpointManager::new();
        let sc = small_sidecar();
        let (bps, diff) = m.set_breakpoints(
            &PathBuf::from("p.tw"),
            &[99],
            Some(&sc),
        );
        assert_eq!(bps.len(), 1);
        assert!(!bps[0].verified);
        assert!(diff.to_install.is_empty());
    }

    #[test]
    fn re_set_breakpoints_clears_previous_in_diff() {
        let mut m = BreakpointManager::new();
        let sc = small_sidecar();
        let f = PathBuf::from("p.tw");

        let (_, _) = m.set_breakpoints(&f, &[1], Some(&sc));
        let (_, diff) = m.set_breakpoints(&f, &[2, 3], Some(&sc));
        assert_eq!(diff.to_clear.len(), 1, "old BP cleared");
        assert_eq!(diff.to_install.len(), 2, "two new BPs installed");
    }

    #[test]
    fn empty_breakpoint_list_clears_file() {
        let mut m = BreakpointManager::new();
        let sc = small_sidecar();
        let f = PathBuf::from("p.tw");

        m.set_breakpoints(&f, &[1, 2], Some(&sc));
        assert_eq!(m.len(), 2);

        let (after, diff) = m.set_breakpoints(&f, &[], Some(&sc));
        assert!(after.is_empty());
        assert_eq!(m.len(), 0);
        assert_eq!(diff.to_clear.len(), 2);
        assert!(diff.to_install.is_empty());
    }

    #[test]
    fn find_hit_finds_breakpoint() {
        let mut m = BreakpointManager::new();
        let sc = small_sidecar();
        m.set_breakpoints(&PathBuf::from("p.tw"), &[2], Some(&sc));
        let hit = m.find_hit(&VmLocation::new("main", 1));
        assert_eq!(hit, Some((PathBuf::from("p.tw"), 2)));
    }

    #[test]
    fn find_hit_returns_none_when_no_match() {
        let m = BreakpointManager::new();
        assert!(m.find_hit(&VmLocation::new("foo", 0)).is_none());
    }

    #[test]
    fn works_without_sidecar() {
        // Adapter accepts setBreakpoints before launch.
        let mut m = BreakpointManager::new();
        let (bps, diff) = m.set_breakpoints(
            &PathBuf::from("p.tw"),
            &[1, 2, 3],
            None,
        );
        assert!(bps.iter().all(|b| !b.verified));
        assert!(diff.to_install.is_empty());
    }

    #[test]
    fn all_returns_all_breakpoints() {
        let mut m = BreakpointManager::new();
        let sc = small_sidecar();
        m.set_breakpoints(&PathBuf::from("p.tw"), &[1, 2], Some(&sc));
        let all = m.all();
        assert_eq!(all.len(), 2);
    }
}

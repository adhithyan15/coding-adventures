//! Modal State Machine -- multiple sub-machines with mode switching.
//!
//! # What is a Modal State Machine?
//!
//! A modal state machine is a collection of named sub-machines (modes), each
//! a DFA, with transitions that switch between them. When a mode switch
//! occurs, the active sub-machine changes.
//!
//! Think of it like a text editor with Normal, Insert, and Visual modes. Each
//! mode handles keystrokes differently, and certain keys switch between modes.
//!
//! # Why modal machines matter
//!
//! The most important use case is **context-sensitive tokenization**. Consider
//! HTML: the characters `p > .foo { color: red; }` mean completely different
//! things depending on whether they appear inside a `<style>` tag (CSS) or
//! in normal text.
//!
//! A modal state machine solves this: the HTML tokenizer has modes like
//! DATA, TAG_OPEN, SCRIPT_DATA, and STYLE_DATA. Each mode has its own DFA
//! with its own token rules. Certain tokens (like seeing `<style>`) trigger
//! a mode switch.
//!
//! # Connection to the Chomsky Hierarchy
//!
//! A single DFA recognizes regular languages (Type 3). A modal state machine
//! is more powerful: it can track context (which mode am I in?) and switch
//! rules accordingly. This moves us toward context-sensitive languages
//! (Type 1), though a modal machine is still not as powerful as a full
//! linear-bounded automaton.

use std::collections::HashMap;

use directed_graph::LabeledDirectedGraph;

use crate::dfa::DFA;

/// Record of a mode switch event.
///
/// Captures which mode we switched from and to, and what triggered it.
#[derive(Debug, Clone, PartialEq)]
pub struct ModeTransitionRecord {
    /// The mode we were in before the switch.
    pub from_mode: String,
    /// The event that triggered the switch.
    pub trigger: String,
    /// The mode we switched to.
    pub to_mode: String,
}

/// A collection of named DFA sub-machines with mode transitions.
///
/// Each mode is a DFA that handles inputs within that context. Mode
/// transitions switch which DFA is active. When a mode switch occurs,
/// the new mode's DFA is reset to its initial state.
pub struct ModalStateMachine {
    /// Map of mode names to their DFA sub-machines.
    modes: HashMap<String, DFA>,
    /// Mode transition rules: (current_mode, trigger) -> target_mode.
    mode_transitions: HashMap<(String, String), String>,
    /// Internal graph of mode transitions.
    ///
    /// Each mode is a node. Each mode transition (from_mode, trigger) -> to_mode
    /// becomes a labeled edge from from_mode to to_mode with trigger as the label.
    /// This graph captures the structure of mode switching for potential
    /// introspection and visualization.
    _mode_graph: LabeledDirectedGraph,
    /// The initial mode name.
    initial_mode: String,
    /// The currently active mode.
    current_mode: String,
    /// History of mode switches.
    mode_trace: Vec<ModeTransitionRecord>,
}

impl ModalStateMachine {
    /// Create a new Modal State Machine.
    ///
    /// # Arguments
    ///
    /// * `modes` -- Map of mode names to DFA sub-machines.
    /// * `mode_transitions` -- (current_mode, trigger) -> target_mode.
    /// * `initial_mode` -- The name of the starting mode.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if validation fails.
    pub fn new(
        modes: HashMap<String, DFA>,
        mode_transitions: HashMap<(String, String), String>,
        initial_mode: String,
    ) -> Result<Self, String> {
        if modes.is_empty() {
            return Err("At least one mode must be provided".to_string());
        }
        if !modes.contains_key(&initial_mode) {
            return Err(format!(
                "Initial mode '{}' is not in the modes dict",
                initial_mode
            ));
        }

        // Validate mode transitions
        for ((from_mode, _trigger), to_mode) in &mode_transitions {
            if !modes.contains_key(from_mode) {
                return Err(format!(
                    "Mode transition source '{}' is not a valid mode",
                    from_mode
                ));
            }
            if !modes.contains_key(to_mode) {
                return Err(format!(
                    "Mode transition target '{}' is not a valid mode",
                    to_mode
                ));
            }
        }

        // --- Build internal mode graph ---
        //
        // Each mode is a node. Each mode transition (from_mode, trigger) -> to_mode
        // becomes a labeled edge with the trigger as the label. Self-loops are
        // allowed since a mode can transition to itself.
        let mut mode_graph = LabeledDirectedGraph::new_allow_self_loops();
        for mode_name in modes.keys() {
            mode_graph.add_node(mode_name);
        }
        for ((from_mode, trigger), to_mode) in &mode_transitions {
            let _ = mode_graph.add_edge(from_mode, to_mode, trigger);
        }

        Ok(ModalStateMachine {
            modes,
            mode_transitions,
            _mode_graph: mode_graph,
            initial_mode: initial_mode.clone(),
            current_mode: initial_mode,
            mode_trace: Vec::new(),
        })
    }

    // === Getters ===

    /// The name of the currently active mode.
    pub fn current_mode(&self) -> &str {
        &self.current_mode
    }

    /// The DFA for the current mode.
    pub fn active_machine(&self) -> &DFA {
        &self.modes[&self.current_mode]
    }

    /// All modes and their names.
    pub fn modes(&self) -> &HashMap<String, DFA> {
        &self.modes
    }

    /// The history of mode switches.
    pub fn mode_trace(&self) -> &[ModeTransitionRecord] {
        &self.mode_trace
    }

    // === Processing ===

    /// Process an input event in the current mode's DFA.
    ///
    /// Delegates to the active DFA's `process()` method.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the event is invalid for the current mode.
    pub fn process(&mut self, event: &str) -> Result<String, String> {
        self.modes
            .get_mut(&self.current_mode)
            .unwrap()
            .process(event)
    }

    /// Switch to a different mode based on a trigger event.
    ///
    /// Looks up (current_mode, trigger) in the mode transitions.
    /// If found, switches to the target mode and resets its DFA
    /// to the initial state.
    ///
    /// # Errors
    ///
    /// Returns `Err` if no mode transition exists for this trigger.
    pub fn switch_mode(&mut self, trigger: &str) -> Result<String, String> {
        let key = (self.current_mode.clone(), trigger.to_string());
        let new_mode = self.mode_transitions.get(&key).cloned().ok_or_else(|| {
            format!(
                "No mode transition for (mode='{}', trigger='{}')",
                self.current_mode, trigger
            )
        })?;

        let old_mode = self.current_mode.clone();

        // Reset the target mode's DFA to its initial state
        self.modes.get_mut(&new_mode).unwrap().reset();

        // Record the switch
        self.mode_trace.push(ModeTransitionRecord {
            from_mode: old_mode,
            trigger: trigger.to_string(),
            to_mode: new_mode.clone(),
        });

        self.current_mode = new_mode.clone();
        Ok(new_mode)
    }

    /// Reset to initial mode and reset all sub-machines.
    pub fn reset(&mut self) {
        self.current_mode = self.initial_mode.clone();
        self.mode_trace.clear();
        for dfa in self.modes.values_mut() {
            dfa.reset();
        }
    }
}

impl std::fmt::Display for ModalStateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut mode_names: Vec<_> = self.modes.keys().collect();
        mode_names.sort();
        write!(
            f,
            "ModalStateMachine(modes={:?}, current_mode='{}')",
            mode_names, self.current_mode
        )
    }
}

impl std::fmt::Debug for ModalStateMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_data_mode() -> DFA {
        DFA::new(
            HashSet::from(["text".into(), "tag_detected".into()]),
            HashSet::from(["char".into(), "open_angle".into()]),
            HashMap::from([
                (("text".into(), "char".into()), "text".into()),
                (("text".into(), "open_angle".into()), "tag_detected".into()),
                (("tag_detected".into(), "char".into()), "text".into()),
                (
                    ("tag_detected".into(), "open_angle".into()),
                    "tag_detected".into(),
                ),
            ]),
            "text".into(),
            HashSet::from(["text".into()]),
        )
        .unwrap()
    }

    fn make_tag_mode() -> DFA {
        DFA::new(
            HashSet::from(["reading_name".into(), "tag_done".into()]),
            HashSet::from(["char".into(), "close_angle".into()]),
            HashMap::from([
                (
                    ("reading_name".into(), "char".into()),
                    "reading_name".into(),
                ),
                (
                    ("reading_name".into(), "close_angle".into()),
                    "tag_done".into(),
                ),
                (("tag_done".into(), "char".into()), "reading_name".into()),
                (("tag_done".into(), "close_angle".into()), "tag_done".into()),
            ]),
            "reading_name".into(),
            HashSet::from(["tag_done".into()]),
        )
        .unwrap()
    }

    fn make_script_mode() -> DFA {
        DFA::new(
            HashSet::from(["raw".into()]),
            HashSet::from(["char".into(), "end_marker".into()]),
            HashMap::from([
                (("raw".into(), "char".into()), "raw".into()),
                (("raw".into(), "end_marker".into()), "raw".into()),
            ]),
            "raw".into(),
            HashSet::from(["raw".into()]),
        )
        .unwrap()
    }

    fn html_tokenizer() -> ModalStateMachine {
        ModalStateMachine::new(
            HashMap::from([
                ("data".into(), make_data_mode()),
                ("tag".into(), make_tag_mode()),
                ("script".into(), make_script_mode()),
            ]),
            HashMap::from([
                (("data".into(), "enter_tag".into()), "tag".into()),
                (("tag".into(), "exit_tag".into()), "data".into()),
                (("tag".into(), "enter_script".into()), "script".into()),
                (("script".into(), "exit_script".into()), "data".into()),
            ]),
            "data".into(),
        )
        .unwrap()
    }

    #[test]
    fn test_valid_construction() {
        let modal = html_tokenizer();
        assert_eq!(modal.current_mode(), "data");
        assert_eq!(modal.modes().len(), 3);
    }

    #[test]
    fn test_no_modes_rejected() {
        let result = ModalStateMachine::new(HashMap::new(), HashMap::new(), "data".into());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("one mode"));
    }

    #[test]
    fn test_invalid_initial_mode() {
        let result = ModalStateMachine::new(
            HashMap::from([("data".into(), make_data_mode())]),
            HashMap::new(),
            "missing".into(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Initial mode"));
    }

    #[test]
    fn test_invalid_transition_source() {
        let result = ModalStateMachine::new(
            HashMap::from([("data".into(), make_data_mode())]),
            HashMap::from([(("missing".into(), "trigger".into()), "data".into())]),
            "data".into(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("source"));
    }

    #[test]
    fn test_invalid_transition_target() {
        let result = ModalStateMachine::new(
            HashMap::from([("data".into(), make_data_mode())]),
            HashMap::from([(("data".into(), "trigger".into()), "missing".into())]),
            "data".into(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("target"));
    }

    #[test]
    fn test_switch_mode() {
        let mut modal = html_tokenizer();
        assert_eq!(modal.current_mode(), "data");
        modal.switch_mode("enter_tag").unwrap();
        assert_eq!(modal.current_mode(), "tag");
    }

    #[test]
    fn test_switch_mode_returns_new() {
        let mut modal = html_tokenizer();
        let result = modal.switch_mode("enter_tag").unwrap();
        assert_eq!(result, "tag");
    }

    #[test]
    fn test_switch_resets_target_dfa() {
        let mut modal = html_tokenizer();
        modal.switch_mode("enter_tag").unwrap();
        modal.process("char").unwrap();
        modal.process("close_angle").unwrap();
        assert_eq!(modal.active_machine().current_state(), "tag_done");

        modal.switch_mode("exit_tag").unwrap();
        modal.switch_mode("enter_tag").unwrap();
        assert_eq!(modal.active_machine().current_state(), "reading_name");
    }

    #[test]
    fn test_invalid_trigger() {
        let mut modal = html_tokenizer();
        let result = modal.switch_mode("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No mode transition"));
    }

    #[test]
    fn test_mode_trace() {
        let mut modal = html_tokenizer();
        modal.switch_mode("enter_tag").unwrap();
        modal.switch_mode("exit_tag").unwrap();

        let trace = modal.mode_trace();
        assert_eq!(trace.len(), 2);
        assert_eq!(trace[0].from_mode, "data");
        assert_eq!(trace[0].trigger, "enter_tag");
        assert_eq!(trace[0].to_mode, "tag");
        assert_eq!(trace[1].from_mode, "tag");
        assert_eq!(trace[1].trigger, "exit_tag");
        assert_eq!(trace[1].to_mode, "data");
    }

    #[test]
    fn test_process_in_data_mode() {
        let mut modal = html_tokenizer();
        let result = modal.process("char").unwrap();
        assert_eq!(result, "text");
    }

    #[test]
    fn test_process_in_tag_mode() {
        let mut modal = html_tokenizer();
        modal.switch_mode("enter_tag").unwrap();
        let result = modal.process("char").unwrap();
        assert_eq!(result, "reading_name");
        let result = modal.process("close_angle").unwrap();
        assert_eq!(result, "tag_done");
    }

    #[test]
    fn test_reset() {
        let mut modal = html_tokenizer();
        modal.switch_mode("enter_tag").unwrap();
        modal.reset();
        assert_eq!(modal.current_mode(), "data");
        assert!(modal.mode_trace().is_empty());
    }

    #[test]
    fn test_display() {
        let modal = html_tokenizer();
        let s = format!("{}", modal);
        assert!(s.contains("ModalStateMachine"));
        assert!(s.contains("data"));
    }
}

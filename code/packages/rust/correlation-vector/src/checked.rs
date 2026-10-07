//! Controlled graph ownership, transaction accounting and iterative DAG queries.
//!
//! Mutation checks a prospective usage value before allocation or insertion.
//! Because graph fields are private, a successful transaction is the only way
//! to extend checked history. Validation rebuilds evidence independently at
//! import/query/export boundaries; a cached counter is never a substitute for
//! checking a graph received from another process.
use super::{compact_id_sequence, CVEntry, CVLog, Origin};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

/// Finite resource limits for checked graph operations. Counts include repeated
/// parent edges and deletion records; payload bytes include encoded text/meta.
#[derive(Clone, Debug)]
pub struct GraphLimits {
    pub max_nodes: usize,
    pub max_edges: usize,
    pub max_events: usize,
    pub max_metadata_values: usize,
    pub max_metadata_depth: usize,
    pub max_metadata_bytes: usize,
    pub max_input_bytes: usize,
    pub max_work: usize,
    pub max_output_bytes: usize,
}

impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_nodes: 1_000_000,
            max_edges: 4_000_000,
            max_events: 4_000_000,
            max_metadata_values: 1_000_000,
            max_metadata_depth: 64,
            max_metadata_bytes: 128 * 1024 * 1024,
            max_input_bytes: 128 * 1024 * 1024,
            max_work: 64_000_000,
            max_output_bytes: 512 * 1024 * 1024,
        }
    }
}

pub(super) fn add_bounded(
    current: &mut usize,
    amount: usize,
    cap: usize,
    kind: &str,
) -> Result<(), String> {
    let next = current
        .checked_add(amount)
        .ok_or_else(|| format!("CV {kind} accounting overflow"))?;
    if next > cap {
        return Err(format!("CV {kind} limit exceeded"));
    }
    *current = next;
    Ok(())
}

impl GraphLimits {
    pub(super) fn validate(&self) -> Result<(), String> {
        // Metadata serialization is recursive, so even callers raising other
        // finite caps cannot select a stack-unsafe serialization depth.
        if self.max_metadata_depth > 64 {
            return Err("CV metadata depth limit cannot exceed 64".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Usage {
    pub nodes: usize,
    pub edges: usize,
    pub events: usize,
    pub values: usize,
    pub bytes: usize,
}

pub(super) struct CheckedState {
    pub limits: GraphLimits,
    pub usage: Usage,
}

pub(super) struct Work {
    used: usize,
    cap: usize,
}
impl Work {
    pub fn new(cap: usize) -> Self {
        Self { used: 0, cap }
    }
    pub fn take(&mut self, amount: usize) -> Result<(), String> {
        add_bounded(&mut self.used, amount, self.cap, "operation work")
    }
    pub fn reserve_conversion(&mut self) -> Result<(), String> {
        self.take(self.used)
    }
}

/// JSON string bytes without constructing an escaped copy. All strings are
/// already UTF-8; serde emits non-control UTF-8 directly and escapes controls.
pub(super) fn string_bytes(text: &str) -> Result<usize, String> {
    let mut bytes = 2usize;
    for c in text.bytes() {
        let width = match c {
            b'"' | b'\\' | b'\n' | b'\r' | b'\t' | 8 | 12 => 2,
            0..=31 => 6,
            _ => 1,
        };
        bytes = bytes
            .checked_add(width)
            .ok_or("CV text accounting overflow")?;
    }
    Ok(bytes)
}

struct Payload<'a> {
    usage: Usage,
    limits: &'a GraphLimits,
    work: &'a mut Work,
}
impl Payload<'_> {
    fn bytes(&mut self, amount: usize) -> Result<(), String> {
        add_bounded(
            &mut self.usage.bytes,
            amount,
            self.limits.max_metadata_bytes,
            "metadata bytes",
        )
    }
    fn text(&mut self, text: &str) -> Result<(), String> {
        // Reject an obviously oversized string before scanning its escaping.
        if text.len() > self.limits.max_metadata_bytes {
            return Err("CV metadata bytes limit exceeded".into());
        }
        self.bytes(string_bytes(text)?)
    }
    fn value(&mut self, value: &Value, depth: usize) -> Result<(), String> {
        let mut stack = vec![(value, depth)];
        while let Some((value, depth)) = stack.pop() {
            self.work.take(1)?;
            add_bounded(
                &mut self.usage.values,
                1,
                self.limits.max_metadata_values,
                "metadata values",
            )?;
            if depth > self.limits.max_metadata_depth {
                return Err("CV metadata depth limit exceeded".into());
            }
            match value {
                Value::Null => self.bytes(4)?,
                Value::Bool(v) => self.bytes(if *v { 4 } else { 5 })?,
                Value::Number(n) => self.bytes(n.to_string().len())?,
                Value::String(s) => self.text(s)?,
                Value::Array(items) => {
                    self.bytes(2)?;
                    self.bytes(items.len().saturating_sub(1))?;
                    self.guard_pending(stack.len(), items.len())?;
                    for item in items.iter().rev() {
                        self.work.take(1)?; // enqueue before growing the pending stack
                        stack.push((item, depth + 1));
                    }
                }
                Value::Object(map) => {
                    self.bytes(2)?;
                    self.bytes(map.len().saturating_sub(1))?;
                    self.guard_pending(stack.len(), map.len())?;
                    // Accounting does not need canonical order. Borrow each
                    // field directly, validating its key before any enqueue;
                    // canonical encoding sorts only after this full preflight.
                    for (key, item) in map.iter().rev() {
                        self.work.take(1)?;
                        self.text(key)?;
                        self.bytes(1)?;
                        self.work.take(1)?;
                        stack.push((item, depth + 1));
                    }
                }
            }
        }
        Ok(())
    }
    fn guard_pending(&self, pending: usize, additional: usize) -> Result<(), String> {
        let total = self
            .usage
            .values
            .checked_add(pending)
            .and_then(|n| n.checked_add(additional))
            .ok_or("CV metadata values accounting overflow")?;
        if total > self.limits.max_metadata_values {
            return Err("CV metadata values limit exceeded".into());
        }
        Ok(())
    }
    fn meta(&mut self, meta: &HashMap<String, Value>) -> Result<(), String> {
        self.work.take(1)?;
        add_bounded(
            &mut self.usage.values,
            1,
            self.limits.max_metadata_values,
            "metadata values",
        )?;
        if self.limits.max_metadata_depth == 0 {
            return Err("CV metadata depth limit exceeded".into());
        }
        self.bytes(2)?;
        self.bytes(meta.len().saturating_sub(1))?;
        self.guard_pending(0, meta.len())?;
        for (key, value) in meta {
            self.work.take(1)?;
            self.text(key)?;
            self.bytes(1)?;
            self.value(value, 2)?;
        }
        Ok(())
    }
    fn origin(&mut self, origin: Option<&Origin>) -> Result<(), String> {
        if let Some(o) = origin {
            self.text(&o.source)?;
            self.text(&o.location)?;
            if let Some(t) = &o.timestamp {
                self.text(t)?;
            }
            self.meta(&o.meta)?;
        }
        Ok(())
    }
}

impl CVLog {
    /// Construct a checked, fully recorded compact log with explicit limits.
    pub fn new_checked_compact(limits: GraphLimits) -> Result<Self, String> {
        limits.validate()?;
        let mut log = Self::new_compact(true);
        log.checked = Some(CheckedState {
            limits,
            usage: Usage::default(),
        });
        Ok(log)
    }
    /// Immutable graph storage. Mutations must pass through the log's methods.
    pub fn entries(&self) -> &HashMap<String, CVEntry> {
        &self.entries
    }
    /// Declared first-occurrence order of recorded stages, not execution chronology.
    pub fn pass_order(&self) -> &[String] {
        &self.pass_order
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    /// Generic recording toggles remain supported. A checked full log cannot
    /// discard evidence by disabling storage halfway through a run.
    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), String> {
        if self.checked.is_some() && !enabled {
            return Err("cannot disable checked CV recording".into());
        }
        self.enabled = enabled;
        Ok(())
    }
    pub(super) fn prospective_node(
        &self,
        parents: &[&str],
        origin: Option<&Origin>,
    ) -> Result<Option<Usage>, String> {
        let Some(state) = &self.checked else {
            return Ok(None);
        };
        if !self.enabled {
            return Err("checked CV recording is disabled".into());
        }
        let mut usage = state.usage;
        add_bounded(&mut usage.nodes, 1, state.limits.max_nodes, "nodes")?;
        add_bounded(
            &mut usage.edges,
            parents.len(),
            state.limits.max_edges,
            "parent edges",
        )?;
        let mut work = Work::new(state.limits.max_work);
        work.take(1)?;
        let last = self
            .compact_sequence
            .ok_or("checked CV identity state is missing")?;
        for parent in parents {
            work.take(1)?;
            if !self.entries.contains_key(*parent) {
                return Err("unknown checked CV parent".into());
            }
            if compact_id_sequence(parent)? > last {
                return Err("checked CV parent exceeds allocation state".into());
            }
        }
        let mut payload = Payload {
            usage,
            limits: &state.limits,
            work: &mut work,
        };
        payload.origin(origin)?;
        Ok(Some(payload.usage))
    }
    pub(super) fn prospective_event(
        &self,
        id: &str,
        source: &str,
        tag: &str,
        meta: &HashMap<String, Value>,
    ) -> Result<Option<Usage>, String> {
        let Some(state) = &self.checked else {
            return Ok(None);
        };
        if !self.enabled {
            return Err("checked CV recording is disabled".into());
        }
        let entry = self.entries.get(id).ok_or("unknown checked CV identity")?;
        if entry.deleted.is_some() {
            return Err("checked CV identity is already deleted".into());
        }
        let mut usage = state.usage;
        add_bounded(&mut usage.events, 1, state.limits.max_events, "events")?;
        let mut work = Work::new(state.limits.max_work);
        work.take(1)?;
        let mut payload = Payload {
            usage,
            limits: &state.limits,
            work: &mut work,
        };
        payload.text(source)?;
        payload.text(tag)?;
        payload.meta(meta)?;
        if !self.pass_sources.contains(source) {
            payload.text(source)?;
        }
        Ok(Some(payload.usage))
    }
    pub(super) fn commit_usage(&mut self, usage: Option<Usage>) {
        if let Some(usage) = usage {
            self.checked.as_mut().expect("checked transaction").usage = usage;
        }
    }
    pub(super) fn record_source(&mut self, source: &str) {
        if self.pass_sources.insert(source.to_string()) {
            self.pass_order.push(source.to_string());
        }
    }
    fn limits(&self) -> GraphLimits {
        self.checked
            .as_ref()
            .map(|s| s.limits.clone())
            .unwrap_or_default()
    }
    /// Checked exports validate independently; compatibility snapshots retain
    /// allocator-only semantics, but their payload and encoding are still bounded.
    pub(super) fn export_work(&self) -> Result<(GraphLimits, Work), String> {
        let limits = self.limits();
        if self.checked.is_some() {
            let graph = self.validated_graph(&limits, true)?;
            return Ok((limits, graph.work));
        }
        let mut work = Work::new(limits.max_work);
        let mut payload = Payload {
            usage: Usage::default(),
            limits: &limits,
            work: &mut work,
        };
        if self.entries.len() > limits.max_nodes {
            return Err("CV nodes limit exceeded".into());
        }
        for (key, entry) in &self.entries {
            payload.work.take(1)?;
            add_bounded(&mut payload.usage.nodes, 1, limits.max_nodes, "nodes")?;
            add_bounded(
                &mut payload.usage.edges,
                entry.parent_ids.len(),
                limits.max_edges,
                "parent edges",
            )?;
            payload.text(key)?;
            payload.text(&entry.id)?;
            payload.origin(entry.origin.as_ref())?;
            for parent in &entry.parent_ids {
                payload.work.take(1)?;
                payload.text(parent)?;
            }
            for event in &entry.contributions {
                payload.work.take(1)?;
                add_bounded(&mut payload.usage.events, 1, limits.max_events, "events")?;
                payload.text(&event.source)?;
                payload.text(&event.tag)?;
                payload.meta(&event.meta)?;
            }
            if let Some(event) = &entry.deleted {
                payload.work.take(1)?;
                add_bounded(&mut payload.usage.events, 1, limits.max_events, "events")?;
                payload.text(&event.source)?;
                payload.text(&event.reason)?;
                payload.meta(&event.meta)?;
            }
        }
        if self.pass_order.len() > limits.max_events {
            return Err("CV stage declarations limit exceeded".into());
        }
        for source in &self.pass_order {
            payload.work.take(1)?;
            payload.text(source)?;
        }
        Ok((limits, work))
    }
    /// Validate graph integrity independently from the mutation ledger.
    pub fn validate_graph(&self) -> Result<(), String> {
        self.validated_graph(&self.limits(), true)?;
        Ok(())
    }
    fn validated_graph(
        &self,
        limits: &GraphLimits,
        verify_usage: bool,
    ) -> Result<ValidatedGraph<'_>, String> {
        self.validated_graph_with_work(limits, verify_usage, Work::new(limits.max_work))
    }
    fn validated_graph_with_work(
        &self,
        limits: &GraphLimits,
        verify_usage: bool,
        mut work: Work,
    ) -> Result<ValidatedGraph<'_>, String> {
        if self.allocator_only_import {
            return Err("allocator-only CV import is not checked graph evidence".into());
        }
        limits.validate()?;
        if self.entries.len() > limits.max_nodes {
            return Err("CV nodes limit exceeded".into());
        }
        // Fallible evidence queries never inherit allocator-only compatibility
        // semantics. A generic compact reload can preserve a disabled-history
        // gap, but that gap cannot become complete lineage through try_*.
        if !self.enabled {
            return Err("CV graph recording is disabled".into());
        }
        if let Some(last) = self.compact_sequence {
            if Some(last) != u64::try_from(self.entries.len()).ok() {
                return Err("CV allocation coverage is incomplete".into());
            }
        }
        let mut payload = Payload {
            usage: Usage::default(),
            limits,
            work: &mut work,
        };
        let mut keys = Vec::new();
        // Bound legacy identity storage before sorting potentially long keys.
        for (key, entry) in &self.entries {
            payload.work.take(1)?;
            if key != &entry.id {
                return Err("CV entry key does not match identity".into());
            }
            if self.compact_sequence.is_none() {
                payload.text(key)?;
            }
            keys.push(key.as_str());
        }
        keys.sort_unstable();
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut indegrees = HashMap::new();
        let mut seen_sources = HashSet::new();
        for key in &keys {
            let entry = &self.entries[*key];
            payload.work.take(1)?;
            add_bounded(&mut payload.usage.nodes, 1, limits.max_nodes, "nodes")?;
            add_bounded(
                &mut payload.usage.edges,
                entry.parent_ids.len(),
                limits.max_edges,
                "parent edges",
            )?;
            payload.origin(entry.origin.as_ref())?;
            let sequence = self
                .compact_sequence
                .map(|_| compact_id_sequence(key))
                .transpose()?;
            if let Some(seq) = sequence {
                if seq > self.compact_sequence.unwrap() {
                    return Err("CV identity exceeds allocation state".into());
                }
            }
            indegrees.insert(*key, entry.parent_ids.len());
            for parent in &entry.parent_ids {
                payload.work.take(1)?;
                if !self.entries.contains_key(parent) {
                    return Err("CV graph has an unknown parent".into());
                }
                if let Some(seq) = sequence {
                    if compact_id_sequence(parent)? >= seq {
                        return Err("CV parent violates compact allocation chronology".into());
                    }
                }
                if self.compact_sequence.is_none() {
                    payload.text(parent)?;
                }
                children.entry(parent.as_str()).or_default().push(*key);
            }
            for event in &entry.contributions {
                payload.work.take(1)?;
                add_bounded(&mut payload.usage.events, 1, limits.max_events, "events")?;
                payload.text(&event.source)?;
                payload.text(&event.tag)?;
                payload.meta(&event.meta)?;
                seen_sources.insert(event.source.as_str());
            }
            if let Some(event) = &entry.deleted {
                payload.work.take(1)?;
                add_bounded(&mut payload.usage.events, 1, limits.max_events, "events")?;
                payload.text(&event.source)?;
                payload.text(&event.reason)?;
                payload.meta(&event.meta)?;
                seen_sources.insert(event.source.as_str());
            }
        }
        let mut declared_sources = HashSet::new();
        for source in &self.pass_order {
            payload.work.take(1)?;
            payload.text(source)?;
            if !declared_sources.insert(source.as_str()) {
                return Err("duplicate CV stage declaration".into());
            }
        }
        let usage = payload.usage;
        if declared_sources != seen_sources {
            return Err("CV stage declarations do not match recorded events".into());
        }
        if let Some(state) = &self.checked {
            if verify_usage && state.usage != usage {
                return Err("checked CV retained usage is inconsistent".into());
            }
        }
        // Kahn's algorithm removes every parent edge before admitting a child.
        // Repeated edges decrement repeated indegree contributions, while a
        // BTreeSet gives deterministic ready selection independent of HashMaps.
        let mut ready = BTreeSet::new();
        for key in &keys {
            work.take(1)?;
            if indegrees[key] == 0 {
                ready.insert(*key);
            }
        }
        let mut topology = Vec::new();
        while let Some(id) = ready.pop_first() {
            work.take(1)?;
            topology.push(id);
            if let Some(next) = children.get(id) {
                for child in next {
                    work.take(1)?;
                    let degree = indegrees.get_mut(child).expect("validated child");
                    *degree = degree
                        .checked_sub(1)
                        .ok_or("CV indegree accounting underflow")?;
                    if *degree == 0 {
                        ready.insert(*child);
                    }
                }
            }
        }
        if topology.len() != keys.len() {
            return Err("CV graph contains a cycle".into());
        }
        Ok(ValidatedGraph {
            topology,
            children,
            work,
            usage,
        })
    }
    /// Nearest-first ancestry, retaining parent-list order within each BFS level.
    pub fn try_ancestors(&self, id: &str) -> Result<Vec<String>, String> {
        let mut graph = self.validated_graph(&self.limits(), true)?;
        if !self.entries.contains_key(id) {
            return Err("unknown CV query identity".into());
        }
        let ids = self.walk_parents(id, &mut graph.work)?;
        Ok(ids.into_iter().map(str::to_string).collect())
    }
    fn walk_parents<'a>(&'a self, id: &str, work: &mut Work) -> Result<Vec<&'a str>, String> {
        let mut visited = HashSet::new();
        visited.insert(id);
        let mut queue = VecDeque::new();
        let root = self.entries.get(id).ok_or("unknown CV query identity")?;
        for parent in &root.parent_ids {
            work.take(1)?;
            if visited.insert(parent.as_str()) {
                queue.push_back(parent.as_str());
            }
        }
        let mut result = Vec::new();
        while let Some(current) = queue.pop_front() {
            work.take(1)?;
            result.push(current);
            for parent in &self.entries[current].parent_ids {
                work.take(1)?;
                if visited.insert(parent.as_str()) {
                    queue.push_back(parent.as_str());
                }
            }
        }
        Ok(result)
    }
    /// Descendants use one reverse index, with children sorted by identity.
    pub fn try_descendants(&self, id: &str) -> Result<Vec<String>, String> {
        let mut graph = self.validated_graph(&self.limits(), true)?;
        if !self.entries.contains_key(id) {
            return Err("unknown CV query identity".into());
        }
        let mut visited = HashSet::new();
        visited.insert(id);
        let mut queue = VecDeque::from([id]);
        let mut result = Vec::new();
        while let Some(current) = queue.pop_front() {
            graph.work.take(1)?;
            if let Some(children) = graph.children.get(current) {
                for child in children {
                    graph.work.take(1)?;
                    if visited.insert(child) {
                        queue.push_back(child);
                        result.push((*child).to_string());
                    }
                }
            }
        }
        Ok(result)
    }
    /// Borrowed lineage includes each reachable ancestor once, every parent
    /// before its child. This is topology, not actual transformation chronology.
    pub fn try_lineage(&self, id: &str) -> Result<Vec<&CVEntry>, String> {
        let mut graph = self.validated_graph(&self.limits(), true)?;
        if !self.entries.contains_key(id) {
            return Err("unknown CV query identity".into());
        }
        let mut selected: HashSet<_> = self
            .walk_parents(id, &mut graph.work)?
            .into_iter()
            .collect();
        selected.insert(id);
        let mut result = Vec::new();
        for key in graph.topology {
            graph.work.take(1)?;
            if selected.contains(key) {
                result.push(&self.entries[key]);
            }
        }
        Ok(result)
    }

    /// Import full compact evidence after bounded parsing and independent
    /// validation. Generic from_json_string is allocator-state import and can
    /// retain incomplete recording; compiler query boundaries use this API.
    pub fn from_checked_json(text: &str, limits: GraphLimits) -> Result<Self, String> {
        let (value, mut work) = super::bounded_json::parse(text, &limits)?;
        // Reserve the parser's structural visits again before serde consumes the
        // bounded representation. No conversion phase receives a fresh budget.
        work.reserve_conversion()?;
        let snapshot: super::LogSnapshot =
            serde_json::from_value(value).map_err(|e| format!("checked CV snapshot: {e}"))?;
        let identity = snapshot
            .identity
            .ok_or("checked CV import requires compact identities")?;
        let last = super::fixed_hex_sequence(&identity.last_sequence)?;
        work.take(snapshot.pass_order.len())?;
        let mut log = Self {
            entries: snapshot.entries,
            pass_sources: snapshot.pass_order.iter().cloned().collect(),
            pass_order: snapshot.pass_order,
            enabled: snapshot.enabled,
            compact_sequence: Some(last),
            base_counters: HashMap::new(),
            child_counters: HashMap::new(),
            checked: Some(CheckedState {
                limits: limits.clone(),
                usage: Usage::default(),
            }),
            allocator_only_import: false,
        };
        let usage = log.validated_graph_with_work(&limits, false, work)?.usage;
        log.checked.as_mut().unwrap().usage = usage;
        Ok(log)
    }
}

struct ValidatedGraph<'a> {
    topology: Vec<&'a str>,
    children: HashMap<&'a str, Vec<&'a str>>,
    work: Work,
    usage: Usage,
}

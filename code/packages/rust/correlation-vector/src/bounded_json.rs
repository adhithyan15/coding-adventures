//! Parse checked snapshots without first discarding duplicate keys or accepting
//! unbounded node/edge/event arrays. Seeds charge a role's count before its body
//! is decoded. A metadata key named "entries" is still metadata, not a graph.
use super::checked::{add_bounded, string_bytes, Work};
use super::{compact_id_sequence, GraphLimits};
use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;

#[derive(Clone, Copy)]
enum Role {
    Root,
    Entries,
    Entry,
    Parents,
    Parent,
    Events,
    Event,
    Origin,
    Deletion,
    Identity,
    Stages,
    Text,
    MaybeText,
    Bool,
    Scheme,
    Sequence,
    Metadata,
}
impl Role {
    fn child(self, key: &str) -> Result<Self, String> {
        use Role::*;
        Ok(match (self, key) {
            (Root, "entries") => Entries,
            (Root, "pass_order") => Stages,
            (Root, "enabled") => Bool,
            (Root, "identity") => Identity,
            (Entry, "id") => Parent,
            (Entry, "parent_ids") => Parents,
            (Entry, "contributions") => Events,
            (Entry, "origin") => Origin,
            (Entry, "deleted") => Deletion,
            (Origin, "source" | "location")
            | (Event, "source" | "tag")
            | (Deletion, "source" | "reason") => Text,
            (Origin, "timestamp") => MaybeText,
            (Origin | Event | Deletion, "meta") => Metadata,
            (Identity, "scheme") => Scheme,
            (Identity, "last_sequence") => Sequence,
            (Entries, _) => Entry,
            (Metadata, _) => Metadata,
            _ => return Err("unknown checked CV snapshot field".into()),
        })
    }
    fn required(self) -> &'static [&'static str] {
        use Role::*;
        match self {
            Root => &["entries", "pass_order", "enabled", "identity"],
            Entry => &["id", "parent_ids", "origin", "contributions", "deleted"],
            Origin => &["source", "location", "timestamp", "meta"],
            Event => &["source", "tag", "meta"],
            Deletion => &["source", "reason", "meta"],
            Identity => &["scheme", "last_sequence"],
            _ => &[],
        }
    }
}

struct Budget<'a> {
    limits: &'a GraphLimits,
    work: Work,
    nodes: usize,
    edges: usize,
    events: usize,
    stages: usize,
    values: usize,
    bytes: usize,
}
impl Budget<'_> {
    fn payload(&mut self, n: usize) -> Result<(), String> {
        add_bounded(
            &mut self.bytes,
            n,
            self.limits.max_metadata_bytes,
            "metadata bytes",
        )
    }
    fn text(&mut self, s: &str) -> Result<(), String> {
        if s.len() > self.limits.max_metadata_bytes {
            return Err("CV metadata bytes limit exceeded".into());
        }
        self.payload(string_bytes(s)?)
    }
}

struct Seed<'a, 'b> {
    budget: &'a mut Budget<'b>,
    role: Role,
    depth: usize,
    meta_depth: usize,
}
impl<'de> DeserializeSeed<'de> for Seed<'_, '_> {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        self.budget.work.take(1).map_err(D::Error::custom)?;
        // Fixed graph wrappers add at most eight levels around bounded metadata.
        if self.depth > 72 {
            return Err(D::Error::custom("CV snapshot nesting limit exceeded"));
        }
        match self.role {
            Role::Entry => add_bounded(
                &mut self.budget.nodes,
                1,
                self.budget.limits.max_nodes,
                "nodes",
            )
            .map_err(D::Error::custom)?,
            Role::Parent => {}
            Role::Event | Role::Deletion => {} // Null tombstones are not events.
            Role::Metadata => {
                add_bounded(
                    &mut self.budget.values,
                    1,
                    self.budget.limits.max_metadata_values,
                    "metadata values",
                )
                .map_err(D::Error::custom)?;
                if self.meta_depth > self.budget.limits.max_metadata_depth {
                    return Err(D::Error::custom("CV metadata depth limit exceeded"));
                }
            }
            _ => {}
        }
        deserializer.deserialize_any(self)
    }
}
impl<'de> Visitor<'de> for Seed<'_, '_> {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a bounded checked CV snapshot value")
    }
    fn visit_unit<E: Error>(self) -> Result<Value, E> {
        if !matches!(
            self.role,
            Role::Metadata | Role::MaybeText | Role::Origin | Role::Deletion
        ) {
            return Err(E::custom("invalid null checked CV field"));
        }
        if matches!(self.role, Role::Metadata) {
            self.budget.payload(4).map_err(E::custom)?;
        }
        Ok(Value::Null)
    }
    fn visit_bool<E: Error>(self, v: bool) -> Result<Value, E> {
        if !matches!(self.role, Role::Metadata | Role::Bool) {
            return Err(E::custom("invalid boolean checked CV field"));
        }
        if matches!(self.role, Role::Metadata) {
            self.budget
                .payload(if v { 4 } else { 5 })
                .map_err(E::custom)?;
        }
        Ok(Value::Bool(v))
    }
    fn visit_i64<E: Error>(self, v: i64) -> Result<Value, E> {
        self.number(Number::from(v))
    }
    fn visit_u64<E: Error>(self, v: u64) -> Result<Value, E> {
        self.number(Number::from(v))
    }
    fn visit_f64<E: Error>(self, v: f64) -> Result<Value, E> {
        self.number(Number::from_f64(v).ok_or_else(|| E::custom("non-finite CV metadata number"))?)
    }
    fn visit_str<E: Error>(mut self, v: &str) -> Result<Value, E> {
        self.check_text(v)?;
        Ok(Value::String(v.to_string()))
    }
    fn visit_string<E: Error>(mut self, v: String) -> Result<Value, E> {
        // Reuse validation; the deserializer's scratch/string storage is bounded
        // by the already-checked input-byte limit.
        self.check_text::<E>(&v)?;
        Ok(Value::String(v))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let child = match self.role {
            Role::Parents => Role::Parent,
            Role::Events => Role::Event,
            Role::Stages => Role::Text,
            Role::Metadata => Role::Metadata,
            _ => return Err(A::Error::custom("invalid checked CV array field")),
        };
        if matches!(self.role, Role::Metadata) {
            self.budget.payload(2).map_err(A::Error::custom)?;
        }
        let mut values = Vec::new();
        loop {
            // Counts are charged by a seed only when an element actually exists.
            let next = seq.next_element_seed(ArraySeed {
                seed: Seed {
                    budget: self.budget,
                    role: child,
                    depth: self.depth + 1,
                    meta_depth: self.meta_depth + 1,
                },
                container: self.role,
            })?;
            let Some(value) = next else { break };
            if matches!(self.role, Role::Metadata) && !values.is_empty() {
                self.budget.payload(1).map_err(A::Error::custom)?;
            }
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        if !matches!(
            self.role,
            Role::Root
                | Role::Entries
                | Role::Entry
                | Role::Event
                | Role::Origin
                | Role::Deletion
                | Role::Identity
                | Role::Metadata
        ) {
            return Err(A::Error::custom("invalid checked CV object field"));
        }
        if matches!(self.role, Role::Deletion) {
            add_bounded(
                &mut self.budget.events,
                1,
                self.budget.limits.max_events,
                "events",
            )
            .map_err(A::Error::custom)?;
        }
        if matches!(self.role, Role::Metadata) {
            self.budget.payload(2).map_err(A::Error::custom)?;
        }
        let mut fields = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            self.budget.work.take(1).map_err(A::Error::custom)?;
            if fields.contains_key(&key) {
                return Err(A::Error::custom(if matches!(self.role, Role::Entries) {
                    "duplicate CV entry identity"
                } else {
                    "duplicate checked CV object key"
                }));
            }
            if matches!(self.role, Role::Root) && key == "view" {
                return Err(A::Error::custom(
                    "declared CV view is not a complete checked log",
                ));
            }
            if matches!(self.role, Role::Entries) {
                compact_id_sequence(&key).map_err(A::Error::custom)?;
            }
            if matches!(self.role, Role::Metadata) {
                self.budget.text(&key).map_err(A::Error::custom)?;
                self.budget
                    .payload(if fields.is_empty() { 1 } else { 2 })
                    .map_err(A::Error::custom)?;
            }
            let role = self.role.child(&key).map_err(A::Error::custom)?;
            let meta_depth = if matches!(role, Role::Metadata) {
                if matches!(self.role, Role::Metadata) {
                    self.meta_depth + 1
                } else {
                    1
                }
            } else {
                0
            };
            let value = map.next_value_seed(Seed {
                budget: self.budget,
                role,
                depth: self.depth + 1,
                meta_depth,
            })?;
            fields.insert(key, value);
        }
        for required in self.role.required() {
            if !fields.contains_key(*required) {
                return Err(A::Error::custom("missing checked CV snapshot field"));
            }
        }
        Ok(Value::Object(fields))
    }
}
impl Seed<'_, '_> {
    fn check_text<E: Error>(&mut self, v: &str) -> Result<(), E> {
        match self.role {
            Role::Scheme if v != "compact-v1" => {
                return Err(E::custom("unsupported checked CV identity scheme"))
            }
            Role::Scheme => {}
            Role::Sequence => {
                super::fixed_hex_sequence(v).map_err(E::custom)?;
            }
            Role::Parent => {
                compact_id_sequence(v).map_err(E::custom)?;
            }
            Role::Text | Role::MaybeText | Role::Metadata => {
                self.budget.text(v).map_err(E::custom)?
            }
            _ => return Err(E::custom("invalid text checked CV field")),
        }
        Ok(())
    }
    fn number<E: Error>(self, n: Number) -> Result<Value, E> {
        if !matches!(self.role, Role::Metadata) {
            return Err(E::custom("invalid numeric checked CV field"));
        }
        if matches!(self.role, Role::Metadata) {
            self.budget
                .payload(n.to_string().len())
                .map_err(E::custom)?;
        }
        Ok(Value::Number(n))
    }
}
struct ArraySeed<'a, 'b> {
    seed: Seed<'a, 'b>,
    container: Role,
}
impl<'de> DeserializeSeed<'de> for ArraySeed<'_, '_> {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        if matches!(self.container, Role::Parents) {
            add_bounded(
                &mut self.seed.budget.edges,
                1,
                self.seed.budget.limits.max_edges,
                "parent edges",
            )
            .map_err(D::Error::custom)?;
        }
        // An event array item must be an object; charging before its body also
        // prevents an oversized malformed item from bypassing the event cap.
        if matches!(self.container, Role::Events) {
            add_bounded(
                &mut self.seed.budget.events,
                1,
                self.seed.budget.limits.max_events,
                "events",
            )
            .map_err(D::Error::custom)?;
        }
        if matches!(self.container, Role::Stages) {
            add_bounded(
                &mut self.seed.budget.stages,
                1,
                self.seed.budget.limits.max_events,
                "stage declarations",
            )
            .map_err(D::Error::custom)?;
        }
        self.seed.deserialize(d)
    }
}

pub(super) fn parse(text: &str, limits: &GraphLimits) -> Result<(Value, Work), String> {
    if text.len() > limits.max_input_bytes {
        return Err("CV input bytes limit exceeded".into());
    }
    limits.validate()?;
    let mut budget = Budget {
        limits,
        work: Work::new(limits.max_work),
        nodes: 0,
        edges: 0,
        events: 0,
        stages: 0,
        values: 0,
        bytes: 0,
    };
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let value = Seed {
        budget: &mut budget,
        role: Role::Root,
        depth: 1,
        meta_depth: 0,
    }
    .deserialize(&mut deserializer)
    .map_err(|e| format!("checked CV import: {e}"))?;
    deserializer
        .end()
        .map_err(|e| format!("checked CV import: {e}"))?;
    Ok((value, budget.work))
}

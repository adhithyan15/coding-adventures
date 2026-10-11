//! A rejected owned argument still needs safe destruction.
//!
//! serde_json::Value normally drops recursively. Validation can stop at depth
//! 65, but the caller may have transferred a tree 65,536 levels deep. These
//! guards retain ownership until insertion and dispose of rejected values with
//! heap iterator frames. A wide array stays in its original allocation; we do
//! not copy every sibling into an additional pending stack.
use super::Origin;
use serde_json::Value;
use std::collections::HashMap;

enum Children {
    Array(std::vec::IntoIter<Value>),
    Object(serde_json::map::IntoIter),
}
impl Children {
    fn next(&mut self) -> Option<Value> {
        match self {
            Self::Array(items) => items.next(),
            Self::Object(fields) => fields.next().map(|(_, value)| value),
        }
    }
}

fn value(root: Value) {
    let mut frames = Vec::new();
    let mut current = Some(root);
    loop {
        if let Some(item) = current.take() {
            match item {
                Value::Array(items) => frames.push(Children::Array(items.into_iter())),
                Value::Object(fields) => frames.push(Children::Object(fields.into_iter())),
                _ => {}
            }
        }
        while let Some(frame) = frames.last_mut() {
            if let Some(item) = frame.next() {
                current = Some(item);
                break;
            }
            // The iterator is exhausted; dropping it cannot recurse through
            // any remaining Value. Only one frame per nesting level is stored.
            frames.pop();
        }
        if current.is_none() {
            break;
        }
    }
}

pub(super) fn metadata(meta: HashMap<String, Value>) {
    for item in meta.into_values() {
        value(item);
    }
}

pub(super) struct OwnedOrigin(Option<Origin>);
impl OwnedOrigin {
    pub fn new(origin: Option<Origin>) -> Self {
        Self(origin)
    }
    pub fn as_ref(&self) -> Option<&Origin> {
        self.0.as_ref()
    }
    pub fn take(&mut self) -> Option<Origin> {
        self.0.take()
    }
}
impl Drop for OwnedOrigin {
    fn drop(&mut self) {
        if let Some(origin) = self.0.take() {
            metadata(origin.meta);
        }
    }
}

pub(super) struct OwnedMetadata(Option<HashMap<String, Value>>);
impl OwnedMetadata {
    pub fn new(meta: HashMap<String, Value>) -> Self {
        Self(Some(meta))
    }
    pub fn as_ref(&self) -> &HashMap<String, Value> {
        self.0.as_ref().expect("owned metadata")
    }
    pub fn take(&mut self) -> HashMap<String, Value> {
        self.0.take().expect("owned metadata")
    }
}
impl Drop for OwnedMetadata {
    fn drop(&mut self) {
        if let Some(meta) = self.0.take() {
            metadata(meta);
        }
    }
}

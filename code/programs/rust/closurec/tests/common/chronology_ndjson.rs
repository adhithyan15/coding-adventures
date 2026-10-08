//! Test adapter for the canonical writer's framing; this is not a production
//! NDJSON loader. Keep raw entry/event bodies so duplicate/escaped object keys
//! remain visible to the final bounded checked JSON importer. No Value tree
//! normalization can silently discard a field before that trust boundary.
use coding_adventures_correlation_vector::{GraphLimits, JournalEntity};
use serde::de::{Error, IgnoredAny, MapAccess, Visitor};
use serde::Deserialize;
use std::fmt;

struct KeyVisitor;
impl Visitor<'_> for KeyVisitor {
    type Value = bool;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an entry key")
    }
    fn visit_str<E: Error>(self, key: &str) -> Result<bool, E> {
        Ok(key == "id")
    }
}
struct Key(bool);
impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_str(KeyVisitor).map(Self)
    }
}
struct EntryId(JournalEntity);
impl<'de> Deserialize<'de> for EntryId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Entry;
        impl<'de> Visitor<'de> for Entry {
            type Value = EntryId;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a compact entry object")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut id = None;
                while let Some(Key(is_id)) = map.next_key::<Key>()? {
                    if is_id {
                        if id.is_some() {
                            return Err(A::Error::custom("duplicate entry id"));
                        }
                        id = Some(map.next_value::<JournalEntity>()?);
                    } else {
                        map.next_value::<IgnoredAny>()?;
                    }
                }
                Ok(EntryId(
                    id.ok_or_else(|| A::Error::custom("missing entry id"))?,
                ))
            }
        }
        d.deserialize_map(Entry)
    }
}
fn append(output: &mut String, text: &str, cap: usize) -> Result<(), String> {
    if output
        .len()
        .checked_add(text.len())
        .is_none_or(|size| size > cap)
    {
        return Err("NDJSON reconstructed input bytes limit exceeded".into());
    }
    output.push_str(text);
    Ok(())
}

pub fn reconstruct(text: &str, limits: &GraphLimits) -> Result<String, String> {
    if text.len() > limits.max_input_bytes {
        return Err("NDJSON input bytes limit exceeded".into());
    }
    let mut entries = Vec::new();
    let mut events = Vec::new();
    let mut footer = None;
    let mut event_phase = false;
    let mut work = 0usize;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        work = work.checked_add(1).ok_or("NDJSON work overflow")?;
        if work > limits.max_work {
            return Err("NDJSON operation work limit exceeded".into());
        }
        if footer.is_some() {
            return Err("NDJSON record after footer".into());
        }
        let line = line.trim();
        if let Some(root) = line
            .strip_prefix("{\"_meta\":")
            .and_then(|v| v.strip_suffix('}'))
        {
            footer = Some(root);
        } else if let Some(record) = line
            .strip_prefix("{\"_event\":")
            .and_then(|v| v.strip_suffix('}'))
        {
            event_phase = true;
            if events.len() >= limits.max_events {
                return Err("NDJSON events limit exceeded".into());
            }
            events.push(record);
        } else {
            if event_phase {
                return Err("NDJSON entry after event frame".into());
            }
            if entries.len() >= limits.max_nodes {
                return Err("NDJSON nodes limit exceeded".into());
            }
            let EntryId(id) = serde_json::from_str::<EntryId>(line)
                .map_err(|e| format!("NDJSON entry framing: {e}"))?;
            entries.push((format!("cv1.{:016x}", id.sequence()), line));
        }
    }
    let footer = footer.ok_or("NDJSON missing footer")?;
    let root = footer
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .ok_or("NDJSON invalid root footer")?;
    // Canonical footer fields have fixed literal keys. Insert without decoding
    // or replacing its existing fields: a duplicate events key remains a
    // duplicate and the final checked parser must reject it.
    let marker = "\"journal\":{";
    let journal = root
        .find(marker)
        .ok_or("NDJSON canonical journal state missing")?
        + marker.len();
    let mut output = String::new();
    let cap = limits.max_input_bytes;
    append(&mut output, "{\"entries\":{", cap)?;
    for (index, (id, entry)) in entries.iter().enumerate() {
        if index != 0 {
            append(&mut output, ",", cap)?;
        }
        append(&mut output, "\"", cap)?;
        append(&mut output, id, cap)?;
        append(&mut output, "\":", cap)?;
        append(&mut output, entry, cap)?;
    }
    append(&mut output, "},", cap)?;
    append(&mut output, &root[..journal], cap)?;
    append(&mut output, "\"events\":[", cap)?;
    for (index, event) in events.iter().enumerate() {
        if index != 0 {
            append(&mut output, ",", cap)?;
        }
        append(&mut output, event, cap)?;
    }
    append(&mut output, "],", cap)?;
    append(&mut output, &root[journal..], cap)?;
    append(&mut output, "}", cap)?;
    Ok(output)
}

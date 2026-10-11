//! # Freshness: the sealed per-namespace index (VLT01 F1, F3)
//!
//! The AEAD on a sealed record answers "did a KEK holder write this?". It
//! cannot answer "is this the *latest* thing a KEK holder wrote here?". An
//! old ciphertext file is still a perfectly valid ciphertext. Restore one from
//! last month's backup and it decrypts, and the vault would happily hand back
//! last month's secret and last month's policy.
//!
//! The freshness index is the vault's memory of what it last wrote. It holds
//! one entry per key, and it is itself sealed, so only a KEK holder can change
//! it:
//!
//! ```text
//!   key          generation   state       tag
//!   ───────────  ──────────   ─────────   ──────────────
//!   weather-key       7       live        9f3c…  the newest record is
//!                                                generation 7, with this tag
//!   old-token         3       tombstone   —      deleted after generation 3
//!   imported          1       legacy      —      mid-migration from format v1
//! ```
//!
//! The **tag** is the newest record's AEAD tag. A generation number alone is
//! not enough to name one record: if a `put` ever reuses a generation (after
//! a crash, or after someone puts back an old copy of the index), two
//! different authentic records share that number, and putting back the older
//! one would pass. With the tag stored, "generation 7" means exactly one
//! ciphertext.
//!
//! Every record written in format v2 carries its generation inside its AEAD
//! associated data. A record file is accepted only if its generation is
//! consistent with the index. See [`accepts`] for the exact table.
//!
//! This module is pure: encoding, decoding and the acceptance rule. Reading
//! and writing the sealed index is the store's job, in `lib.rs`.

use std::collections::BTreeMap;

/// The leading bytes of every encoded index.
const MAGIC: &[u8; 6] = b"VFRESH";

/// The index encoding version (F1).
const VERSION: u8 = 1;

/// Bytes in a record's AEAD tag, which a live entry pins (F1).
pub(crate) const TAG_BYTES: usize = 16;

/// The tag carried by entries that pin no record (tombstone, legacy).
pub(crate) const NO_TAG: [u8; TAG_BYTES] = [0; TAG_BYTES];

/// The most entries one namespace's index may hold (F1).
pub(crate) const MAX_ENTRIES: usize = 65_536;

/// What an index entry says about its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryState {
    /// The newest record for this key has this generation.
    Live,
    /// The key was deleted. Records at or below this generation are refused.
    Tombstone,
    /// Mid-migration (F6): a v1 record or its v2 generation-1 replacement.
    Legacy,
}

impl EntryState {
    fn to_byte(self) -> u8 {
        match self {
            Self::Live => 1,
            Self::Tombstone => 2,
            Self::Legacy => 3,
        }
    }

    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Live),
            2 => Some(Self::Tombstone),
            3 => Some(Self::Legacy),
            _ => None,
        }
    }
}

/// One key's entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) generation: u64,
    pub(crate) state: EntryState,
    /// For `Live`, the AEAD tag of the record at `generation`. [`NO_TAG`]
    /// otherwise.
    pub(crate) tag: [u8; TAG_BYTES],
}

/// One namespace's index, decoded.
///
/// A `BTreeMap` because the encoding requires strictly ascending keys, and
/// a map that is always sorted cannot produce anything else.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FreshnessIndex {
    /// Increases by one on every write. Nothing reads it yet; it is the
    /// value an external anchor would pin (F10, P1.20b).
    pub(crate) epoch: u64,
    pub(crate) entries: BTreeMap<String, Entry>,
}

impl FreshnessIndex {
    /// Whether migration (F6) is unfinished: any entry still `Legacy`.
    pub(crate) fn has_legacy(&self) -> bool {
        self.entries
            .values()
            .any(|entry| entry.state == EntryState::Legacy)
    }

    /// The canonical encoding (F1).
    ///
    /// ```text
    /// "VFRESH" | version u8 | epoch u64 BE | count u32 BE
    /// count × ( key_len u16 BE | key | generation u64 BE | state u8 | tag 16 )
    /// ```
    ///
    /// Returns `None` when the index is over [`MAX_ENTRIES`] or a key is
    /// longer than a `u16` can count. The caller refuses the write rather
    /// than truncating, because a truncated index would forget keys and so
    /// re-admit their old records.
    pub(crate) fn encode(&self) -> Option<Vec<u8>> {
        if self.entries.len() > MAX_ENTRIES {
            return None;
        }
        let mut out = Vec::with_capacity(19 + self.entries.len() * 48);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(&self.epoch.to_be_bytes());
        out.extend_from_slice(&u32::try_from(self.entries.len()).ok()?.to_be_bytes());
        for (key, entry) in &self.entries {
            out.extend_from_slice(&u16::try_from(key.len()).ok()?.to_be_bytes());
            out.extend_from_slice(key.as_bytes());
            out.extend_from_slice(&entry.generation.to_be_bytes());
            out.push(entry.state.to_byte());
            out.extend_from_slice(&entry.tag);
        }
        Some(out)
    }

    /// Decode, totally and strictly. Anything other than the canonical
    /// encoding is `None`: a wrong magic or version, truncation, trailing
    /// bytes, keys out of order or duplicated, non-UTF-8 keys, an unknown
    /// state, or more than [`MAX_ENTRIES`].
    ///
    /// The bytes have already passed the AEAD, so a failure here means a
    /// KEK holder wrote something this code does not understand. Refusing is
    /// still right: reading an index wrongly is how a rollback gets admitted.
    pub(crate) fn decode(bytes: &[u8]) -> Option<Self> {
        let mut reader = Reader(bytes);
        if reader.take(MAGIC.len())? != MAGIC || reader.u8()? != VERSION {
            return None;
        }
        let epoch = reader.u64()?;
        let count = usize::try_from(reader.u32()?).ok()?;
        if count > MAX_ENTRIES {
            return None;
        }
        let mut entries = BTreeMap::new();
        let mut previous: Option<&[u8]> = None;
        for _ in 0..count {
            let length = usize::from(reader.u16()?);
            let key = reader.take(length)?;
            if previous.is_some_and(|previous| previous >= key) {
                return None;
            }
            previous = Some(key);
            let generation = reader.u64()?;
            let state = EntryState::from_byte(reader.u8()?)?;
            let tag: [u8; TAG_BYTES] = reader.take(TAG_BYTES)?.try_into().ok()?;
            entries.insert(
                std::str::from_utf8(key).ok()?.to_string(),
                Entry {
                    generation,
                    state,
                    tag,
                },
            );
        }
        if !reader.0.is_empty() {
            return None;
        }
        Some(Self { epoch, entries })
    }
}

/// A cursor over a byte slice, where every read is bounds-checked.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        if self.0.len() < length {
            return None;
        }
        let (head, tail) = self.0.split_at(length);
        self.0 = tail;
        Some(head)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes(self.take(2)?.try_into().ok()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_be_bytes(self.take(8)?.try_into().ok()?))
    }
}

/// F3: whether a record file may be returned.
///
/// - `index`: whether the namespace has an index at all.
/// - `entry`: the key's entry, if the index has one.
/// - `record`: the record's generation and AEAD tag, where `None` means a v1
///   record.
///
/// | Index   | Entry              | v1     | v2, generation `g`, tag `u` |
/// |---------|--------------------|--------|-----------------------------|
/// | none    | —                  | accept | refuse                      |
/// | present | live `n`, tag `t`  | refuse | `g > n`, or `g == n && u == t` |
/// | present | tombstone `n`      | refuse | `g > n`                     |
/// | present | legacy             | accept | `g >= 1`                    |
/// | present | absent             | refuse | accept                      |
///
/// The rows that accept a v2 record *ahead of* the index (`g > n`, or no
/// entry) are not a loophole. Only a KEK holder can produce a v2 record, so a
/// record ahead of its index is a `put` whose index update did not land:
/// a crash, or a lost race. Refusing it would turn every such crash into
/// data loss. The next write absorbs it into the index (F4).
pub(crate) fn accepts(
    index: bool,
    entry: Option<Entry>,
    record: Option<(u64, [u8; TAG_BYTES])>,
) -> bool {
    if !index {
        // A v2 record means an index once existed. Its absence now means
        // someone deleted it, and with it every rollback check.
        return record.is_none();
    }
    match (entry, record) {
        (None, record) => record.is_some(),
        (Some(entry), None) => entry.state == EntryState::Legacy,
        (Some(entry), Some((generation, tag))) => match entry.state {
            EntryState::Live => {
                generation > entry.generation
                    || (generation == entry.generation && tag == entry.tag)
            }
            EntryState::Tombstone => generation > entry.generation,
            EntryState::Legacy => generation >= 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: [u8; TAG_BYTES] = [7; TAG_BYTES];
    const OTHER: [u8; TAG_BYTES] = [8; TAG_BYTES];

    fn live(generation: u64) -> Option<Entry> {
        Some(Entry {
            generation,
            state: EntryState::Live,
            tag: T,
        })
    }

    fn tombstone(generation: u64) -> Option<Entry> {
        Some(Entry {
            generation,
            state: EntryState::Tombstone,
            tag: NO_TAG,
        })
    }

    fn legacy() -> Option<Entry> {
        Some(Entry {
            generation: 1,
            state: EntryState::Legacy,
            tag: NO_TAG,
        })
    }

    fn sample() -> FreshnessIndex {
        let mut index = FreshnessIndex {
            epoch: 9,
            entries: BTreeMap::new(),
        };
        index.entries.insert("b".into(), live(4).unwrap());
        index.entries.insert("a/z".into(), tombstone(2).unwrap());
        index.entries.insert("c".into(), legacy().unwrap());
        index
    }

    #[test]
    fn the_acceptance_table_is_exactly_f3() {
        let v2 = |generation| Some((generation, T));
        // No index: only pre-migration records.
        assert!(accepts(false, None, None));
        assert!(!accepts(false, None, v2(1)));
        // Live n: the exact record at n, or anything ahead. Never older,
        // never v1, never a different record reusing n.
        assert!(accepts(true, live(5), v2(5)));
        assert!(!accepts(true, live(5), Some((5, OTHER))));
        assert!(accepts(true, live(5), v2(6)));
        assert!(accepts(true, live(5), Some((6, OTHER))));
        assert!(!accepts(true, live(5), v2(4)));
        assert!(!accepts(true, live(5), None));
        // Tombstone n: only a put after the delete.
        assert!(accepts(true, tombstone(5), v2(6)));
        assert!(!accepts(true, tombstone(5), v2(5)));
        assert!(!accepts(true, tombstone(5), None));
        // Legacy: either side of the migration.
        assert!(accepts(true, legacy(), None));
        assert!(accepts(true, legacy(), v2(1)));
        // Absent: a v2 put whose index update did not land, never a v1.
        assert!(accepts(true, None, v2(1)));
        assert!(!accepts(true, None, None));
    }

    #[test]
    fn the_encoding_round_trips_and_is_canonical() {
        let index = sample();
        let bytes = index.encode().unwrap();
        assert_eq!(FreshnessIndex::decode(&bytes), Some(index.clone()));
        // Sorted by key bytes: "a/z" < "b" < "c".
        let a = bytes.windows(3).position(|w| w == b"a/z").unwrap();
        let b = bytes.iter().rposition(|&byte| byte == b'b').unwrap();
        assert!(a < b);
        assert!(FreshnessIndex::default().encode().is_some());
        assert!(index.has_legacy());
        assert!(!FreshnessIndex::default().has_legacy());
    }

    #[test]
    fn every_malformed_encoding_is_refused() {
        let good = sample().encode().unwrap();
        // Truncation at every length, and one trailing byte.
        for cut in 0..good.len() {
            assert_eq!(FreshnessIndex::decode(&good[..cut]), None, "cut {cut}");
        }
        let mut trailing = good.clone();
        trailing.push(0);
        assert_eq!(FreshnessIndex::decode(&trailing), None);
        // Magic and version.
        let mut magic = good.clone();
        magic[0] = b'X';
        assert_eq!(FreshnessIndex::decode(&magic), None);
        let mut version = good.clone();
        version[6] = 2;
        assert_eq!(FreshnessIndex::decode(&version), None);
        // An unknown state byte: the last entry's state sits just before its
        // 16-byte tag.
        let mut state = good.clone();
        let at = state.len() - TAG_BYTES - 1;
        state[at] = 9;
        assert_eq!(FreshnessIndex::decode(&state), None);
    }

    fn encode_raw(entries: &[(&[u8], u64, u8)], count: u32) -> Vec<u8> {
        // Every raw entry carries a zero tag.
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.extend_from_slice(&0u64.to_be_bytes());
        out.extend_from_slice(&count.to_be_bytes());
        for (key, generation, state) in entries {
            out.extend_from_slice(&(key.len() as u16).to_be_bytes());
            out.extend_from_slice(key);
            out.extend_from_slice(&generation.to_be_bytes());
            out.push(*state);
            out.extend_from_slice(&NO_TAG);
        }
        out
    }

    #[test]
    fn order_duplicates_utf8_and_the_count_bound_are_enforced() {
        let ordered = encode_raw(&[(b"a", 1, 1), (b"b", 1, 1)], 2);
        assert!(FreshnessIndex::decode(&ordered).is_some());
        let reversed = encode_raw(&[(b"b", 1, 1), (b"a", 1, 1)], 2);
        assert_eq!(FreshnessIndex::decode(&reversed), None);
        let duplicate = encode_raw(&[(b"a", 1, 1), (b"a", 2, 1)], 2);
        assert_eq!(FreshnessIndex::decode(&duplicate), None);
        let not_utf8 = encode_raw(&[(&[0xff], 1, 1)], 1);
        assert_eq!(FreshnessIndex::decode(&not_utf8), None);
        let too_many = encode_raw(&[], MAX_ENTRIES as u32 + 1);
        assert_eq!(FreshnessIndex::decode(&too_many), None);

        let mut full = FreshnessIndex::default();
        for i in 0..=MAX_ENTRIES {
            full.entries.insert(format!("k{i}"), live(1).unwrap());
        }
        assert_eq!(full.encode(), None);
    }
}

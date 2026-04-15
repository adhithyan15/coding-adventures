# DT26 — LSM Tree (Log-Structured Merge-Tree)

## Overview

An **LSM tree** (Log-Structured Merge-Tree) is a data structure designed from
the ground up for one goal: **make writes as fast as possible**.

The central insight: sequential writes to disk are 10–100× faster than random
writes. A spinning hard drive's sequential write throughput is limited by the
disk's rotational speed (~100–200 MB/s), but random writes require a physical
seek between each operation — dropping throughput to ~1–2 MB/s. On SSDs, the
gap is smaller but still significant due to write amplification and flash page
management.

LSM trees exploit sequential writes by **never modifying data in-place**. Every
write appends to a log; no random seeks. Periodically, the system compacts these
logs into sorted files. Read performance is traded off to achieve this: instead
of one B+ tree lookup, a read may need to check several sorted files.

```
B+ Tree write path:                   LSM Tree write path:
  1. Find leaf via tree traversal        1. Append to WAL (sequential write)
  2. Modify leaf in place (random I/O)   2. Insert into in-memory table (RAM)
  3. Maybe split and rebalance           3. Done (no disk I/O for the write!)

  Write amplification: HIGH              Write amplification: LOW
  Write latency: ~10ms (disk seek)       Write latency: ~1μs (RAM + WAL)
  Read amplification: LOW (1 tree walk)  Read amplification: HIGHER (multi-level)
```

Real-world uses: **RocksDB** (Meta/Facebook), **LevelDB** (Google), **Apache
Cassandra**, **Apache HBase**, **ScyllaDB**, **InfluxDB**, **TiKV** (TiDB's
storage engine), **BadgerDB** (Go). The pattern also appears in time-series
databases and write-heavy workloads like logs, metrics, and event streams.

## Layer Position

```
DT11: b-tree             ← sibling (B-tree favors reads; LSM favors writes)
DT12: b-plus-tree        ← sibling (standard database index; different tradeoffs)
DT20: skip-list          ← used as the in-memory table (memtable) structure
DT22: bloom-filter       ← used per SSTable to skip disk reads for missing keys
DT17: hash-functions     ← used by the bloom filter and block index

DT26: lsm-tree           ← [YOU ARE HERE]
  ├── used by: RocksDB, LevelDB, Cassandra, HBase, BadgerDB
  ├── used by: time-series databases (InfluxDB, Prometheus TSDB)
  └── used by: embedded write-heavy stores

F00:  block-ram          ← storage medium for SSTables
CMP00: lz77              ← optional block compression within SSTables
```

**Depends on:** DT20 (skip-list: the memtable), DT22 (bloom filter: per-SSTable
skip filter), F00 (block-ram: disk page abstraction).
**Contrasts with:** DT12 (B+ tree: in-place updates, lower read amplification).
**Uses from:** DT17 (hash functions for bloom filter and block index checksums).

## Concepts

### The Three Big Ideas

An LSM tree is built on three ideas that work together:

**Idea 1 — Write to RAM first (the Memtable):**
All writes go directly into an in-memory sorted data structure called the
*memtable*. No disk I/O happens for the write itself. The memtable is typically
a skip list (DT20) or a red-black tree (DT09) — any sorted container works.

**Idea 2 — Durability via append-only WAL:**
Before touching the memtable, every write is appended to a *Write-Ahead Log*
(WAL) — a plain sequential file on disk. If the process crashes, the WAL is
replayed to reconstruct the memtable. The WAL is never read during normal
operation; it's only for crash recovery.

**Idea 3 — Immutable sorted files (SSTables):**
When the memtable grows too large (typically 64 MB), it is *flushed* to disk
as an immutable *SSTable* (Sorted String Table). SSTables are never modified
after creation — they are read-only files containing sorted key-value pairs.
When too many SSTables accumulate, they are *compacted*: merged and rewritten
into fewer, larger SSTables.

```
Write path:
  write(k, v)
      │
      ├──► WAL (append-only log on disk)  — for crash recovery
      │
      └──► Memtable (sorted in RAM)       — for fast reads

  When memtable is full:
      Memtable ──► flush ──► SSTable on disk (Level 0)

  When Level 0 has too many SSTables:
      Compact Level 0 + Level 1 ──► new Level 1 SSTables

Read path (for key k):
  1. Check memtable                        — O(log n) in RAM
  2. Check each Level 0 SSTable (newest first) — bloom filter first
  3. Check Level 1 (at most 1 SSTable)     — binary search on index
  4. Check Level 2 ...                     — binary search on index
  ...until found or all levels exhausted
```

### The Memtable

The memtable is the live, mutable, in-memory store. It holds the most recent
version of every key that has been written but not yet flushed to disk.

```
Memtable (Skip List, DT20):

  Key   │ Value
  ──────┼──────────────
  alice │ "Wonderland"
  bob   │ "Builder"
  carol │ [TOMBSTONE]   ← deletion is a special marker, not an actual delete
  dave  │ "Bowman"

  All keys are sorted.
  When size exceeds threshold (e.g., 64 MB), flush to SSTable.
```

**Tombstones:** Deleting a key does not remove it from the memtable — it inserts
a special marker called a *tombstone*. This is necessary because older versions
of the key might still exist in SSTables on disk. The tombstone propagates
through the levels during compaction and eventually eliminates all older copies.

```
Timeline of deleting "alice":

  T=1: write("alice", "Wonderland")   → memtable: {alice: "Wonderland"}
  T=2: flush to SSTable 1             → disk: [alice: "Wonderland"]
  T=3: delete("alice")                → memtable: {alice: TOMBSTONE}
  T=4: flush to SSTable 2             → disk: [alice: TOMBSTONE]

  Read("alice") at T=4:
    Check memtable → TOMBSTONE → return "not found"  ✓

  T=5: compaction merges SSTable 1 + SSTable 2:
    alice: "Wonderland" vs alice: TOMBSTONE
    → TOMBSTONE wins (newer) → alice is dropped from result
    → Compacted SSTable contains NO entry for alice  ✓
```

### SSTables (Sorted String Tables)

An SSTable is an immutable on-disk file containing sorted key-value pairs. It
has three sections:

```
┌──────────────────────────────────────────┐
│ SSTable File Layout                      │
├──────────────────────────────────────────┤
│  1. Data Blocks                          │
│     ┌────────────────────────────────┐   │
│     │ Block 0: sorted (k,v) pairs    │   │
│     │   alice: "Wonderland"          │   │
│     │   bob: "Builder"               │   │
│     │   carol: [TOMBSTONE]           │   │
│     └────────────────────────────────┘   │
│     ┌────────────────────────────────┐   │
│     │ Block 1: sorted (k,v) pairs    │   │
│     │   dave: "Bowman"               │   │
│     │   eve: "Online"                │   │
│     └────────────────────────────────┘   │
│                                          │
│  2. Block Index                          │
│     ┌────────────────────────────────┐   │
│     │ Block 0: first_key=alice,      │   │
│     │          offset=0, size=4096   │   │
│     │ Block 1: first_key=dave,       │   │
│     │          offset=4096, size=4096│   │
│     └────────────────────────────────┘   │
│                                          │
│  3. Bloom Filter                         │
│     ┌────────────────────────────────┐   │
│     │ Bloom filter bits (DT22)       │   │
│     │ Contains all keys in file.     │   │
│     │ Tells us quickly: "key X is    │   │
│     │ DEFINITELY NOT in this file"   │   │
│     └────────────────────────────────┘   │
│                                          │
│  4. Footer                               │
│     ┌────────────────────────────────┐   │
│     │ offset of block index          │   │
│     │ offset of bloom filter         │   │
│     │ magic number (integrity check) │   │
│     └────────────────────────────────┘   │
└──────────────────────────────────────────┘
```

**Why blocks?** Compressing and indexing 4 KB blocks rather than individual
key-value pairs dramatically reduces the index size and improves I/O efficiency:
one 4 KB read fetches many key-value pairs at once.

**Why an immutable file?** Immutability makes SSTables trivially safe to read
concurrently. Multiple readers can access the same SSTable file without any
locks. A write never touches an existing file — it creates a new one.

### Levels

SSTables are organized into levels (L0, L1, L2, ...). Each level has a size
limit that is a fixed multiple of the previous level (typically 10×).

```
Level 0 (L0):  Recently flushed SSTables.
               Keys may OVERLAP between files in L0.
               L0 has no size limit per file, just a count limit (e.g., ≤4 files).

Level 1 (L1):  Compacted SSTables.
               Keys DO NOT overlap between files in L1.
               Total size limit: e.g., 10 MB.

Level 2 (L2):  Total size limit: 100 MB (10× L1).

Level 3 (L3):  Total size limit: 1 GB.

...and so on.

Why no overlap in L1+?
  Non-overlapping files mean a read needs to check AT MOST ONE file per level
  (binary search on the file's key range). This keeps read amplification
  bounded: at most 1 file per level from L1 onwards.

Why does L0 allow overlap?
  L0 is populated by flushed memtables. We flush the memtable as-is — there's
  no time to sort/merge with existing L0 files. Overlap is tolerated at L0
  because L0 has so few files that checking all of them is cheap.
```

### Compaction

Compaction is the background process that merges SSTables to:
1. Remove tombstones (garbage-collect deleted keys).
2. Remove old versions of overwritten keys.
3. Enforce level size limits.
4. Restore the invariant that L1+ files don't overlap.

```
Leveled Compaction (LevelDB/RocksDB style):

  When L0 has ≥4 SSTables:
    1. Pick one L0 file.
    2. Find ALL L1 files whose key range overlaps this L0 file.
    3. Merge-sort all selected files.
    4. Write new, non-overlapping L1 files.
    5. Delete the old L0 and L1 files.

  Merge-sort of two SSTables (k,v entries in sorted order):

    SSTable A: [alice, carol, eve]
    SSTable B: [bob, carol, dave]    ← newer: carol here wins

    Merge iteration (like merge step of merge-sort):
      alice (A only)  → keep alice: "Wonderland"
      bob   (B only)  → keep bob: "Builder"
      carol (both!)   → B is newer → keep B's carol (discard A's carol)
      dave  (B only)  → keep dave: "Bowman"
      eve   (A only)  → keep eve: "Online"

    Result: [alice, bob, carol, dave, eve] — deduplicated, newest wins

Write amplification in leveled compaction:
  Each byte of data is rewritten once per level transition.
  With 7 levels: data is rewritten ~7 times from L0 → L6.
  RocksDB measures write amplification as: total bytes written / user bytes written.
  Typical: 10–30× write amplification in leveled compaction.
```

**Size-Tiered Compaction (Cassandra style):** An alternative strategy. Instead
of organizing files into strict levels, group files by similar size. When a
group reaches a threshold count, merge all files in the group into one larger
file. Lower write amplification than leveled, but higher space amplification
(up to 2× storage for temporary compaction). Better for write-heavy workloads.

```
Size-Tiered Compaction:

  Tier 1 (small files, ~10 MB each):
    [file A] [file B] [file C] [file D]  ← 4 files, trigger compaction

    Merge all 4 → one ~40 MB file → promote to Tier 2

  Tier 2 (medium files, ~40 MB each):
    [file E] [file F] [file G] [file H]  ← 4 files, trigger compaction

    Merge all 4 → one ~160 MB file → promote to Tier 3

  Trade-off vs leveled:
    Size-tiered: lower write amplification, higher read amplification
                 (more files to check per read; more overlap)
    Leveled:     higher write amplification, lower read amplification
                 (strictly bounded files per level; non-overlapping)
```

### Read Path in Detail

```
get("carol"):

  1. Check memtable (skip list lookup, O(log n)):
     Found? Return value. Tombstone? Return "not found".
     Not found? Continue.

  2. Check immutable memtable (being flushed, if any):
     Same as step 1.

  3. For each L0 SSTable (newest first):
     a. Check bloom filter → "Definitely NOT here"? Skip file. (NO DISK READ!)
     b. Binary search block index → find candidate block.
     c. Read block from disk → linear scan within block.
     d. Found? Return value. Tombstone? Return "not found".

  4. For each level L1, L2, ...:
     a. Binary search on file metadata → find the ONE file whose range covers "carol".
     b. Check bloom filter → skip if "Definitely NOT here".
     c. Binary search block index → find candidate block.
     d. Read block from disk → scan.
     e. Found or tombstone? Return.

  5. Key not found in any level → return "not found".

Read amplification:
  Worst case without bloom filters: check ALL SSTables = very slow.
  With bloom filters (1% FPR, 10 levels): ~10.1 bloom filter checks
    (≈ 10 per level × 1.01 for false positives) + ~1 disk read on average.
  Missing keys (the most common case in real workloads): bloom filter
    eliminates disk reads in 99% of cases. Just a cheap in-memory bit check.
```

### Write-Ahead Log (WAL)

The WAL guarantees durability: if the process crashes after acknowledging a
write, the write will survive — even though it was only in the memtable (RAM).

```
WAL record format:

  ┌──────────┬────────────┬──────────────┬───────────┬────────────────┐
  │ Sequence │ Record     │ Key length   │ Value     │ Key + Value    │
  │ number   │ type       │ (varint)     │ length    │ data           │
  │ (8 bytes)│ (1 byte)   │              │ (varint)  │                │
  └──────────┴────────────┴──────────────┴───────────┴────────────────┘

  Record types:
    0x01 = PUT   (key, value)
    0x02 = DELETE (key, tombstone)
    0x03 = BEGIN  (transaction start)
    0x04 = COMMIT (transaction commit)

WAL lifecycle:
  1. On every write: append record to WAL.
  2. On memtable flush: create a NEW WAL file. Mark old WAL file as obsolete.
  3. On recovery: replay WAL from last checkpoint to reconstruct memtable.
  4. Periodically: delete obsolete WAL files.

Crash recovery:
  Crash during memtable flush (memtable was in RAM, now lost):
    → Replay WAL from the beginning of the oldest incomplete flush.
    → Rebuild memtable from WAL records.
    → Discard any partially written SSTable.

  Crash during compaction:
    → Old SSTables still exist (compaction writes NEW files, then deletes old ones).
    → On restart, see the new files and the old files.
    → Determine which compaction finished (check manifest) and clean up.
```

### Manifest (MANIFEST file)

The *manifest* is a log of every structural change to the SSTable set:
which files were added, which were deleted, which level each file belongs to.
This allows crash recovery without scanning the entire directory.

```
Manifest records (MANIFEST file, append-only):

  {op: "add_file",    level: 0, file_id: 42, smallest: "alice", largest: "eve"}
  {op: "add_file",    level: 1, file_id: 43, smallest: "alice", largest: "dave"}
  {op: "delete_file", level: 0, file_id: 42}

On startup: replay manifest to reconstruct which files belong to which level.
```

### Snapshot Reads and Sequence Numbers

Every key-value pair is stamped with a *sequence number* — a monotonically
increasing integer assigned at write time. Sequence numbers enable:

1. **Multi-version reads:** "Read the state of the database as of sequence
   number N." Compaction only discards versions older than the oldest active
   snapshot.

2. **Version ordering during merges:** When two SSTables have the same key,
   the higher sequence number wins.

```
Sequence numbers in practice:

  write("alice", "v1")   → (alice, v1, seq=100)
  write("alice", "v2")   → (alice, v2, seq=101)
  write("bob",   "v1")   → (bob, v1, seq=102)

  Snapshot at seq=100: read alice → "v1"
  Snapshot at seq=101: read alice → "v2"
  Current read: read alice → "v2"  (most recent)

  During compaction of a file containing both seq=100 and seq=101 for alice:
    → If no snapshot is pinned at or below seq=100: discard seq=100, keep seq=101.
    → If a snapshot is pinned at seq=100: keep BOTH versions (don't compact them).
```

## Representation

### Core Data Structures

```python
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Iterator

class RecordType(Enum):
    PUT       = auto()
    TOMBSTONE = auto()

@dataclass
class MemEntry:
    """
    One entry in the memtable.
    A TOMBSTONE entry represents a deletion — it has no value.
    The sequence number orders entries from newest (highest) to oldest (lowest).
    """
    key:     Any
    value:   Any | None       # None if this is a tombstone
    record_type: RecordType
    seq:     int              # sequence number; higher = newer

@dataclass
class BlockIndexEntry:
    """
    Points to one data block within an SSTable file.
    The block index lives at the end of the SSTable file.
    """
    first_key:  Any           # the smallest key in this block
    offset:     int           # byte offset of block within the file
    size:       int           # size of block in bytes (for reading exactly one block)

@dataclass
class SSTableMeta:
    """
    Metadata about one SSTable file. Kept in memory for fast access.
    The actual (key, value) data lives on disk.
    """
    file_id:    int
    level:      int
    path:       str           # path to the file on disk
    min_key:    Any           # smallest key in this file
    max_key:    Any           # largest key in this file
    size_bytes: int
    bloom:      BloomFilter   # DT22 — loaded into RAM, saves disk seeks

@dataclass
class LSMTree:
    """
    Top-level structure holding the entire LSM tree state.
    """
    memtable:           SkipList[Any, MemEntry]  # DT20 — mutable, in RAM
    immutable_memtable: SkipList | None          # being flushed to disk
    levels:             list[list[SSTableMeta]]  # levels[i] = SSTables at level i
    wal_path:           str                      # path to current WAL file
    seq:                int                      # next sequence number
    snapshot_seqs:      set[int]                 # active snapshot sequence numbers
```

### SSTable File Layout (Binary Format)

```
Byte layout of one SSTable file:

  ┌─────────────────────────────────────────────────────┐
  │ Block 0 (4096 bytes):                               │
  │   Entry: [key_len: u32][val_len: u32][key][val]     │
  │   Entry: [key_len: u32][val_len: u32][key][val]     │
  │   Entry: [key_len: u32][0xFFFFFFFF][key]            │  ← tombstone (val_len sentinel)
  │   ...                                               │
  │   Padding to fill block to 4096 bytes               │
  ├─────────────────────────────────────────────────────┤
  │ Block 1 (4096 bytes): ...                           │
  ├─────────────────────────────────────────────────────┤
  │ Block N: ...                                        │
  ├─────────────────────────────────────────────────────┤
  │ Block Index:                                        │
  │   n_blocks: u32                                     │
  │   Entry 0: [first_key_len: u32][first_key][offset: u64][size: u32]
  │   Entry 1: ...                                      │
  ├─────────────────────────────────────────────────────┤
  │ Bloom Filter (serialized bit array):                │
  │   m: u64, k: u32, bits: [u8 × ceil(m/8)]           │
  ├─────────────────────────────────────────────────────┤
  │ Footer (fixed size, 40 bytes):                      │
  │   block_index_offset: u64                           │
  │   bloom_filter_offset: u64                          │
  │   n_entries: u64                                    │
  │   magic: u64  (= 0x4C534D54524545 = "LSMTREE")     │
  └─────────────────────────────────────────────────────┘
```

## Algorithms (Pure Functions)

```python
# ─── Write path ─────────────────────────────────────────────────────────────

def put(tree: LSMTree, key: Any, value: Any) -> LSMTree:
    """
    Insert or update a key-value pair.

    Order of operations:
      1. Append to WAL (durable write — survives crashes).
      2. Insert into memtable (fast, in-memory).
      3. If memtable is full, trigger a flush (may be async).

    The write is considered durable after step 1.
    Step 2 makes it visible to reads.
    Time: O(log n) for memtable insert + O(1) amortized for WAL.
    """
    new_seq = tree.seq + 1
    entry = MemEntry(key=key, value=value, record_type=RecordType.PUT, seq=new_seq)

    wal_append(tree.wal_path, entry)           # step 1: write to disk (WAL)
    new_memtable = tree.memtable.insert(key, entry)  # step 2: update memtable

    new_tree = replace(tree, memtable=new_memtable, seq=new_seq)

    if memtable_is_full(new_tree):             # step 3: maybe flush
        return trigger_flush(new_tree)
    return new_tree

def delete(tree: LSMTree, key: Any) -> LSMTree:
    """
    Delete a key by inserting a TOMBSTONE marker.

    A tombstone is NOT a removal from the memtable — it is an entry with
    record_type=TOMBSTONE. This is necessary because older versions of the
    key may still exist in SSTables on disk. The tombstone propagates down
    through compaction, eventually deleting all older versions.

    Time: same as put().
    """
    new_seq = tree.seq + 1
    tombstone = MemEntry(key=key, value=None, record_type=RecordType.TOMBSTONE, seq=new_seq)
    wal_append(tree.wal_path, tombstone)
    new_memtable = tree.memtable.insert(key, tombstone)
    return replace(tree, memtable=new_memtable, seq=new_seq)

# ─── Read path ──────────────────────────────────────────────────────────────

def get(tree: LSMTree, key: Any, snapshot_seq: int | None = None) -> Any | None:
    """
    Read the most recent value for key (or value at snapshot_seq).

    Search order (newest-first):
      1. Memtable
      2. Immutable memtable (if a flush is in progress)
      3. Level 0 SSTables (newest first; may overlap with each other)
      4. Level 1, 2, ... (at most 1 file per level; non-overlapping)

    At each step: if we find a TOMBSTONE, return None immediately.
    If we find a PUT, return its value.
    If we find nothing, continue to the next step.

    The bloom filter at each SSTable lets us skip files where the key
    DEFINITELY does not exist — typically eliminating 99% of disk reads.

    Time: O(log n) memtable + O(k × log F) SSTable checks
      where k = number of SSTables checked, F = entries per SSTable.
    """
    target_seq = snapshot_seq if snapshot_seq is not None else tree.seq

    # 1. Check memtable
    entry = tree.memtable.search(key)
    if entry is not None and entry.seq <= target_seq:
        return None if entry.record_type == RecordType.TOMBSTONE else entry.value

    # 2. Check immutable memtable
    if tree.immutable_memtable is not None:
        entry = tree.immutable_memtable.search(key)
        if entry is not None and entry.seq <= target_seq:
            return None if entry.record_type == RecordType.TOMBSTONE else entry.value

    # 3. Check L0 SSTables (newest first — they may overlap)
    for sst in reversed(tree.levels[0]):
        if not sst.bloom.contains(key):   # DT22: skip if definitely absent
            continue                       # (no disk read — just bit checks!)
        entry = sstable_get(sst, key, target_seq)
        if entry is not None:
            return None if entry.record_type == RecordType.TOMBSTONE else entry.value

    # 4. Check L1, L2, ... (at most 1 file per level)
    for level in tree.levels[1:]:
        sst = find_sstable_for_key(level, key)  # binary search on key ranges
        if sst is None:
            continue
        if not sst.bloom.contains(key):    # DT22: skip if definitely absent
            continue
        entry = sstable_get(sst, key, target_seq)
        if entry is not None:
            return None if entry.record_type == RecordType.TOMBSTONE else entry.value

    return None  # key not found in any level

def sstable_get(sst: SSTableMeta, key: Any, seq: int) -> MemEntry | None:
    """
    Read a single key from an SSTable file on disk.

    1. Load block index (cached in memory — small).
    2. Binary search block index for the block that could contain key.
    3. Read that ONE block from disk (one I/O, typically 4 KB).
    4. Linear scan within the block for the key.

    Note: only one disk read per SSTable lookup (step 3). This is the
    point of the block index — avoid reading the entire file.
    """
    block_idx = load_block_index(sst.path)  # O(log B) binary search
    block_entry = binary_search_blocks(block_idx, key)
    if block_entry is None:
        return None  # key is below or above this file's range
    block_data = read_block(sst.path, block_entry.offset, block_entry.size)  # ONE disk read
    return scan_block(block_data, key, seq)  # linear scan within 4 KB block

# ─── Flush (memtable → SSTable) ─────────────────────────────────────────────

def flush_memtable(tree: LSMTree) -> LSMTree:
    """
    Write the immutable memtable to a new Level 0 SSTable file.

    Steps:
      1. Iterate memtable in sorted key order (O(n) — skip list supports this).
      2. Group entries into 4 KB blocks.
      3. Write blocks sequentially to a new file.
      4. Write block index.
      5. Write bloom filter.
      6. Write footer.
      7. Update the manifest with the new file.
      8. Discard the old WAL file (replaced by new WAL started at flush time).
      9. Clear the immutable memtable.

    Time: O(n) where n = number of entries. All writes are sequential.
    """
    ...

# ─── Compaction ─────────────────────────────────────────────────────────────

def compact_level(tree: LSMTree, level: int) -> LSMTree:
    """
    Compact Level `level` into Level `level+1`.

    Strategy: pick one file from `level`, find all overlapping files in
    `level+1`, merge-sort them, write new non-overlapping files to `level+1`,
    delete the old files from both levels.

    The merge step is a standard k-way merge (like DT02 merge-sort):
      - Use a min-heap of (current_key, SSTable_iterator) pairs.
      - Always take the smallest key.
      - When two iterators produce the same key, keep the one with the
        higher sequence number (newer); discard the older.
      - Skip tombstones if no snapshot is pinned below their sequence number.

    Time: O(N log k) where N = total entries, k = number of SSTables being merged.
    All reads and writes are sequential — compaction is I/O-efficient.
    """
    ...

def merge_entries(
    iterators: list[Iterator[MemEntry]],
    min_snapshot_seq: int | None
) -> Iterator[MemEntry]:
    """
    K-way merge of sorted SSTable iterators.
    Deduplicates entries by key (keeps highest seq), drops obsolete tombstones.

    Rules:
      1. When two iterators have the same key, emit only the one with higher seq.
      2. If the emitted entry is a TOMBSTONE and no snapshot is pinned at or
         below its seq, drop the tombstone entirely (it has served its purpose).
      3. Otherwise emit the entry.

    This is the core of compaction's garbage collection.
    """
    ...

# ─── Crash Recovery ─────────────────────────────────────────────────────────

def recover(data_dir: str) -> LSMTree:
    """
    Reconstruct LSMTree state after a crash.

    Steps:
      1. Read the manifest to find which SSTable files are valid.
         (Avoids trusting the directory listing, which may have partial files.)
      2. Load SSTableMeta for each valid file (reads footer and bloom filter).
      3. Find the newest WAL file (incomplete memtable from before crash).
      4. Replay WAL records to rebuild the memtable.
      5. Return a consistent LSMTree.

    Time: O(W) for WAL replay where W = WAL entries since last flush.
    """
    ...
```

## Public API

```python
from typing import Any, Generic, TypeVar, Iterator
from contextlib import contextmanager

K = TypeVar("K")
V = TypeVar("V")

class LSMTree(Generic[K, V]):
    """
    A Log-Structured Merge-Tree: a write-optimized, persistent key-value store.

    Trade-offs vs B+ tree (DT12):
      Writes:        Much faster (append-only WAL + in-memory memtable)
      Reads:         Slightly slower (may check multiple levels)
      Space:         Higher overhead (multiple versions until compaction)
      Durability:    Same (WAL provides crash safety)
      Range scans:   Efficient at the SSTable level (sorted files)

    Use this when: you have many more writes than reads (e.g., logging,
                   time series, event sourcing, audit trails).
    Use B+ tree when: reads dominate, or random-access latency is critical.
    """

    def __init__(
        self,
        data_dir: str,
        memtable_size_bytes: int = 64 * 1024 * 1024,  # 64 MB default
        level_size_multiplier: int = 10,
        block_size_bytes: int = 4096,
        bloom_fpr: float = 0.01,
    ) -> None:
        """
        Open (or create) an LSM tree rooted at data_dir.
        If data_dir contains an existing tree, recover its state.
        """
        ...

    # ─── Writes ──────────────────────────────────────────────────────
    def put(self, key: K, value: V) -> None:
        """
        Insert or update key.
        Durable after this call returns (WAL has been flushed).
        O(log n) amortized.
        """
        ...

    def delete(self, key: K) -> None:
        """
        Delete key by inserting a tombstone.
        Does not immediately reclaim space — compaction cleans up later.
        O(log n) amortized.
        """
        ...

    def __setitem__(self, key: K, value: V) -> None: ...
    def __delitem__(self, key: K) -> None: ...

    # ─── Reads ───────────────────────────────────────────────────────
    def get(self, key: K) -> V | None:
        """
        Read most recent value for key, or None if not present.
        O(log n) in the common case (bloom filter skips most SSTables).
        """
        ...

    def __getitem__(self, key: K) -> V: ...  # raises KeyError
    def __contains__(self, key: K) -> bool: ...

    def range_scan(self, low: K, high: K) -> list[tuple[K, V]]:
        """
        Return all (key, value) pairs where low ≤ key ≤ high, sorted.
        Merges results from memtable and all SSTable levels.
        O(log n + k) where k = number of results.
        """
        ...

    # ─── Snapshots ───────────────────────────────────────────────────
    @contextmanager
    def snapshot(self):
        """
        Create a point-in-time read snapshot.
        Reads within the context block see the database as of snapshot creation.
        Compaction will not delete versions needed by active snapshots.

        Example:
            with db.snapshot() as snap:
                v1 = snap.get("alice")
                # ... do other work that modifies "alice" ...
                v2 = snap.get("alice")  # still returns v1's value
        """
        ...

    # ─── Maintenance ─────────────────────────────────────────────────
    def compact(self, level: int = 0) -> None:
        """
        Manually trigger compaction of the given level.
        Normally runs automatically in the background.
        Useful for testing or for one-time bulk-load optimization.
        """
        ...

    def flush(self) -> None:
        """
        Flush the current memtable to Level 0 immediately.
        Useful before a clean shutdown to avoid long WAL replay on next open.
        """
        ...

    # ─── Iteration ───────────────────────────────────────────────────
    def __iter__(self) -> Iterator[K]:
        """Iterate all keys in sorted order (merge from all levels)."""
        ...

    def items(self) -> Iterator[tuple[K, V]]:
        """Iterate all (key, value) pairs in sorted order."""
        ...

    # ─── Stats ───────────────────────────────────────────────────────
    def stats(self) -> dict:
        """
        Return diagnostic information:
          - memtable_entries: int
          - memtable_size_bytes: int
          - level_file_counts: list[int]  (one per level)
          - level_size_bytes: list[int]
          - estimated_keys: int           (approximate, from bloom filters)
          - write_amplification: float    (total bytes written / user bytes)
          - bloom_filter_size_bytes: int  (total across all SSTables)
        """
        ...

    # ─── Lifecycle ───────────────────────────────────────────────────
    def close(self) -> None:
        """Flush memtable and close all file handles cleanly."""
        ...

    def __enter__(self) -> "LSMTree": ...
    def __exit__(self, *_) -> None: self.close()
```

## Composition Model

The LSM tree composes three primitives from earlier layers:

```
LSMTree
  ├── SkipList  (DT20) — the memtable; sorted + fast inserts + sequential scan
  ├── BloomFilter (DT22) — one per SSTable; eliminates disk reads for missing keys
  └── BlockRAM  (F00) — page-aligned I/O for SSTable files
```

### Python

```python
# Python: memtable is a SkipList from DT20
class LSMTree:
    def __init__(self, data_dir: str, ...):
        self._memtable = SkipList()              # DT20
        self._immutable: SkipList | None = None
        self._levels: list[list[SSTableMeta]] = [[] for _ in range(7)]
        self._wal = open(os.path.join(data_dir, "wal.log"), "ab")
        self._seq = 0
        self._data_dir = data_dir
```

### Rust

```rust
// Rust: crossbeam-skiplist for concurrent memtable; memmap2 for SSTable I/O
use crossbeam_skiplist::SkipMap;
use memmap2::Mmap;

pub struct LSMTree<K: Ord, V> {
    memtable:   Arc<SkipMap<K, MemEntry<V>>>,     // DT20 analog
    levels:     Arc<RwLock<Vec<Vec<SSTableMeta>>>>,
    wal:        Arc<Mutex<File>>,
    seq:        AtomicU64,
    compactor:  JoinHandle<()>,                    // background thread
}
```

### Go

```go
// Go: use a custom skip list (DT20) or sync.Map for memtable
type LSMTree[K constraints.Ordered, V any] struct {
    memtable  *SkipList[K, memEntry[V]]  // DT20
    levels    [][]sstableMeta
    wal       *os.File
    seq       atomic.Uint64
    mu        sync.RWMutex
    compactCh chan struct{}
}
```

### TypeScript

```typescript
// TypeScript: in-memory demo (no real disk I/O in browser context)
class LSMTree<K, V> {
  private memtable: SkipList<K, MemEntry<V>>;  // DT20
  private levels: SSTableMeta[][];
  private seq: number = 0;

  constructor(private readonly compareFn: (a: K, b: K) => number) {
    this.memtable = new SkipList(compareFn);
    this.levels = Array.from({ length: 7 }, () => []);
  }
}
```

## Test Strategy

### Unit Tests

```python
# 1. Basic put/get round-trip
def test_put_get():
    db = LSMTree(tmp_dir())
    db.put("alice", "Wonderland")
    assert db.get("alice") == "Wonderland"
    assert db.get("missing") is None

# 2. Overwrite: latest value wins
def test_overwrite():
    db = LSMTree(tmp_dir())
    db.put("alice", "v1")
    db.put("alice", "v2")
    assert db.get("alice") == "v2"

# 3. Delete via tombstone
def test_delete():
    db = LSMTree(tmp_dir())
    db.put("alice", "Wonderland")
    db.delete("alice")
    assert db.get("alice") is None

# 4. Delete non-existent key is a no-op (no error)
def test_delete_missing():
    db = LSMTree(tmp_dir())
    db.delete("ghost")  # should not raise
    assert db.get("ghost") is None

# 5. Keys survive a flush to SSTable
def test_survives_flush():
    db = LSMTree(tmp_dir(), memtable_size_bytes=1)  # tiny memtable → flush immediately
    db.put("alice", "v1")
    db.put("bob", "v2")
    db.flush()
    assert db.get("alice") == "v1"
    assert db.get("bob") == "v2"
    assert len(db._levels[0]) == 1  # one SSTable at L0

# 6. Bloom filter skips non-matching SSTables (count disk reads)
def test_bloom_filter_skips_reads():
    db = LSMTree(tmp_dir())
    for i in range(1000):
        db.put(f"key_{i}", f"val_{i}")
    db.flush()

    # "definitely_absent" was never written — bloom filter should say NO
    reads_before = db.stats()["disk_reads"]
    result = db.get("definitely_absent")
    reads_after = db.stats()["disk_reads"]

    assert result is None
    assert reads_after == reads_before  # bloom filter prevented ALL disk reads

# 7. Range scan returns all keys in range, sorted
def test_range_scan():
    db = LSMTree(tmp_dir())
    for i in range(100):
        db.put(i, i * 10)
    results = db.range_scan(30, 39)
    assert results == [(k, k * 10) for k in range(30, 40)]

# 8. Range scan across memtable and SSTables
def test_range_scan_cross_levels():
    db = LSMTree(tmp_dir(), memtable_size_bytes=512)
    for i in range(200):
        db.put(i, i)  # fills multiple SSTables + leaves some in memtable
    results = db.range_scan(50, 150)
    assert len(results) == 101
    assert all(k == v for k, v in results)

# 9. Crash recovery from WAL
def test_crash_recovery():
    data_dir = tmp_dir()
    db1 = LSMTree(data_dir)
    db1.put("alice", "Wonderland")
    db1.put("bob", "Builder")
    # Simulate crash: do NOT call db1.close() or db1.flush()
    del db1  # just drop the object (WAL still on disk)

    db2 = LSMTree(data_dir)  # recovery: replays WAL
    assert db2.get("alice") == "Wonderland"
    assert db2.get("bob") == "Builder"

# 10. Compaction deduplicates and removes tombstones
def test_compaction_cleans_up():
    db = LSMTree(tmp_dir(), memtable_size_bytes=256)
    db.put("alice", "v1")
    db.flush()
    db.put("alice", "v2")   # newer version
    db.flush()
    db.delete("bob")         # tombstone for key never written
    db.flush()

    db.compact(level=0)      # merge L0 → L1

    # After compaction: only alice: "v2" should exist; v1 and bob tombstone gone
    assert db.get("alice") == "v2"
    assert db.get("bob") is None
    assert len(db._levels[0]) == 0   # L0 is empty
    assert len(db._levels[1]) == 1   # one file at L1

# 11. Snapshot isolation
def test_snapshot():
    db = LSMTree(tmp_dir())
    db.put("key", "original")

    with db.snapshot() as snap:
        db.put("key", "updated")           # write happens during snapshot
        assert snap.get("key") == "original"  # snapshot sees old value
        assert db.get("key") == "updated"     # current view sees new value

# 12. Iteration is sorted and complete
def test_iteration():
    db = LSMTree(tmp_dir(), memtable_size_bytes=256)
    expected = {f"k{i:04d}": i for i in range(500)}
    for k, v in expected.items():
        db.put(k, v)
    # Multiple flushes will have happened
    result = dict(db.items())
    assert result == expected
```

### Coverage Targets

- 95%+ line coverage
- All code paths: memtable hit, immutable memtable hit, L0 SSTable hit,
  L1+ SSTable hit, bloom filter skip, key not found
- All recovery paths: clean shutdown, WAL replay, partial flush recovery
- Compaction paths: L0→L1, tombstone elimination, snapshot pinning
- SSTable paths: block boundary, key at first block, key at last block

### Performance Benchmarks

```python
# 13. Write throughput: LSM should dramatically outperform a naive B-tree
#     for sequential write-heavy workloads
def bench_write_throughput():
    db = LSMTree(tmp_dir())
    start = time.time()
    for i in range(1_000_000):
        db.put(f"key_{i:08d}", f"val_{i}")
    elapsed = time.time() - start
    print(f"1M writes: {elapsed:.2f}s ({1_000_000/elapsed:,.0f} writes/sec)")
    # Expect: > 100K writes/sec on modern hardware

# 14. Read amplification with bloom filters
def bench_read_amplification():
    db = LSMTree(tmp_dir())
    # Write 100K keys and flush
    for i in range(100_000):
        db.put(f"key_{i}", i)
    db.flush()

    # Read 10K present keys
    present_time = timeit(lambda: db.get(f"key_{random.randint(0,99999)}"), n=10_000)
    # Read 10K absent keys (bloom filter should eliminate most disk reads)
    absent_time  = timeit(lambda: db.get(f"absent_{random.randint(0,99999)}"), n=10_000)

    print(f"Present read avg: {present_time*1000:.2f}ms")
    print(f"Absent  read avg: {absent_time*1000:.2f}ms")
    # Absent reads should be nearly as fast as present reads (bloom filter)
```

## Future Extensions

**Concurrent memtable:** Replace the single-threaded skip list with a
concurrent skip list (e.g., Java's `ConcurrentSkipListMap` or Rust's
`crossbeam-skiplist`). Multiple writer threads can insert into the memtable
simultaneously without blocking each other. The WAL still requires ordering,
but writes can be batched.

**Column families:** Partition the keyspace into independent sub-stores called
column families (RocksDB's key feature). Each column family has its own
memtable, WAL, and SSTable set, but they share the same compaction thread pool.
This lets different key types have different compaction policies (e.g., metadata
keys vs. data blocks).

**Block compression:** Compress each data block using LZ4 or Snappy before
writing to disk (CMP00). The block index stores compressed sizes. Reading
a block requires a decompression step after the disk read. Typical compression
ratios: 2–4×, halving storage and doubling effective I/O bandwidth. RocksDB
compresses ~90% of its production data.

**Prefix compression within blocks:** Within a single sorted block, consecutive
keys often share a prefix (e.g., `user:alice`, `user:bob`, `user:carol`).
Store the shared prefix once per block, then store only the unique suffixes.
Reduces block size, increases the number of entries per block, reduces disk
reads.

**Tiered + Leveled hybrid (Universal Compaction):** An alternative to pure
leveled compaction. Recent files use size-tiered rules (fewer rewrites). Once
files reach a size threshold, switch to leveled rules. This gives better write
amplification for bursty write workloads while maintaining bounded read
amplification for stable workloads. Used in RocksDB's universal compaction mode.

**Write stalls:** When compaction falls behind writes, the LSM tree risks
running out of L0 slots. RocksDB implements *write stalls*: if L0 file count
exceeds a threshold, slow down writes; if it exceeds a higher threshold, stop
writes entirely until compaction catches up. A production LSM must implement
back-pressure to prevent this.

**Transactions (optimistic concurrency control):** Wrap multiple puts/deletes
in an atomic batch. Buffer all writes in a `WriteBatch` structure; apply the
batch atomically by writing a single WAL group record and applying all entries
to the memtable under a lock. For read-write transactions, use MVCC with
optimistic conflict detection: read your snapshot seq, write your batch, verify
no conflicting writes happened since your snapshot, then commit.

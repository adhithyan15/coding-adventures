# DB01c — C++17 Profile for Logical SLC NAND Flash

This profile pins a native C++17 implementation of [DB01](DB01-nand-flash.md)
in `code/packages/cpp/nand-flash`. It is an independent implementation and
does not bind to the C package.

## Public API and ownership

Use namespace `ca::nand_flash`, value configuration, and a non-copyable device:

```cpp
struct Geometry {
    std::int64_t blocks;
    std::int64_t pages_per_block;
    std::int64_t page_size_bytes;
};

struct Config {
    Geometry geometry;
    std::optional<std::int64_t> max_erase_cycles; // nullopt means no configured ceiling
};

struct Snapshot {
    std::vector<std::vector<std::string>> pages;
    std::vector<std::uint64_t> erase_counts;
    std::vector<std::vector<bool>> programmed;
};

class Device {
public:
    explicit Device(Config config);
    Device(const Device&) = delete;
    Device& operator=(const Device&) = delete;

    Geometry geometry() const noexcept;
    std::vector<std::uint8_t> read(std::int64_t block, std::int64_t page) const;
    void program(std::int64_t block, std::int64_t page,
                 const std::vector<std::uint8_t>& data);
    void erase_block(std::int64_t block);
    std::uint64_t erase_count(std::int64_t block) const;
    bool is_programmed(std::int64_t block, std::int64_t page) const;
    Snapshot snapshot() const;
};
```

`Snapshot` is a detached value with `pages` as nested lowercase-hex strings,
`erase_counts` as per-block `uint64_t` values, and `programmed` as nested
booleans, matching the shared fixture's state shape. Geometry is returned by
value. `read` returns a new byte vector. `program` copies all bytes into
device-owned storage and retains no caller reference. The vector reference is
the C++ byte-payload boundary: another element type cannot be passed, and a
null payload has no representation. A wrong-length vector raises the typed
payload exception with DB01's size message. Concurrent mutation of the input
vector during a call is a caller data race; keep it stable through the call.

The device owns flattened page bytes initialized to `0xFF`, one program marker
per page, and one erase counter per block. It is non-copyable to avoid accidental
duplication of media state. Callers serialize operations on a given device;
the implementation has no mutex or concurrent-command contract. Independent
devices share no mutable state.

## Geometry, limits, and mutation rules

Accept exactly DB01's geometry ranges: 1..1024 blocks, 1..256 pages per block,
and 1..65536 bytes per page, with at most 16 MiB of page storage. Reject
negative dimensions before conversion to `size_t`. Validate each field before
casting; check `page_count <= max_storage / page_size` before multiplying, and
confirm all accepted sizes fit `size_t`. A configured erase ceiling is
`std::optional<std::int64_t>` in 0..9007199254740991. With no configured
ceiling, the implementation still refuses an erase when the counter has
reached that safe-integer maximum.

A page may be programmed once between erases, including with all `0xFF`
bytes. Since a new page is all ones, every first full-page value is
representable by the 1-to-0 rule. Successful erase restores all pages in one
block, clears their markers, and increments only that block's counter.

For media commands, validate block before page. `program` validates address,
then exact vector size, then the page marker. `erase_block` validates the block
before checking its erase budget. Finish all checks before changing device
state. Any refusal preserves page bytes, markers, and erase counts. Query
methods return values rather than writing through caller-owned output pointers.

Use typed C++ exceptions derived from `NandFlashError`:

| Exception | Condition and exact `what()` text |
|---|---|
| `GeometryError` | Invalid dimension, storage product, or erase ceiling — `geometry values are outside supported bounds` |
| `AddressError` | Invalid block — `block index out of range`; invalid page — `page index out of range` |
| `PayloadError` | Wrong vector length — `page data must match page_size_bytes` |
| `ProgramError` | Page already programmed — `page has already been programmed since erase` |
| `EraseLimitError` | Configured or safe counter ceiling reached — `block erase limit reached` |

The exception class name and message are stable vector-visible behavior. The
static vector type makes DB01's “not bytes” input error unrepresentable at this
API boundary. Allocation failure propagates the standard `std::bad_alloc`; it
is not a shared media-vector error. Constructor and read/snapshot failures
leave no partially visible mutation.

## Canonical fixture adapter

The runtime uses the C++17 standard library. The test fixture adapter uses the
header-only `cpp/bounded-json` package within the boundary checked in
[DB-A03a](DB-A03a-cpp-bounded-json-audit.md). It preserves duplicate object
members, distinguishes integer tokens from booleans and floating tokens, checks
UTF-8 and trailing input, and accepts caller-tightened input, depth, node, and
allocation limits. It is a syntax parser, not a DB01 schema validator: the
adapter itself must reject duplicate, missing, unknown, wrongly typed, or
malformed fields and unknown operation/error names before replay.

Pin its test-only options to a 64 KiB byte cap, depth 64, 16384 nodes, and an
8 MiB requested-allocation budget. Read the canonical fixture from
`../../../specs/fixtures/database-media-v1/nand-flash.json` while running tests
from the package directory, checking file length before allocating the input
string. Every fixture number is an exact integer within signed 64-bit range;
the adapter must reject `Kind::floating` wherever an integer is required. It
must count object members and compare exact key sets because duplicate names
are preserved by the parser. Replay compares the named result or exact typed
error/message and every page, marker, and erase count after each operation.
The fixture remains an example trace; the independent model and generated
checks are separate evidence.

Compatibility check for the proposed adapter: the checked-in canonical file is
6171 bytes, has six top-level members, and uses integer-valued geometry. A
temporary `g++ -std=c++17 -pedantic-errors -Wall -Wextra -Werror` probe parsed
it under the limits above. A duplicate-key probe retained both members, and a
geometry field was reported as `Kind::integer`. The native and sanitizer
suite results are recorded in DB-A03a.

## Native validation plan

Provide `BUILD` and `BUILD_windows` fronts through the repository's C++
`iso-harness`, an empty runtime capability manifest, README, and changelog. The
test build may add `../bounded-json/include` to include paths; production
headers include only C++17 standard-library headers. Tests should include:

- canonical shared fixture replay with exact result, exception class/message,
  and full state after every operation;
- geometry endpoints, one-past-limit values, negative dimensions, safe product
  boundaries, and values that do not fit `size_t`;
- address-before-payload-before-marker refusal precedence, erase limits at zero
  and one, the last permitted absolute counter step, and full-state atomicity
  for every refusal. Use a private test seam or the runtime's counter
  calculation for the absolute-ceiling check;
- all-`0xFF` first programming, read-buffer independence, source-vector
  mutation after `program`, detached nested snapshots, and two-block erase
  isolation;
- an independent byte-array model over a fixed-seed operation stream, checking
  bytes, markers, counts, returned values, exception identity, and state after
  each step;
- bounded exhaustive histories of exactly six commands from
  `{read, program-FF, program-00, erase}` on a one-block/one-page/one-byte
  device with a one-erase ceiling: 4096 histories;
- malformed fixture inputs for duplicate keys, wrong token kinds, unknown or
  missing keys, invalid error names, truncated/trailing input, and each parser
  resource limit; and
- a bounded command-stream fuzz target checked against the independent model.

Use libFuzzer with Clang when available, with an explicit maximum input and
operation count; keep deterministic malformed seeds in the ordinary suite so
GCC-only hosts still exercise the rejection paths. Run native tests under
AddressSanitizer and UndefinedBehaviorSanitizer, run gcov/coverage when
available, and report exact compiler, flags, counts, seed, fuzz budget, and
unreached branches. Do not label finite histories, fixture replay, sanitizers,
or fuzzing as a proof.

## Local validation environment

The local toolchains are Debian GCC 14.2.0 and Clang 19.1.7. A standalone
Clang smoke run verified that the host can link and execute libFuzzer with
AddressSanitizer and UndefinedBehaviorSanitizer. Record the package-specific
fuzz budget and results with the other native evidence.

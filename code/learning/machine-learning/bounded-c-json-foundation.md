# Owning a checkpoint's syntax before owning a learner

NN47 is a C parsing prerequisite, not a C training loop. A future dense loader
must distinguish `1` from `1.0`, `true` and `null` before it can safely call any
matrix operation. It also needs storage whose lifetime is independent of the
input bytes. Those are parser responsibilities; deciding whether a matrix shape
or activation is allowed remains the future framework's responsibility.

The public API produces one owned document with const views. The parser appends
nodes and strings to a bounded arena. On malformed input or allocation failure,
it walks the allocation chain and releases everything; it does not try to free
a half-built recursive tree. After success the caller can discard the original
buffer. Releasing the document resets its handle and invalidates its views.

For example, parsing `{"step":1,"step":2,"step\u0000x":3}` retains three ordered
members and their complete key lengths. Silent last-wins replacement or C-string
truncation would hide information a closed checkpoint loader needs to reject.
The parser therefore does not pretend an object is already validated metadata.

Strings are checked and counted before allocating their exact decoded storage.
UTF-8 continuation bytes, Unicode scalar bounds and surrogate pairs are checked
before emission. Numbers pass JSON's grammar before conversion. Exact int64
integer tokens retain their type; fractional/exponent tokens stay floats.
The implementation does not change the application's numeric locale.

Four independent envelopes limit bytes, nested containers, value count and total
allocation. The tests reach the actual boundaries, inject failure at every
allocation in representative trees, and assert no partial owners survive.
Sanitizers complement those assertions, while strict compiler builds check
portable C syntax. A percentage from a failed test run is not coverage evidence.

Parsing all 55 checkpoint fixture documents is only syntax inventory evidence.
Most intentionally invalid checkpoints are perfectly valid JSON. Only a later
native dense loader can provide schema rejection and actual training/resume;
six-native interchange will need its own real process exchanges after that.
See the [package](../../packages/c/bounded-json/README.md) and
[NN47 contract](../../specs/NN47-c-bounded-json-foundation.md).

# DT04e — Checked C++17 heap and installable package

**Status:** selected specification for DS-H-CPP-A. This profile extends the current C++ heap package without changing its legacy behavior. It does not include an operation benchmark or capability escalation.

## 1. Scope and compatibility

The current package at `code/packages/cpp/heap` is a header-only C++17 library. Preserve `ca::binary_heap<T, Compare>`, `ca::min_heap<T>`, `ca::max_heap<T>`, `heap_sort`, `nlargest`, and `nsmallest`, including signatures and behavior. New checked behavior is additive.

The existing comparator convention is authoritative: `higher(a, b)` is true when `a` belongs closer to the root. `std::less<T>` therefore gives a min-heap and `std::greater<T>` a max-heap. Equal-priority elements are not stable. Tests compare priority and identity membership without requiring a particular order among ties.

The public header must directly include every standard declaration it uses, including `<utility>` for `std::move`. All public targets remain C++17 and header-only.

## 2. Bounded API

Add:

```cpp
namespace ca {

enum class push_result { inserted, full };

class range_capacity_error : public std::length_error {
public:
    explicit range_capacity_error(std::size_t accepted_count);
    std::size_t accepted_count() const noexcept;
};

template <typename T, typename Higher = std::less<T>>
class bounded_priority_queue;

template <typename T>
using bounded_min_heap = bounded_priority_queue<T, std::less<T>>;

template <typename T>
using bounded_max_heap = bounded_priority_queue<T, std::greater<T>>;

}
```

The ordinary constructors are `explicit bounded_priority_queue(std::size_t limit)` and `bounded_priority_queue(std::size_t limit, Higher higher)`. The first participates only when `Higher` is default-constructible; the second stores the supplied comparator and supports stateful, non-default-constructible comparators. There is no hidden default-comparator requirement on the explicit-comparator overload. The range overloads are `bounded_priority_queue(std::size_t limit, InputIt first, InputIt last)` (available only for default-constructible `Higher`) and `bounded_priority_queue(std::size_t limit, InputIt first, InputIt last, Higher higher)`. Both take iterator and comparator arguments by value and are constrained to C++17 input iterators. Iterator/comparator parameter copying and constructor argument construction occur before the constructor body; their effects cannot be prevented by its validation.

The concrete public signatures are `std::size_t size() const noexcept`, `bool empty() const noexcept`, `std::size_t limit() const noexcept`, `push_result try_push(const T&)`, `push_result try_push(T&&)`, `std::optional<std::reference_wrapper<const T>> peek() const noexcept`, `std::optional<T> pop()`, and `void clear() noexcept`. The lvalue `try_push` overload participates only for copy-constructible `T`. `try_push` and `pop` are not `noexcept` because copying, comparison, and temporary path allocation may fail as specified below. Constructors propagate their documented validation, iterator, and payload exceptions; range overflow throws `range_capacity_error` initialized with the accepted prefix length. `limit()` reports the configured logical limit, not the vector's physical capacity. There is no resize operation. `clear()` empties the queue and retains its limit.

`try_push` returns `push_result::full` when `size()==limit`; it does not throw for ordinary full rejection. `peek()` returns `std::optional<std::reference_wrapper<const T>>`; `pop()` returns `std::optional<T>`. An empty queue returns `nullopt` from both. A successful mutation may invalidate every previously returned peek reference. The borrow is valid only while its owning queue remains alive and unmoved, and until a successful mutating operation that may invalidate it; it must not outlive the queue. Failed operations covered by the guarantees below preserve its validity. For `T = std::optional<int>`, tests must distinguish an empty outer `peek()` result from a present reference to a stored `nullopt`, and an empty outer `pop()` result from a present `optional<T>` whose inner value is empty.

## 3. Type and comparator contract

The bounded type uses an internal `slot<T>{T value;}` in `std::vector<slot<T>>`. This avoids `std::vector<bool>`'s proxy-reference specialization while retaining value semantics. `std::vector<bool>` range input is supported: each proxy is materialized as a `bool` value. No API exposes physical storage layout or promises payload-object contiguity.

`T` must be nothrow move-constructible, nothrow move-assignable, and nothrow destructible. It may be move-only. The lvalue insertion overload participates only when `T` is copy-constructible; that copy may throw. `Higher` must be nothrow move-constructible and nothrow destructible so queue moves and destruction cannot fail after changing comparator state. A comparator move must preserve the ordering/value semantics of the destination queue. It must be callable as a boolean comparison of `const T&` values and obey a deterministic strict weak ordering while elements are queued. Mutating priorities behind the queue's back or re-entering/mutating the queue from a comparison violates the contract. Observational counters are permitted; external comparator effects are not rolled back.

Raw function-pointer comparators are supported only when non-null. The constructor checks a raw function pointer and throws `std::invalid_argument` for null before any heap-side range comparison, dereference, or increment. Raw member-pointer comparator types are rejected at compile time. Do not probe arbitrary functor truthiness. An empty `std::function` comparator is not specially inspected; its `std::bad_function_call` propagates from comparison before queue mutation.

The queue itself is move-constructible and not copyable. Copy construction, copy assignment, and move assignment are deleted. Its nothrow move constructor transfers the contents, comparator, and limit while preserving the destination's payload values and ordering semantics, then leaves the source empty with limit zero; the moved-from comparator is never called because every insertion is rejected as full. This gives the moved-from object defined query/clear/destruction behavior without relying on an arbitrary comparator's moved-from ordering. No queue swap or assignment operation is exposed.

## 4. Capacity, ordering, and arithmetic

Capacity is a `std::size_t` logical item limit. Zero is valid. Before any heap-performed input-iterator comparison, dereference, or increment, validate that the requested limit is no larger than `std::vector<slot<T>>::max_size()`; otherwise throw `std::length_error`. Validate a raw function-pointer comparator for null before those heap-performed iterator operations as well. Invalid limit takes precedence over null comparator. Parameter copying/construction, including `Higher` or iterator arguments, happens before constructor entry and may have its own effects. These validations do not allocate. Do not reserve the full configured limit eagerly.

A full `try_push` returns before copying or moving the argument, invoking the comparator, allocating, or changing the queue. Thus a rejected external rvalue remains unchanged and caller-owned. Lvalue insertion snapshots its argument before any possible vector growth, so `try_push(*queue.peek())` is supported even if growth later reallocates the vector. An rvalue that aliases an element already stored in the same queue is a documented precondition violation; no self-alias scan is performed.

Use overflow-safe geometric vector growth only when the needed size exceeds `vector::capacity()`. The growth target is at least the needed size and no greater than the logical limit or storage `max_size()`. Compute it without overflowing `size_t`; never reserve `size()+1` on every push and never claim an exact physical vector capacity.

For sift-down, establish that a left child exists before computing `2*i+1`; compute a right child only from the proven-safe left index. The implementation must not use unchecked `2*i+1`/`2*i+2` expressions on arbitrary indices. A small internal helper may be tested at `size_t` boundaries without allocating a huge vector.

## 5. Exception, reference, and ownership guarantees

Insertion is staged as a transaction:

1. Check full first.
2. For lvalue input, make the candidate copy before comparisons or growth; copy failure leaves contents and references unchanged.
3. For rvalue input, compare through the original argument reference and defer its move until commit.
4. Compute the complete sift path and allocate temporary path storage before queue mutation. Comparator and temporary-storage exceptions propagate with queue contents, size, heap invariant, and existing peek references unchanged.
5. If geometric vector growth is needed, perform `reserve(target)` as the final potentially throwing operation. With the required nothrow payload moves, allocation failure leaves the queue logically unchanged; successful reserve may relocate storage, but after it succeeds no throwing comparator, allocation, or payload move remains.
6. Commit only by nothrow moves/assignments and nonthrowing destruction. A successful insertion may invalidate peek references. An accepted external rvalue is consumed only during this nonthrowing commit; comparator, temporary-allocation, or reserve failure leaves it unmoved. Full rejection leaves it untouched.

`pop()` computes every comparator decision and any temporary path allocation before moving the root or rearranging storage. Comparator failure therefore leaves the queue and peek references unchanged. Once commit begins it uses only the specified nothrow payload operations. `clear()` is nonthrowing under the required destructor constraint and invalidates references.

The contract does not claim deterministic injection of real `std::bad_alloc`. Tests must not label impossible-size rejection as an out-of-memory experiment. The allocation guarantee is based on the stated type requirements and standard-vector behavior; the `max_size()` test covers size validation, not allocator exhaustion.

## 6. One-pass iterator construction

Provide an iterator-pair constructor taking `(limit, first, last, higher)`, accepting C++17 input iterators and using incremental insertion. Validate the limit and raw function comparator before the first iterator comparison/dereference/increment. An invalid limit takes precedence over a null function comparator; neither case performs heap-side iterator operations.

Consume each accepted item at most once. Construct a `T` from the iterator's value (including proxy references) and insert it incrementally. For `k` accepted elements, construction makes `O(k log k)` comparison work and uses geometric storage growth; it is not Floyd heapification and is not `O(k)` comparisons.

When the queue has accepted `limit` items, compare the current iterator with `last`. If an extra item exists, throw `range_capacity_error` with `accepted_count()==limit`, without dereferencing or incrementing that over-limit iterator position. The failed construction destroys its accepted prefix and returns no partial queue. Input consumption is not rolled back. A single-pass iterator may prefetch while incrementing past an accepted value, and iterator comparison/copy itself may have source-specific effects; the guarantee is only that the heap does not dereference or increment the extra position. Exceptions from iterator comparison, dereference, increment, value construction, or comparison propagate; any source effects remain.

`range_capacity_error` is constructed with the count of accepted elements; its public `accepted_count() const noexcept` returns that value. An empty range succeeds at zero capacity. A nonempty range at zero capacity throws with accepted count zero. Limit validation precedes even checking whether a range is empty.

## 7. Independent validation

Retain the existing legacy tests and independently verify their integer min/max order, heapify, sorting, and top/bottom-k behavior. New tests use a sorted-vector/multiset model, not the implementation's heap layout. After every command compare all nondestructively observable state: `size`, `empty`, operation result, root priority and identity, and ownership of inserted/rejected values. The noncopyable queue exposes no complete membership snapshot, so verify the full identity/priority multiset by draining each history's final queue and comparing it with the model. Because every finite history prefix is also enumerated as its own history, this observes the complete state at every reachable finite prefix without adding an inspection API. Count and report these drain probes separately from generated commands. Tied values may emerge in any order.

The finite exhaustive matrix is fixed: capacities 0–3, min and max orders, and every history of length 0–5 over six commands (`try_push(-1)`, `try_push(0)`, `try_push(1)`, `pop`, `peek`, `clear`). It contains `(1+6+36+216+1296+7776) * 4 * 2 = 74,648` histories. Check the model after every generated command and perform the separately counted final-drain probes above. Add 32 fixed 64-bit seeds with 4,096 generated commands each (131,072 generated commands total), with logged seed, capacity, command mapping, and complete replay output on failure; bound successful-run logs and report generated-command counts separately from drain probes. Include a deterministic byte-to-command corpus and a small reducer that preserves the actual failure kind and produces a 1-minimal history under single-command deletion, rechecking that the reduced history reproduces the same failure kind. Do not claim a global minimum or universal proof.

Cover move-only payloads, `vector<bool>`, nested optional payloads, duplicates/ties, custom comparator state and move, raw null function comparator, compile-time member-pointer rejection, queue special-member traits/source state, full-rvalue ownership, lvalue aliasing through peek, throwing lvalue copies, comparator throws at push/pop path depths, empty/singleton/zero/max-size boundaries, iterator one-pass/overflow/prefetch caveat, and moved-from queries. Assert failed comparator/copy paths preserve queue contents and borrowed-reference validity. Do not write tests that depend on stable equal-key ordering.

Negative controls must be connected to the actual model oracle: an over-capacity adapter and a wrong-root/removal adapter must each produce a reproducible oracle mismatch. An assertion that merely finds certain command kinds in a history is not sufficient.

A counting comparator may increment an external observation counter. Record comparison counts against the sifting height for push/pop; tests can enforce the algorithmic comparison envelope. State that one growth-triggering push may move `Theta(n)` elements and that geometric growth only amortizes total storage movement across a sequence. Comparison counts and asymptotic bounds are not wall-clock, cache, locality, or universal performance evidence.

Keep the repository's strict ISO compiler matrix: GCC and Clang where CI requires them, and the actual MSVC route on Windows. Separately measure production-header line coverage, excluding tests and harness, and require at least 95% as a readiness gate. If the installed toolchain cannot produce a valid measurement, treat that as a blocker to resolve; do not waive the gate. Do not report branch coverage unless it is separately measured. Preserve exact commands, compiler/tool versions, counts, skipped cases, and tool limitations.

## 8. Installable C++ package and BUILD integration

Add a minimal CMake interface package with project/package ID `ca_heap`, package version `0.2.0`, and imported target `ca::heap`. Install the public header plus relocatable CMake config, version, and exported target files. Use build/install include interfaces and no runtime/link dependency. Copy the package-local `LICENSE` byte-for-byte from the repository-owned MIT text at `code/packages/typescript/heap/LICENSE` blob `4a4dc9a35858c625d7a0833d0a63bb2a7e05563c`; this preserves actual repository attribution rather than inventing license text.

Wire the same installed-prefix smoke into both platform BUILD fronts, in addition to the existing ISO test route. Both `BUILD` and `BUILD_windows` must declare `# build-tool: deps=c/iso-harness`, since both routes source that package. Keep both commands calling their package runner; do not leave an uncalled consumer fixture.

The smoke installs to a fresh prefix, copies/relocates it to a second prefix, makes the original unavailable for consumer configuration, then configures/builds/runs a consumer outside the repository. The consumer uses only `find_package(ca_heap CONFIG REQUIRED)` and `ca::heap`, with `PATHS <relocated-prefix> NO_DEFAULT_PATH`, package registries disabled, and ambient CMake package paths cleared. Assert that the resolved config and imported include directories are inside the relocated prefix and do not point back into source or the original install. Run checked commands with captured output and exit status. This is local installed-artifact evidence, not registry publication.

The current package has no `required_capabilities.json` and no direct runtime OS effects. Keep the checked library pure; do not add a nonempty capability profile or change the held collector/approval work. Tests must avoid fixture reads or process/clock APIs in the heap test program. Package BUILD/install activity is build tooling, not a runtime capability claim. Do not add Warrant or a registry dependency.

## 9. Documentation and exclusions

Update the heap README with checked API examples, move-only/borrowed-peek lifetime, capacity/range/error rules, comparator order, package installation, and exact exception guarantees. Add a `0.2.0` changelog entry. Keep explanations in the public header beginner-readable per repository literate-source rules.

Do not add a timer, heap-construction benchmark, profiler, cache experiment, PMU claim, ranking, or ad hoc performance threshold. The shared C++ collector remains held by its own profile/approval and CI work. This package item makes no registry-publication claim; registry/toolchain publication policy is separate.

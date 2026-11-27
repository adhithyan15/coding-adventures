# Heap (C++)

This header-only C++17 package contains two priority-queue APIs. The original
`ca::binary_heap<T, Compare>`, `ca::min_heap<T>`, and `ca::max_heap<T>` remain
unchanged. The additive `ca::bounded_priority_queue<T, Higher>` API lets a
caller put a logical ceiling on the number of queued values.

The comparator keeps the existing convention: `Higher(a, b)` is true when
`a` should appear closer to the root. The default `std::less<T>` gives a
min-heap, and `std::greater<T>` gives a max-heap. Equal-priority values are not
stable; callers should not depend on which tied value appears first.

## Legacy API

```cpp
#include "heap.hpp"

ca::min_heap<int> h;
h.push(5);
h.push(1);
h.push(3);
int top = h.peek().value();       // 1
std::optional<int> value = h.pop(); // 1

auto sorted = ca::heap_sort(std::vector<int>{3, 1, 4, 1});
auto top3 = ca::nlargest(std::vector<int>{5, 1, 8, 3, 9}, 3);
```

The legacy `peek` and `pop` return `std::optional<T>`. Constructing a legacy
heap from `std::vector<T>` uses bottom-up heapify.

## Bounded API

The aliases `ca::bounded_min_heap<T>` and `ca::bounded_max_heap<T>` use the
same comparator order as their legacy counterparts:

```cpp
#include "heap.hpp"

ca::bounded_min_heap<int> queue(3); // logical limit, with no eager allocation
queue.try_push(8);
queue.try_push(2);

auto borrowed = queue.peek();
if (borrowed) {
    int root = borrowed->get(); // 2
}

if (queue.try_push(5) == ca::push_result::inserted) {
    // accepted
}
if (queue.try_push(1) == ca::push_result::full) {
    // ordinary capacity rejection; an rvalue argument remains caller-owned
}

std::optional<int> next = queue.pop();
queue.clear(); // empties the queue but keeps its configured limit
```

`size()`, `empty()`, and `limit()` are constant-time, nonthrowing queries.
Capacity zero is valid. The queue does not grow its logical limit, and it does
not allocate storage equal to the limit up front. `try_push` returns
`push_result::full` before copying, moving, comparing, or growing.

`peek()` returns `std::optional<std::reference_wrapper<const T>>`. The outer
optional says whether the queue has a root; the reference wrapper borrows the
root value. Keep the queue alive and unmoved, and stop using a borrowed
reference after a successful queue mutation. A failed copy/comparison/allocation
path covered by the API guarantee leaves existing borrows valid. For a nested
payload such as `T = std::optional<int>`, an empty outer result means the queue
has no value, while a present reference can refer to a stored `nullopt`.

`pop()` returns `std::optional<T>`; for the same nested payload its result is
an outer optional containing an inner optional. The queue is move-constructible
but deliberately not copyable or assignable. Moving transfers its contents,
comparator, and limit; the source becomes empty with limit zero. Values must be
nothrow move-constructible, move-assignable, and destructible. Comparators must
be nothrow move-constructible and destructible. Move-only payloads are
supported; the lvalue insertion overload is available only when `T` is
copy-constructible.

## Construction from a range

The range constructors accept C++17 input iterators and insert incrementally:

```cpp
std::vector<int> values{7, 2, 5};
ca::bounded_min_heap<int> from_range(values.size(), values.begin(), values.end());
```

An explicit comparator can be passed as the final argument when `Higher` is
stateful or is not default-constructible. The requested limit is checked
against storage `max_size()` before the queue performs iterator comparison,
dereference, or increment. Iterator/comparator argument copies occur before
constructor entry and may have their own effects.

If the source has more elements than the limit, construction throws
`ca::range_capacity_error`; `accepted_count()` reports how many values were
inserted before overflow. The failed constructor returns no partial queue, and
already consumed input cannot be rolled back. The queue does not dereference
or increment the over-limit iterator position. A single-pass iterator may
prefetch while incrementing after the last accepted value, which is an
iterator-source behavior the queue cannot prevent. Construction takes
O(k log k) comparison work for k accepted items; it is incremental insertion,
not Floyd heapification.

## Failure and ownership guarantees

The bounded insertion path compares the proposed value and records its sift
path before changing the queue. A lvalue value is copied first, which makes
`queue.try_push(queue.peek()->get())` safe even if later storage growth moves
the vector. For an external rvalue, the queue delays moving from the caller
until comparisons, path allocation, and vector reserve have succeeded. A full
rejection or a comparator/path-allocation/reserve failure leaves the external
rvalue untouched. Passing an rvalue that aliases an element already stored in
that queue is a precondition violation.

The queue reserves geometrically only when its physical vector is too small.
Reserve is its final potentially throwing insertion step; after successful
reserve, the documented nothrow value moves finish the commit. Therefore a
growth-triggering push may move Theta(n) slots, while geometric growth
amortizes storage movement across a sequence of pushes. The sift comparison
work is O(log n); that bound is not a worst-case bound on all storage movement.

`pop()` also performs every comparison and allocates its temporary sift path
before it moves the root. Comparator failure therefore leaves the queue and
existing peek references unchanged. `clear()` is nonthrowing and invalidates
references. A raw function-pointer comparator must be non-null; a null pointer
is rejected with `std::invalid_argument` before heap-side range operations.
An empty `std::function` comparator propagates `std::bad_function_call` from
the comparison. Comparators must have deterministic strict weak ordering,
must not mutate queued priorities, and must not re-enter or mutate the queue.
Observational comparison counters are allowed; unrelated external comparator
effects are not rolled back.

## Install the package

The CMake package exports the relocatable target `ca::heap` as package
`ca_heap` version `0.2.0`. After installing to a prefix, a consumer can use:

```cmake
find_package(ca_heap CONFIG REQUIRED)
target_link_libraries(my_program PRIVATE ca::heap)
```

The package installs its header, CMake config/version/export files, and
`share/licenses/ca_heap/LICENSE`. The BUILD smoke test relocates the installed
prefix and checks that the consumer resolves the moved config, include path,
and exact pinned license bytes without using the source tree or original
install path. This is local installed-artifact validation, not a registry
publication claim.

## Development

Run `BUILD` on Unix or `BUILD_windows` on Windows. Both use the shared pure-ISO
C++ harness and also run the relocated installed-consumer smoke. The C++
collector benchmark is a separate held work item; this package does not add a
timer or claim cache behavior. Tests use an independent sorted-value model and
comparison counts. These are finite correctness and algorithmic checks, not
wall-clock performance evidence or proof for every input.

To measure production-header line coverage with GCC, build and run the same
test source with coverage enabled, then ask `gcov` for the header report:

```sh
g++ -std=c++17 -O0 --coverage \
  -I ../../c/iso-harness/include -I include tests/heap_test.cpp \
  -o heap-coverage
./heap-coverage
gcov -b -c -o heap-coverage-heap_test.gcno tests/heap_test.cpp
python tests/aggregate_gcov_source_lines.py heap.hpp.gcov
```

The helper reports unique physical executable lines in `heap.hpp`: a line is
covered when any template instantiation executes it. This differs from gcov's
expanded instantiated-line and branch totals, which count template copies and
must be reported separately. The model and coverage counts are validation
evidence, not timing or cache claims.

This package ports the Rust heap API. See the [Rust heap](../../rust/heap) and
the [C heap](../../c/heap/README.md).

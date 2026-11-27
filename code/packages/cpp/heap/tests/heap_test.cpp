// Tests for the C++ binary heap, using the header-only iso_test.h harness.
// Covers min/max heaps, push/pop/peek, empty behavior, heapify-construction,
// heap_sort, and nlargest/nsmallest.
#include "iso_test.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <functional>
#include <iterator>
#include <limits>
#include <optional>
#include <stdexcept>
#include <type_traits>
#include <utility>
#include <vector>

#include "heap.hpp"

namespace {

struct Record {
    int priority;
    int identity;
};

struct RecordOrder {
    bool maximum = false;
    bool operator()(const Record &left, const Record &right) const noexcept {
        return maximum ? left.priority > right.priority
                       : left.priority < right.priority;
    }
};

struct NonDefaultOrder {
    explicit NonDefaultOrder(bool maximum) noexcept : maximum(maximum) {}
    bool maximum;
    bool operator()(int left, int right) const noexcept {
        return maximum ? left > right : left < right;
    }
};

struct MoveOnly {
    explicit MoveOnly(int value) noexcept : value(value) {}
    MoveOnly(const MoveOnly &) = delete;
    MoveOnly &operator=(const MoveOnly &) = delete;
    MoveOnly(MoveOnly &&other) noexcept : value(other.value) {
        other.value = -999;
    }
    MoveOnly &operator=(MoveOnly &&other) noexcept {
        value = other.value;
        other.value = -999;
        return *this;
    }
    int value;
};

struct MoveOnlyOrder {
    bool operator()(const MoveOnly &left, const MoveOnly &right) const noexcept {
        return left.value < right.value;
    }
};

struct CopyFault {
    explicit CopyFault(int value) noexcept : value(value) {}
    CopyFault(const CopyFault &other) : value(other.value) {
        if (throw_copy) {
            throw std::runtime_error("copy fault");
        }
    }
    CopyFault(CopyFault &&other) noexcept : value(other.value) {
        other.value = -999;
    }
    CopyFault &operator=(CopyFault &&other) noexcept {
        value = other.value;
        other.value = -999;
        return *this;
    }
    static bool throw_copy;
    int value;
};
bool CopyFault::throw_copy = false;

struct CopyFaultOrder {
    bool operator()(const CopyFault &left, const CopyFault &right) const noexcept {
        return left.value < right.value;
    }
};

struct ThrowOrder {
    int *calls;
    int *throw_on;
    bool operator()(const Record &left, const Record &right) const {
        ++*calls;
        if (*throw_on > 0 && *calls == *throw_on) {
            throw std::runtime_error("comparison fault");
        }
        return left.priority < right.priority;
    }
};

struct CountingOrder {
    int *calls;
    bool operator()(int left, int right) const noexcept {
        ++*calls;
        return left < right;
    }
};

bool record_less(const Record &left, const Record &right) {
    return left.priority < right.priority;
}

struct CountedInput {
    using iterator_category = std::input_iterator_tag;
    using value_type = Record;
    using difference_type = std::ptrdiff_t;
    using pointer = const Record *;
    using reference = const Record &;

    const Record *current = nullptr;
    int *comparisons = nullptr;
    int *dereferences = nullptr;
    int *increments = nullptr;

    reference operator*() const {
        ++*dereferences;
        return *current;
    }
    CountedInput &operator++() {
        ++*increments;
        ++current;
        return *this;
    }
    CountedInput operator++(int) {
        CountedInput previous = *this;
        ++*this;
        return previous;
    }
    friend bool operator==(const CountedInput &left,
                           const CountedInput &right) {
        ++*left.comparisons;
        return left.current == right.current;
    }
    friend bool operator!=(const CountedInput &left,
                           const CountedInput &right) {
        return !(left == right);
    }
};

enum class CommandKind { push0, push1, push2, pop, peek, clear };
struct Command {
    CommandKind kind;
};

struct HistoryResult {
    bool ok = true;
    std::size_t command = 0;
    const char *failure = "";
};

std::size_t generated_command_count = 0;
std::size_t final_drain_probe_count = 0;

int priority_for(CommandKind kind) {
    switch (kind) {
    case CommandKind::push0: return -1;
    case CommandKind::push1: return 0;
    case CommandKind::push2: return 1;
    default: return 0;
    }
}

bool is_push(CommandKind kind) {
    return kind == CommandKind::push0 || kind == CommandKind::push1 ||
           kind == CommandKind::push2;
}

int best_priority(const std::vector<Record> &model, bool maximum) {
    int best = model.front().priority;
    for (const Record &item : model) {
        if (maximum ? item.priority > best : item.priority < best) {
            best = item.priority;
        }
    }
    return best;
}

bool eligible_identity(const std::vector<Record> &model, const Record &item,
                       int expected_priority) {
    for (const Record &candidate : model) {
        if (candidate.identity == item.identity &&
            candidate.priority == expected_priority) {
            return true;
        }
    }
    return false;
}

template <typename Queue>
HistoryResult run_history(std::size_t capacity, bool maximum,
                          const std::vector<Command> &commands) {
    Queue queue(capacity, RecordOrder{maximum});
    std::vector<Record> model;
    int next_identity = 1;
    auto fail = [](std::size_t at, const char *kind) {
        return HistoryResult{false, at, kind};
    };

    for (std::size_t at = 0; at < commands.size(); ++at) {
        ++generated_command_count;
        const Command command = commands[at];
        if (is_push(command.kind)) {
            const Record input{priority_for(command.kind), next_identity};
            const ca::push_result actual = queue.try_push(input);
            const bool accepts = model.size() < capacity;
            if ((actual == ca::push_result::inserted) != accepts) {
                return fail(at, "push-result");
            }
            if (accepts) {
                model.push_back(input);
                ++next_identity;
            } else if (input.identity != next_identity ||
                       input.priority != priority_for(command.kind)) {
                return fail(at, "rejected-input-ownership");
            }
        } else if (command.kind == CommandKind::pop) {
            std::optional<Record> actual = queue.pop();
            if (model.empty()) {
                if (actual) return fail(at, "pop-empty");
            } else {
                const int expected = best_priority(model, maximum);
                if (!actual || actual->priority != expected ||
                    !eligible_identity(model, *actual, expected)) {
                    return fail(at, "pop-value");
                }
                for (std::size_t i = 0; i < model.size(); ++i) {
                    if (model[i].identity == actual->identity) {
                        model.erase(model.begin() + static_cast<std::ptrdiff_t>(i));
                        break;
                    }
                }
            }
        } else if (command.kind == CommandKind::peek) {
            const auto actual = queue.peek();
            if (model.empty()) {
                if (actual) return fail(at, "peek-empty");
            } else {
                const int expected = best_priority(model, maximum);
                if (!actual || actual->get().priority != expected ||
                    !eligible_identity(model, actual->get(), expected)) {
                    return fail(at, "peek-value");
                }
            }
        } else {
            queue.clear();
            model.clear();
        }

        if (queue.size() != model.size() || queue.empty() != model.empty() ||
            queue.limit() != capacity) {
            return fail(at, "observable-size-state");
        }
        const auto root = queue.peek();
        if (model.empty()) {
            if (root) return fail(at, "observable-empty-root");
        } else {
            const int expected = best_priority(model, maximum);
            if (!root || root->get().priority != expected ||
                !eligible_identity(model, root->get(), expected)) {
                return fail(at, "observable-root");
            }
        }
    }

    // A complete identity comparison is possible at final drain without
    // exposing storage or requiring a copy of the noncopyable queue.
    while (!model.empty()) {
        ++final_drain_probe_count;
        const int expected = best_priority(model, maximum);
        std::optional<Record> actual = queue.pop();
        if (!actual || actual->priority != expected ||
            !eligible_identity(model, *actual, expected)) {
            return fail(commands.size(), "final-drain");
        }
        for (std::size_t i = 0; i < model.size(); ++i) {
            if (model[i].identity == actual->identity) {
                model.erase(model.begin() + static_cast<std::ptrdiff_t>(i));
                break;
            }
        }
    }
    if (!queue.empty() || queue.size() != 0) {
        return fail(commands.size(), "final-empty");
    }
    return HistoryResult{};
}

void report_history_failure(const std::vector<Command> &commands,
                            std::size_t capacity, bool maximum,
                            const HistoryResult &result) {
    std::printf("bounded heap history failed: capacity=%lu order=%s at=%lu kind=%s commands=",
                static_cast<unsigned long>(capacity), maximum ? "max" : "min",
                static_cast<unsigned long>(result.command), result.failure);
    for (const Command &command : commands) {
        const char code = is_push(command.kind)
                              ? static_cast<char>('0' + static_cast<int>(command.kind))
                              : (command.kind == CommandKind::pop ? 'p' :
                                 command.kind == CommandKind::peek ? 'q' : 'c');
        std::printf("%c", code);
    }
    std::printf("\n");
}

struct HistoryCounter {
    std::size_t histories = 0;
    bool maximum = false;
    std::size_t capacity = 0;
    std::array<Command, 5> commands{};
};

template <typename Queue>
void enumerate_histories(HistoryCounter &counter, std::size_t depth,
                         std::size_t target) {
    if (depth == target) {
        ++counter.histories;
        std::vector<Command> history(counter.commands.begin(),
                                     counter.commands.begin() +
                                         static_cast<std::ptrdiff_t>(depth));
        const HistoryResult result = run_history<Queue>(
            counter.capacity, counter.maximum, history);
        if (!result.ok) {
            report_history_failure(history, counter.capacity, counter.maximum,
                                   result);
            ISO_CHECK_MSG(result.ok, "finite history matches independent model");
        }
        return;
    }
    for (int code = 0; code < 6; ++code) {
        counter.commands[depth].kind = static_cast<CommandKind>(code);
        enumerate_histories<Queue>(counter, depth + 1, target);
    }
}

template <typename Queue>
void enumerate_all(HistoryCounter &counter) {
    for (std::size_t length = 0; length <= 5; ++length) {
        enumerate_histories<Queue>(counter, 0, length);
    }
}

std::uint64_t next_random(std::uint64_t &state) {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    return state;
}

template <typename Queue>
void run_generated(std::uint64_t seed, bool maximum) {
    std::uint64_t state = seed;
    std::vector<Command> commands;
    commands.reserve(4096);
    for (std::size_t i = 0; i < 4096; ++i) {
        commands.push_back(Command{static_cast<CommandKind>(next_random(state) % 6)});
    }
    const HistoryResult result = run_history<Queue>(17, maximum, commands);
    if (!result.ok) {
        report_history_failure(commands, 17, maximum, result);
    }
    ISO_CHECK_MSG(result.ok, "seeded command history matches independent model");
}

// Deliberately faulty model adapters prove that the oracle catches two
// meaningful mistakes, and that reduction preserves the failure category.
enum class FailureKind { none, over_capacity, wrong_root };
struct FaultCommand { bool push; int value; };

FailureKind replay_fault(const std::vector<FaultCommand> &history,
                         std::size_t limit, bool accept_over_capacity,
                         bool remove_last) {
    std::vector<int> adapter;
    std::vector<int> model;
    for (const FaultCommand command : history) {
        if (command.push) {
            const bool expected_insert = model.size() < limit;
            const bool actual_insert = accept_over_capacity || expected_insert;
            if (expected_insert) model.push_back(command.value);
            if (actual_insert) adapter.push_back(command.value);
            if (actual_insert != expected_insert) return FailureKind::over_capacity;
        } else if (!model.empty()) {
            int expected = model.front();
            for (int value : model) if (value < expected) expected = value;
            const int actual = remove_last ? adapter.back() : adapter.front();
            if (actual != expected) return FailureKind::wrong_root;
            model.erase(std::find(model.begin(), model.end(), expected));
            adapter.erase(remove_last ? adapter.end() - 1 : adapter.begin());
        }
    }
    return FailureKind::none;
}

void print_fault_history(const char *label,
                         const std::vector<FaultCommand> &history) {
    std::printf("%s:", label);
    for (const FaultCommand command : history) {
        if (command.push) {
            std::printf(" push(%d)", command.value);
        } else {
            std::printf(" pop");
        }
    }
    std::printf("\n");
}

std::vector<FaultCommand> reduce_failure(std::vector<FaultCommand> history,
                                         std::size_t limit,
                                         bool accept_over_capacity,
                                         bool remove_last,
                                         FailureKind target) {
    bool reduced = true;
    while (reduced) {
        reduced = false;
        for (std::size_t i = 0; i < history.size(); ++i) {
            std::vector<FaultCommand> candidate = history;
            candidate.erase(candidate.begin() + static_cast<std::ptrdiff_t>(i));
            if (replay_fault(candidate, limit, accept_over_capacity, remove_last) == target) {
                history = candidate;
                reduced = true;
                break;
            }
        }
    }
    return history;
}

void test_fault_controls_and_reducer() {
    const std::vector<FaultCommand> over_capacity{{true, 1}, {true, 2}, {false, 0}};
    const auto over_kind = replay_fault(over_capacity, 1, true, false);
    const auto over_reduced = reduce_failure(over_capacity, 1, true, false, over_kind);
    if (over_kind != FailureKind::over_capacity ||
        replay_fault(over_reduced, 1, true, false) != over_kind) {
        std::printf("over-capacity control replay/reduction failed; original=%lu reduced=%lu\n",
                    static_cast<unsigned long>(over_capacity.size()),
                    static_cast<unsigned long>(over_reduced.size()));
        print_fault_history("over-capacity original", over_capacity);
        print_fault_history("over-capacity reduced", over_reduced);
    }
    ISO_CHECK(over_kind == FailureKind::over_capacity);
    ISO_CHECK(replay_fault(over_reduced, 1, true, false) == over_kind);
    for (std::size_t i = 0; i < over_reduced.size(); ++i) {
        auto candidate = over_reduced;
        candidate.erase(candidate.begin() + static_cast<std::ptrdiff_t>(i));
        ISO_CHECK(replay_fault(candidate, 1, true, false) != over_kind);
    }

    const std::vector<FaultCommand> wrong_root{{true, 1}, {true, 4}, {false, 0}};
    const auto wrong_kind = replay_fault(wrong_root, 2, false, true);
    const auto wrong_reduced = reduce_failure(wrong_root, 2, false, true, wrong_kind);
    if (wrong_kind != FailureKind::wrong_root ||
        replay_fault(wrong_reduced, 2, false, true) != wrong_kind) {
        std::printf("wrong-root control replay/reduction failed; original=%lu reduced=%lu\n",
                    static_cast<unsigned long>(wrong_root.size()),
                    static_cast<unsigned long>(wrong_reduced.size()));
        print_fault_history("wrong-root original", wrong_root);
        print_fault_history("wrong-root reduced", wrong_reduced);
    }
    ISO_CHECK(wrong_kind == FailureKind::wrong_root);
    ISO_CHECK(replay_fault(wrong_reduced, 2, false, true) == wrong_kind);
    for (std::size_t i = 0; i < wrong_reduced.size(); ++i) {
        auto candidate = wrong_reduced;
        candidate.erase(candidate.begin() + static_cast<std::ptrdiff_t>(i));
        ISO_CHECK(replay_fault(candidate, 2, false, true) != wrong_kind);
    }
}

void test_bounded_edges() {
    using Queue = ca::bounded_min_heap<int>;
    Queue zero(0);
    int untouched = 9;
    ISO_CHECK(zero.try_push(std::move(untouched)) == ca::push_result::full);
    ISO_CHECK_EQ_INT(untouched, 9);
    ISO_CHECK(zero.empty());
    ISO_CHECK(!zero.peek());
    ISO_CHECK(!zero.pop());

    Queue queue(4);
    ISO_CHECK(queue.try_push(5) == ca::push_result::inserted);
    const int &borrow = queue.peek()->get();
    ISO_CHECK(queue.try_push(borrow) == ca::push_result::inserted);
    ISO_CHECK_EQ_UINT(queue.size(), 2);
    ISO_CHECK_EQ_INT(queue.peek()->get(), 5);
    ISO_CHECK(queue.try_push(1) == ca::push_result::inserted);
    ISO_CHECK(queue.try_push(7) == ca::push_result::inserted);
    int full_value = 11;
    ISO_CHECK(queue.try_push(std::move(full_value)) == ca::push_result::full);
    ISO_CHECK_EQ_INT(full_value, 11);
    ISO_CHECK_EQ_INT(*queue.pop(), 1);
    queue.clear();
    ISO_CHECK(queue.empty());
    ISO_CHECK_EQ_UINT(queue.limit(), 4);

    ca::bounded_priority_queue<int, NonDefaultOrder> maximum(
        3, NonDefaultOrder(true));
    maximum.try_push(1);
    maximum.try_push(4);
    maximum.try_push(2);
    ISO_CHECK_EQ_INT(*maximum.pop(), 4);
    ca::bounded_priority_queue<int, NonDefaultOrder> moved_maximum(
        std::move(maximum));
    ISO_CHECK_EQ_UINT(moved_maximum.size(), 2);
    ISO_CHECK_EQ_UINT(moved_maximum.limit(), 3);
    ISO_CHECK_EQ_INT(moved_maximum.peek()->get(), 2);
    ISO_CHECK(maximum.empty());
    ISO_CHECK_EQ_UINT(maximum.limit(), 0);

    static_assert(std::is_move_constructible<Queue>::value,
                  "bounded queue is move constructible");
    static_assert(!std::is_copy_constructible<Queue>::value,
                  "bounded queue is not copy constructible");
    static_assert(!std::is_copy_assignable<Queue>::value,
                  "bounded queue is not copy assignable");
    static_assert(!std::is_move_assignable<Queue>::value,
                  "bounded queue is not move assignable");
    Queue source(3);
    source.try_push(8);
    Queue destination(std::move(source));
    ISO_CHECK_EQ_UINT(destination.size(), 1);
    ISO_CHECK_EQ_UINT(destination.limit(), 3);
    ISO_CHECK(source.empty());
    ISO_CHECK_EQ_UINT(source.limit(), 0);
    ISO_CHECK(source.try_push(2) == ca::push_result::full);
    source.clear();
    ISO_CHECK(!source.pop());

    const std::vector<bool> bool_values{true, false};
    ca::bounded_priority_queue<bool> booleans(3, bool_values.begin(),
                                               bool_values.end());
    ISO_CHECK(!*booleans.pop());
    ISO_CHECK(*booleans.pop());
    ISO_CHECK(!booleans.pop());

    ca::bounded_min_heap<std::optional<int>> nested(2);
    ISO_CHECK(!nested.peek());
    nested.try_push(std::nullopt);
    auto nested_peek = nested.peek();
    ISO_CHECK(nested_peek.has_value());
    ISO_CHECK(!nested_peek->get().has_value());
    auto nested_pop = nested.pop();
    ISO_CHECK(nested_pop.has_value());
    ISO_CHECK(!nested_pop->has_value());
    ISO_CHECK(!nested.pop());

    ca::bounded_priority_queue<MoveOnly, MoveOnlyOrder> move_only(
        1, MoveOnlyOrder{});
    MoveOnly first(6);
    ISO_CHECK(move_only.try_push(std::move(first)) == ca::push_result::inserted);
    ISO_CHECK_EQ_INT(first.value, -999);
    MoveOnly rejected(7);
    ISO_CHECK(move_only.try_push(std::move(rejected)) == ca::push_result::full);
    ISO_CHECK_EQ_INT(rejected.value, 7);
    ISO_CHECK_EQ_INT(move_only.pop()->value, 6);

    ca::bounded_priority_queue<CopyFault, CopyFaultOrder> copy_queue(
        4, CopyFaultOrder{});
    copy_queue.try_push(CopyFault(4));
    const CopyFault *before = &copy_queue.peek()->get();
    CopyFault candidate(1);
    CopyFault::throw_copy = true;
    bool copy_threw = false;
    try {
        copy_queue.try_push(candidate);
    } catch (const std::runtime_error &) {
        copy_threw = true;
    }
    CopyFault::throw_copy = false;
    ISO_CHECK(copy_threw);
    ISO_CHECK_EQ_UINT(copy_queue.size(), 1);
    ISO_CHECK(&copy_queue.peek()->get() == before);
    CopyFault::throw_copy = true;
    bool full_without_copy = false;
    ca::bounded_priority_queue<CopyFault, CopyFaultOrder> full_copy_queue(
        1, CopyFaultOrder{});
    full_copy_queue.try_push(CopyFault(8));
    try {
        full_without_copy =
            full_copy_queue.try_push(candidate) == ca::push_result::full;
    } catch (...) {
        full_without_copy = false;
    }
    CopyFault::throw_copy = false;
    ISO_CHECK(full_without_copy);

    int full_calls = 0;
    int full_throw_on = 0;
    ca::bounded_priority_queue<Record, ThrowOrder> full_comparator_queue(
        1, ThrowOrder{&full_calls, &full_throw_on});
    full_comparator_queue.try_push(Record{1, 1});
    full_calls = 0;
    full_throw_on = 1;
    Record not_compared{2, 2};
    const auto full_status = full_comparator_queue.try_push(std::move(not_compared));
    full_throw_on = 0;
    ISO_CHECK(full_status == ca::push_result::full);
    ISO_CHECK_EQ_INT(full_calls, 0);
    ISO_CHECK_EQ_INT(not_compared.identity, 2);

    using EmptyOrder = std::function<bool(const int &, const int &)>;
    ca::bounded_priority_queue<int, EmptyOrder> empty_function(
        2, EmptyOrder{});
    empty_function.try_push(1);
    int function_candidate = 0;
    bool bad_function = false;
    try {
        empty_function.try_push(std::move(function_candidate));
    } catch (const std::bad_function_call &) {
        bad_function = true;
    }
    ISO_CHECK(bad_function);
    ISO_CHECK_EQ_INT(function_candidate, 0);
    ISO_CHECK_EQ_UINT(empty_function.size(), 1);

    int calls = 0;
    int throw_on = 0;
    using ThrowingQueue = ca::bounded_priority_queue<Record, ThrowOrder>;
    ThrowingQueue throwing(16, ThrowOrder{&calls, &throw_on});
    for (int i = 0; i < 7; ++i) throwing.try_push(Record{i, i});
    const Record *stable_root = &throwing.peek()->get();
    const std::size_t stable_size = throwing.size();
    // Exercise failures at several points in insertion and removal paths.
    for (int failure_call : {1, 2, 3}) {
        calls = 0;
        throw_on = failure_call;
        Record external{-10, 90};
        bool push_threw = false;
        try {
            throwing.try_push(std::move(external));
        } catch (const std::runtime_error &) {
            push_threw = true;
        }
        throw_on = 0;
        ISO_CHECK(push_threw);
        ISO_CHECK_EQ_INT(external.priority, -10);
        ISO_CHECK_EQ_INT(external.identity, 90);
        ISO_CHECK_EQ_UINT(throwing.size(), stable_size);
        ISO_CHECK(&throwing.peek()->get() == stable_root);
    }
    for (int failure_call : {1, 2, 3}) {
        calls = 0;
        throw_on = failure_call;
        bool pop_threw = false;
        try {
            (void)throwing.pop();
        } catch (const std::runtime_error &) {
            pop_threw = true;
        }
        throw_on = 0;
        ISO_CHECK(pop_threw);
        ISO_CHECK_EQ_UINT(throwing.size(), stable_size);
        ISO_CHECK(&throwing.peek()->get() == stable_root);
    }
    ISO_CHECK(throwing.size() == stable_size);
    ISO_CHECK(&throwing.peek()->get() == stable_root);

    int comparisons = 0;
    ca::bounded_priority_queue<int, CountingOrder> counted(
        128, CountingOrder{&comparisons});
    int maximum_push_comparisons = 0;
    for (int value = 64; value >= 1; --value) {
        comparisons = 0;
        std::size_t height = 0;
        for (std::size_t index = counted.size(); index > 0;
             index = (index - 1) / 2) {
            ++height;
        }
        counted.try_push(value);
        ISO_CHECK(static_cast<std::size_t>(comparisons) <= height);
        if (comparisons > maximum_push_comparisons) {
            maximum_push_comparisons = comparisons;
        }
    }
    int maximum_pop_comparisons = 0;
    for (int n = 64; n > 0; --n) {
        comparisons = 0;
        std::size_t height = 0;
        for (std::size_t count = counted.size() - 1; count > 1; count /= 2) {
            ++height;
        }
        (void)counted.pop();
        ISO_CHECK(static_cast<std::size_t>(comparisons) <= 2 * height);
        if (comparisons > maximum_pop_comparisons) {
            maximum_pop_comparisons = comparisons;
        }
    }
    std::printf("counting comparator: max push=%d, max pop=%d; per-operation "
                "bounds use insertion/sift-down height\n",
                maximum_push_comparisons, maximum_pop_comparisons);
}

void test_range_construction() {
    const Record input[]{{5, 1}, {1, 2}, {3, 3}};
    int comparisons = 0;
    int dereferences = 0;
    int increments = 0;
    CountedInput first{input, &comparisons, &dereferences, &increments};
    CountedInput last{input + 3, &comparisons, &dereferences, &increments};
    bool overflow = false;
    try {
        ca::bounded_priority_queue<Record, RecordOrder> too_small(
            2, first, last, RecordOrder{});
    } catch (const ca::range_capacity_error &error) {
        overflow = true;
        ISO_CHECK_EQ_UINT(error.accepted_count(), 2);
    }
    ISO_CHECK(overflow);
    ISO_CHECK_EQ_INT(dereferences, 2);
    ISO_CHECK_EQ_INT(increments, 2);

    comparisons = dereferences = increments = 0;
    CountedInput empty_first{input, &comparisons, &dereferences, &increments};
    CountedInput empty_last{input, &comparisons, &dereferences, &increments};
    ca::bounded_priority_queue<Record, RecordOrder> empty(
        0, empty_first, empty_last, RecordOrder{});
    ISO_CHECK(empty.empty());
    ISO_CHECK_EQ_INT(comparisons, 1);
    ISO_CHECK_EQ_INT(dereferences, 0);
    ISO_CHECK_EQ_INT(increments, 0);

    comparisons = dereferences = increments = 0;
    CountedInput nonempty_first{input, &comparisons, &dereferences, &increments};
    CountedInput nonempty_last{input + 1, &comparisons, &dereferences, &increments};
    overflow = false;
    try {
        ca::bounded_priority_queue<Record, RecordOrder> none(
            0, nonempty_first, nonempty_last, RecordOrder{});
    } catch (const ca::range_capacity_error &error) {
        overflow = true;
        ISO_CHECK_EQ_UINT(error.accepted_count(), 0);
    }
    ISO_CHECK(overflow);
    ISO_CHECK_EQ_INT(dereferences, 0);
    ISO_CHECK_EQ_INT(increments, 0);

    comparisons = dereferences = increments = 0;
    CountedInput invalid_first{input, &comparisons, &dereferences, &increments};
    CountedInput invalid_last{input + 1, &comparisons, &dereferences, &increments};
    bool invalid_limit = false;
    try {
        ca::bounded_priority_queue<Record, RecordOrder> huge(
            (std::numeric_limits<std::size_t>::max)(), invalid_first,
            invalid_last, RecordOrder{});
    } catch (const std::length_error &) {
        invalid_limit = true;
    }
    ISO_CHECK(invalid_limit);
    ISO_CHECK_EQ_INT(comparisons, 0);
    ISO_CHECK_EQ_INT(dereferences, 0);
    ISO_CHECK_EQ_INT(increments, 0);

    using FunctionOrder = bool (*)(const Record &, const Record &);
    comparisons = dereferences = increments = 0;
    CountedInput null_first{input, &comparisons, &dereferences, &increments};
    CountedInput null_last{input + 1, &comparisons, &dereferences, &increments};
    bool null_rejected = false;
    try {
        ca::bounded_priority_queue<Record, FunctionOrder> null_queue(
            2, null_first, null_last, static_cast<FunctionOrder>(nullptr));
    } catch (const std::invalid_argument &) {
        null_rejected = true;
    }
    ISO_CHECK(null_rejected);
    ISO_CHECK_EQ_INT(comparisons, 0);
    ISO_CHECK_EQ_INT(dereferences, 0);
    ISO_CHECK_EQ_INT(increments, 0);

    comparisons = dereferences = increments = 0;
    CountedInput precedence_first{input, &comparisons, &dereferences, &increments};
    CountedInput precedence_last{input + 1, &comparisons, &dereferences, &increments};
    bool limit_precedes_null = false;
    try {
        ca::bounded_priority_queue<Record, FunctionOrder> invalid_both(
            (std::numeric_limits<std::size_t>::max)(), precedence_first,
            precedence_last, static_cast<FunctionOrder>(nullptr));
    } catch (const std::length_error &) {
        limit_precedes_null = true;
    }
    ISO_CHECK(limit_precedes_null);
    ISO_CHECK_EQ_INT(comparisons, 0);
    ISO_CHECK_EQ_INT(dereferences, 0);
    ISO_CHECK_EQ_INT(increments, 0);

    ca::bounded_priority_queue<Record, FunctionOrder> function_queue(
        3, input, input + 3, &record_less);
    ISO_CHECK_EQ_INT(function_queue.peek()->get().priority, 1);
}

void test_model_matrices() {
    std::printf("bounded model command codes: 0=push(-1), 1=push(0), "
                "2=push(1), p=pop, q=peek, c=clear\n");
    std::size_t histories = 0;
    for (std::size_t capacity = 0; capacity <= 3; ++capacity) {
        for (bool maximum : {false, true}) {
            HistoryCounter counter;
            counter.capacity = capacity;
            counter.maximum = maximum;
            if (maximum) {
                enumerate_all<ca::bounded_priority_queue<Record, RecordOrder>>(counter);
            } else {
                enumerate_all<ca::bounded_priority_queue<Record, RecordOrder>>(counter);
            }
            histories += counter.histories;
        }
    }
    ISO_CHECK_EQ_UINT(histories, 74648);
    ISO_CHECK_EQ_UINT(generated_command_count, 358320);
    const std::size_t finite_commands = generated_command_count;

    // Thirty-two recorded 64-bit seeds each generate 4096 commands. Each
    // generated trace is checked in both comparator directions.
    for (std::uint64_t seed_index = 0; seed_index < 32; ++seed_index) {
        const std::uint64_t seed = 0x9e3779b97f4a7c15ULL ^
                                   (seed_index * 0x100000001b3ULL);
        std::printf("bounded model seed[%lu]=0x%016llx\n",
                    static_cast<unsigned long>(seed_index),
                    static_cast<unsigned long long>(seed));
        run_generated<ca::bounded_priority_queue<Record, RecordOrder>>(seed,
                                                                        false);
        run_generated<ca::bounded_priority_queue<Record, RecordOrder>>(seed,
                                                                        true);
    }
    const std::size_t seeded_command_executions =
        generated_command_count - finite_commands;

    // A fixed byte corpus maps every byte to one of the six commands.
    std::vector<Command> byte_commands;
    for (unsigned int byte = 0; byte < 256; ++byte) {
        byte_commands.push_back(Command{static_cast<CommandKind>(byte % 6)});
    }
    const HistoryResult bytes_min = run_history<
        ca::bounded_priority_queue<Record, RecordOrder>>(11, false,
                                                          byte_commands);
    const HistoryResult bytes_max = run_history<
        ca::bounded_priority_queue<Record, RecordOrder>>(11, true,
                                                          byte_commands);
    if (!bytes_min.ok) report_history_failure(byte_commands, 11, false, bytes_min);
    if (!bytes_max.ok) report_history_failure(byte_commands, 11, true, bytes_max);
    ISO_CHECK(bytes_min.ok);
    ISO_CHECK(bytes_max.ok);
    const std::size_t corpus_command_executions =
        generated_command_count - finite_commands - seeded_command_executions;
    std::printf("bounded model: %lu finite histories/%lu commands; "
                "32 seeds x 4096 distinct commands = 131072, run in two orders "
                "(%lu executions); 256 byte commands x 2 orders (%lu executions); "
                "total %lu command executions and %lu final-drain pop probes\n",
                static_cast<unsigned long>(histories),
                static_cast<unsigned long>(finite_commands),
                static_cast<unsigned long>(seeded_command_executions),
                static_cast<unsigned long>(corpus_command_executions),
                static_cast<unsigned long>(generated_command_count),
                static_cast<unsigned long>(final_drain_probe_count));
}

} // namespace

int main() {
    // MIN heap: smallest pops first.
    ca::min_heap<int> h;
    ISO_CHECK(h.empty());
    ISO_CHECK(!h.pop().has_value());
    ISO_CHECK(!h.peek().has_value());

    for (int v : {5, 3, 8, 1, 9, 2, 7}) {
        h.push(v);
    }
    ISO_CHECK_EQ_UINT(h.size(), 7);
    ISO_CHECK_EQ_INT(h.peek().value(), 1);

    std::vector<int> drained;
    while (auto v = h.pop()) {
        drained.push_back(*v);
    }
    ISO_CHECK(drained == (std::vector<int>{1, 2, 3, 5, 7, 8, 9}));

    // MAX heap constructed via bulk heapify.
    ca::max_heap<int> m(std::vector<int>{4, 10, 6, 2, 8});
    ISO_CHECK_EQ_INT(m.peek().value(), 10);
    std::vector<int> maxdrain;
    while (auto v = m.pop()) {
        maxdrain.push_back(*v);
    }
    ISO_CHECK(maxdrain == (std::vector<int>{10, 8, 6, 4, 2}));

    // heap_sort ascending (with duplicates and negatives).
    auto sorted = ca::heap_sort(std::vector<int>{5, -3, 5, 0, 9, -3, 1, 2});
    ISO_CHECK(sorted == (std::vector<int>{-3, -3, 0, 1, 2, 5, 5, 9}));
    ISO_CHECK(ca::heap_sort(std::vector<int>{}).empty());

    // nlargest / nsmallest.
    std::vector<int> data{5, 1, 8, 3, 9, 2, 7};
    ISO_CHECK(ca::nlargest(data, 3) == (std::vector<int>{9, 8, 7}));
    ISO_CHECK(ca::nsmallest(data, 3) == (std::vector<int>{1, 2, 3}));
    ISO_CHECK(ca::nlargest(data, 0).empty());
    ISO_CHECK(ca::nsmallest(data, 0).empty());
    ISO_CHECK(ca::nlargest(data, 9) ==
              (std::vector<int>{9, 8, 7, 5, 3, 2, 1}));
    // n >= size returns everything, sorted.
    ISO_CHECK(ca::nsmallest(std::vector<int>{3, 1, 2}, 9) ==
              (std::vector<int>{1, 2, 3}));

    test_bounded_edges();
    test_range_construction();
    test_fault_controls_and_reducer();
    test_model_matrices();

    return ISO_TEST_RESULT();
}

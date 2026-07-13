// garbage_collector.hpp — a mark-and-sweep garbage collector.
// ============================================================
//
// A faithful, header-only port of the Rust `garbage-collector` crate (namespace
// `ca::garbage_collector`) — a language-agnostic tracing GC any VM can use.
//
//   1. Mark:  from the roots, follow every reference and mark reachable objects
//             (the already-marked guard makes cycles terminate).
//   2. Sweep: free every object that was not marked.
//   3. Reset: clear marks on survivors for the next cycle.
//
// Heap objects (`ConsCell`, `Symbol`, `LispClosure`) derive from `HeapObject`
// and report the heap addresses they reference. Roots are `Value`s; only
// address-like values are followed. Addresses increase monotonically from
// 0x10000 and are never reused.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef GARBAGE_COLLECTOR_HPP
#define GARBAGE_COLLECTOR_HPP

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

namespace ca::garbage_collector {

// ── Root values ──────────────────────────────────────────────────────────────

class Value;

namespace detail {
struct VInt {
    std::int64_t v;
};
struct VAddress {
    std::size_t v;
};
struct VStr {
    std::string v;
};
struct VBool {
    bool v;
};
struct VNil {};
struct VList {
    std::vector<Value> items;
};
}  // namespace detail

// A runtime value that may or may not be a heap address. The GC follows only
// Address (and Int reinterpreted as an address) and recurses into List.
class Value {
   public:
    std::variant<detail::VInt, detail::VAddress, detail::VStr, detail::VBool,
                 detail::VNil, detail::VList>
        node;

    static Value integer(std::int64_t v) { return Value{detail::VInt{v}}; }
    static Value address(std::size_t v) { return Value{detail::VAddress{v}}; }
    static Value str(std::string v) { return Value{detail::VStr{std::move(v)}}; }
    static Value boolean(bool v) { return Value{detail::VBool{v}}; }
    static Value nil() { return Value{detail::VNil{}}; }
    static Value list(std::vector<Value> items) {
        return Value{detail::VList{std::move(items)}};
    }
};

// ── Heap objects ─────────────────────────────────────────────────────────────

// Base class for anything on the managed heap. The GC calls `references()`
// during marking to find transitive references.
class HeapObject {
   public:
    virtual ~HeapObject() = default;
    virtual std::vector<std::size_t> references() const = 0;
    virtual const char* type_name() const = 0;
    bool is_marked() const { return marked_; }
    void set_marked(bool m) { marked_ = m; }

   protected:
    bool marked_ = false;
};

// A cons cell: a pair (car, cdr). Non-negative fields are heap addresses.
class ConsCell : public HeapObject {
   public:
    std::int64_t car;
    std::int64_t cdr;
    ConsCell(std::int64_t car, std::int64_t cdr) : car(car), cdr(cdr) {}
    std::vector<std::size_t> references() const override {
        std::vector<std::size_t> refs;
        if (car >= 0) refs.push_back(static_cast<std::size_t>(car));
        if (cdr >= 0) refs.push_back(static_cast<std::size_t>(cdr));
        return refs;
    }
    const char* type_name() const override { return "ConsCell"; }
};

// An interned symbol — a named atom.
class Symbol : public HeapObject {
   public:
    std::string name;
    explicit Symbol(std::string name) : name(std::move(name)) {}
    std::vector<std::size_t> references() const override { return {}; }
    const char* type_name() const override { return "Symbol"; }
};

// A closure: code + a captured environment (name → value, possibly an address).
class LispClosure : public HeapObject {
   public:
    std::string code;
    std::unordered_map<std::string, std::int64_t> env;
    std::vector<std::string> params;
    LispClosure(std::string code,
                std::unordered_map<std::string, std::int64_t> env,
                std::vector<std::string> params)
        : code(std::move(code)), env(std::move(env)), params(std::move(params)) {}
    std::vector<std::size_t> references() const override {
        std::vector<std::size_t> refs;
        for (const auto& [key, v] : env)
            if (v >= 0) refs.push_back(static_cast<std::size_t>(v));
        return refs;
    }
    const char* type_name() const override { return "LispClosure"; }
};

// ── The collector ────────────────────────────────────────────────────────────

struct GcStats {
    std::size_t total_allocations;
    std::size_t total_collections;
    std::size_t total_freed;
    std::size_t heap_size;

    friend bool operator==(const GcStats& a, const GcStats& b) {
        return a.total_allocations == b.total_allocations &&
               a.total_collections == b.total_collections &&
               a.total_freed == b.total_freed && a.heap_size == b.heap_size;
    }
};

// Abstract GC interface (VMs depend on this, never on a specific algorithm).
class GarbageCollector {
   public:
    virtual ~GarbageCollector() = default;
    virtual std::size_t allocate(std::unique_ptr<HeapObject> obj) = 0;
    virtual const HeapObject* deref(std::size_t address) const = 0;
    virtual std::size_t collect(const std::vector<Value>& roots) = 0;
    virtual std::size_t heap_size() const = 0;
    virtual GcStats stats() const = 0;
    virtual bool is_valid_address(std::size_t address) const = 0;
};

class MarkAndSweepGC : public GarbageCollector {
   public:
    std::size_t allocate(std::unique_ptr<HeapObject> obj) override {
        std::size_t address = next_address_++;
        heap_.emplace(address, std::move(obj));
        ++total_allocations_;
        return address;
    }

    const HeapObject* deref(std::size_t address) const override {
        auto it = heap_.find(address);
        return it == heap_.end() ? nullptr : it->second.get();
    }

    std::size_t collect(const std::vector<Value>& roots) override {
        ++total_collections_;
        for (const auto& r : roots) mark_value(r);

        std::vector<std::size_t> to_delete;
        for (const auto& [addr, obj] : heap_)
            if (!obj->is_marked()) to_delete.push_back(addr);
        for (std::size_t addr : to_delete) heap_.erase(addr);
        for (auto& [addr, obj] : heap_) obj->set_marked(false);

        total_freed_ += to_delete.size();
        return to_delete.size();
    }

    std::size_t heap_size() const override { return heap_.size(); }

    GcStats stats() const override {
        return GcStats{total_allocations_, total_collections_, total_freed_,
                       heap_.size()};
    }

    bool is_valid_address(std::size_t address) const override {
        return heap_.count(address) != 0;
    }

   private:
    void mark_address(std::size_t address) {
        auto it = heap_.find(address);
        if (it == heap_.end() || it->second->is_marked()) return;
        auto refs = it->second->references();
        it->second->set_marked(true);
        for (std::size_t r : refs) mark_address(r);
    }

    void mark_value(const Value& v) {
        if (const auto* a = std::get_if<detail::VAddress>(&v.node))
            mark_address(a->v);
        else if (const auto* i = std::get_if<detail::VInt>(&v.node))
            mark_address(static_cast<std::size_t>(i->v));
        else if (const auto* l = std::get_if<detail::VList>(&v.node))
            for (const auto& item : l->items) mark_value(item);
    }

    std::unordered_map<std::size_t, std::unique_ptr<HeapObject>> heap_;
    std::size_t next_address_ = 0x10000;
    std::size_t total_allocations_ = 0;
    std::size_t total_collections_ = 0;
    std::size_t total_freed_ = 0;
};

// ── Symbol table ─────────────────────────────────────────────────────────────

// Interns symbols so equal names share the same heap address (identity-based
// equality). Borrows its backing GC.
class SymbolTable {
   public:
    explicit SymbolTable(GarbageCollector& gc) : gc_(gc) {}

    std::size_t intern(const std::string& name) {
        auto it = table_.find(name);
        if (it != table_.end() && gc_.is_valid_address(it->second))
            return it->second;
        std::size_t addr = gc_.allocate(std::make_unique<Symbol>(name));
        table_[name] = addr;
        return addr;
    }

    std::optional<std::size_t> lookup(const std::string& name) const {
        auto it = table_.find(name);
        if (it != table_.end() && gc_.is_valid_address(it->second))
            return it->second;
        return std::nullopt;
    }

    std::unordered_map<std::string, std::size_t> all_symbols() const {
        std::unordered_map<std::string, std::size_t> alive;
        for (const auto& [name, addr] : table_)
            if (gc_.is_valid_address(addr)) alive.emplace(name, addr);
        return alive;
    }

   private:
    GarbageCollector& gc_;
    std::unordered_map<std::string, std::size_t> table_;
};

}  // namespace ca::garbage_collector

#endif  // GARBAGE_COLLECTOR_HPP

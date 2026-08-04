// clr_simulator.hpp — a CLR (.NET) bytecode simulator, header-only ISO C++17.
// ===========================================================================
//
// A faithful port of the Rust `clr-simulator` crate, in namespace
// `ca::clr_simulator`: a type-inferring, stack-based virtual machine for a
// subset of Microsoft's CIL (the CLR's bytecode). Unlike the JVM, the CLR
// infers operand types from the stack — one `add` opcode works for any numeric
// type.
//
// The value model is `Value`: a 32-bit integer, or an object reference (null,
// or an index into the object heap of `object[]` arrays). Stack and local slots
// are `std::optional<Value>` (an unset local is distinct from a null value).
//
// The Rust crate PANICS on malformed input; here those become C++ exceptions
// (`ca::clr_simulator::Error`, carrying an `ErrorKind`). Executing untrusted
// bytecode never reads out of bounds — every operand read and heap/array index
// is checked, since `std::vector::at` and explicit guards replace Rust's
// bounds-checked slice indexing.
//
// Pure ISO C++17: <cstdint>, <optional>, <stdexcept>, <vector>. No extensions.

#ifndef CLR_SIMULATOR_HPP
#define CLR_SIMULATOR_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace clr_simulator {

// ── Opcodes ─────────────────────────────────────────────────────────────────
enum : std::uint8_t {
    kOpNop = 0x00,
    kOpLdarg0 = 0x02,
    kOpLdarg3 = 0x05,
    kOpLdloc0 = 0x06,
    kOpLdloc3 = 0x09,
    kOpStloc0 = 0x0A,
    kOpStloc3 = 0x0D,
    kOpLdargS = 0x0E,
    kOpLdlocS = 0x11,
    kOpStlocS = 0x13,
    kOpLdnull = 0x14,
    kOpLdcI4_0 = 0x16,
    kOpLdcI4_8 = 0x1E,
    kOpLdcI4S = 0x1F,
    kOpLdcI4 = 0x20,
    kOpDup = 0x25,
    kOpCall = 0x28,
    kOpRet = 0x2A,
    kOpBrS = 0x2B,
    kOpBrfalseS = 0x2C,
    kOpBrtrueS = 0x2D,
    kOpAdd = 0x58,
    kOpSub = 0x59,
    kOpMul = 0x5A,
    kOpDiv = 0x5B,
    kOpXor = 0x61,
    kOpIsinst = 0x75,
    kOpBox = 0x8C,
    kOpNewarr = 0x8D,
    kOpLdelemRef = 0xA2,
    kOpStelemRef = 0xA4,
    kOpUnboxAny = 0xA5,
    kOpPrefixFe = 0xFE
};
enum : std::uint8_t { kCeqByte = 0x01, kCgtByte = 0x02, kCltByte = 0x04 };

// DoS guards (matching the Rust crate).
constexpr std::size_t kMaxArrayLen = std::size_t{1} << 20;
constexpr std::size_t kMaxCallDepth = 10000;

// ── Errors ──────────────────────────────────────────────────────────────────
enum class ErrorKind {
    StackUnderflow,
    ExpectedInt,
    NullOperand,
    DivideByZero,
    NullReference,
    ExpectedArray,
    IndexOutOfRange,
    UninitializedLocal,
    UninitializedArg,
    PcOutOfRange,
    BytecodeOverrun,
    Halted,
    UnknownOpcode,
    ArrayTooLarge,
    CallDepthExceeded,
    InvalidToken,
    NoMethod
};

class Error : public std::runtime_error {
  public:
    explicit Error(ErrorKind kind, const std::string& what)
        : std::runtime_error(what), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

  private:
    ErrorKind kind_;
};

// ── Value model ─────────────────────────────────────────────────────────────
//
// A stack value: either a 32-bit integer, or an object reference. A reference
// is null (`ref == std::nullopt`) or a heap index (`ref == Some(i)`).
class Value {
  public:
    static Value Int(std::int32_t i) {
        Value v;
        v.is_int_ = true;
        v.int_ = i;
        return v;
    }
    static Value Null() {
        Value v;
        v.is_int_ = false;
        v.ref_ = std::nullopt;
        return v;
    }
    static Value Ref(std::size_t idx) {
        Value v;
        v.is_int_ = false;
        v.ref_ = idx;
        return v;
    }

    bool is_int() const noexcept { return is_int_; }
    bool is_ref() const noexcept { return !is_int_; }

    // The integer, or throws ExpectedInt if this is a reference.
    std::int32_t as_int() const {
        if (!is_int_) {
            throw Error(ErrorKind::ExpectedInt,
                        "expected an integer on the stack, found a reference");
        }
        return int_;
    }

    // int as-is; ref null -> 0, Some -> 1. Used by ceq/cgt/clt.
    std::int32_t as_cmp_int() const noexcept {
        if (is_int_) {
            return int_;
        }
        return ref_.has_value() ? 1 : 0;
    }

    // int != 0, or ref is non-null.
    bool is_truthy() const noexcept {
        if (is_int_) {
            return int_ != 0;
        }
        return ref_.has_value();
    }

    const std::optional<std::size_t>& ref() const noexcept { return ref_; }

    bool operator==(const Value& o) const noexcept {
        if (is_int_ != o.is_int_) {
            return false;
        }
        return is_int_ ? int_ == o.int_ : ref_ == o.ref_;
    }
    bool operator!=(const Value& o) const noexcept { return !(*this == o); }

  private:
    bool is_int_ = true;
    std::int32_t int_ = 0;
    std::optional<std::size_t> ref_ = std::nullopt;
};

using Slot = std::optional<Value>;

// A method: its body plus frame shape.
struct Method {
    std::vector<std::uint8_t> body;
    std::size_t num_locals = 0;
    std::size_t num_args = 0;
};

// ── The simulator ───────────────────────────────────────────────────────────
class Simulator {
  public:
    Simulator() = default;

    // Load a single method (no calls), with `num_locals` locals (default 16).
    void load(const std::vector<std::uint8_t>& bytecode,
              std::size_t num_locals = 16) {
        bytecode_ = bytecode;
        pc_ = 0;
        halted_ = false;
        cur_method_ = 0;
        locals_.assign(num_locals, std::nullopt);
        stack_.clear();
        args_.clear();
        frames_.clear();
        heap_.clear();
    }

    // Load a method table and start executing at `entry`.
    void load_program(std::vector<Method> methods, std::size_t entry) {
        if (methods.empty() || entry >= methods.size()) {
            throw Error(ErrorKind::NoMethod, "entry method index out of range");
        }
        methods_ = std::move(methods);
        const Method& em = methods_[entry];
        bytecode_ = em.body;
        pc_ = 0;
        halted_ = false;
        cur_method_ = entry;
        locals_.assign(em.num_locals, std::nullopt);
        args_.assign(em.num_args, std::nullopt);
        stack_.clear();
        frames_.clear();
        heap_.clear();
    }

    // Execute one instruction. A clean `ret` from the entry method halts.
    void step();

    // Step until halted or `max_steps` reached; returns the count executed.
    std::size_t run(std::size_t max_steps) {
        std::size_t n = 0;
        while (n < max_steps && !halted_) {
            step();
            ++n;
        }
        return n;
    }

    bool halted() const noexcept { return halted_; }
    std::size_t pc() const noexcept { return pc_; }
    const std::vector<Slot>& stack() const noexcept { return stack_; }
    const std::vector<Slot>& locals() const noexcept { return locals_; }

    // Convenience accessors (return std::nullopt if out of range / unset).
    Slot stack_top() const {
        if (stack_.empty()) {
            return std::nullopt;
        }
        return stack_.back();
    }
    Slot local_at(std::size_t i) const {
        if (i >= locals_.size()) {
            return std::nullopt;
        }
        return locals_[i];
    }

    // ── Encoding helpers ──────────────────────────────────────────────────
    static std::vector<std::uint8_t> encode_ldc_i4(std::int32_t n) {
        if (n >= 0 && n <= 8) {
            return {static_cast<std::uint8_t>(kOpLdcI4_0 + n)};
        }
        if (n >= -128 && n <= 127) {
            return {kOpLdcI4S, static_cast<std::uint8_t>(n)};
        }
        auto u = static_cast<std::uint32_t>(n);
        return {kOpLdcI4, static_cast<std::uint8_t>(u & 0xFFu),
                static_cast<std::uint8_t>((u >> 8) & 0xFFu),
                static_cast<std::uint8_t>((u >> 16) & 0xFFu),
                static_cast<std::uint8_t>((u >> 24) & 0xFFu)};
    }
    static std::vector<std::uint8_t> encode_stloc(std::uint8_t slot) {
        if (slot <= 3) {
            return {static_cast<std::uint8_t>(kOpStloc0 + slot)};
        }
        return {kOpStlocS, slot};
    }
    static std::vector<std::uint8_t> encode_ldloc(std::uint8_t slot) {
        if (slot <= 3) {
            return {static_cast<std::uint8_t>(kOpLdloc0 + slot)};
        }
        return {kOpLdlocS, slot};
    }
    // Flatten a list of encoded instructions into one bytecode blob.
    static std::vector<std::uint8_t> assemble(
        const std::vector<std::vector<std::uint8_t>>& instrs) {
        std::vector<std::uint8_t> out;
        for (const auto& ins : instrs) {
            out.insert(out.end(), ins.begin(), ins.end());
        }
        return out;
    }

  private:
    // An `object[]` heap array.
    using HeapArray = std::vector<Value>;
    // A saved caller frame (operand stack + heap are shared).
    struct Frame {
        std::size_t return_pc;
        std::size_t return_method;
        std::vector<std::uint8_t> return_bytecode;
        std::vector<Slot> return_locals;
        std::vector<Slot> return_args;
    };

    // ── checked helpers ──────────────────────────────────────────────────
    Value pop() {
        if (stack_.empty()) {
            throw Error(ErrorKind::StackUnderflow, "stack underflow");
        }
        Slot s = stack_.back();
        stack_.pop_back();
        if (!s.has_value()) {
            throw Error(ErrorKind::NullOperand, "popped an uninitialized slot");
        }
        return *s;
    }
    void push(Value v) { stack_.push_back(v); }

    std::uint8_t read_u8(std::size_t off) const {
        if (pc_ >= bytecode_.size() || off > bytecode_.size() - 1 - pc_) {
            throw Error(ErrorKind::BytecodeOverrun,
                        "operand runs past end of bytecode");
        }
        return bytecode_[pc_ + off];
    }
    void need_bytes(std::size_t n) const {
        if (pc_ >= bytecode_.size() || bytecode_.size() - pc_ < n) {
            throw Error(ErrorKind::BytecodeOverrun,
                        "operand runs past end of bytecode");
        }
    }
    std::int32_t read_i32_operand() const {
        need_bytes(5);
        std::uint32_t u = static_cast<std::uint32_t>(bytecode_[pc_ + 1]) |
                          (static_cast<std::uint32_t>(bytecode_[pc_ + 2]) << 8) |
                          (static_cast<std::uint32_t>(bytecode_[pc_ + 3]) << 16) |
                          (static_cast<std::uint32_t>(bytecode_[pc_ + 4]) << 24);
        return static_cast<std::int32_t>(u);
    }

    void ldarg(std::size_t idx) {
        if (idx >= args_.size() || !args_[idx].has_value()) {
            throw Error(ErrorKind::UninitializedArg, "uninitialized argument");
        }
        push(*args_[idx]);
    }
    void ldloc(std::size_t idx) {
        if (idx >= locals_.size() || !locals_[idx].has_value()) {
            throw Error(ErrorKind::UninitializedLocal, "uninitialized local");
        }
        push(*locals_[idx]);
    }
    void stloc(std::size_t idx) {
        Value v = pop();
        if (idx >= locals_.size()) {
            locals_.resize(idx + 1, std::nullopt);
        }
        locals_[idx] = v;
    }

    void execute_binop(std::uint8_t op) {
        Value bv = pop();
        Value av = pop();
        std::uint32_t a = static_cast<std::uint32_t>(av.as_int());
        std::uint32_t b = static_cast<std::uint32_t>(bv.as_int());
        std::uint32_t r;
        switch (op) {
            case kOpAdd: r = a + b; break;
            case kOpSub: r = a - b; break;
            case kOpMul: r = a * b; break;
            case kOpXor: r = a ^ b; break;
            default:
                throw Error(ErrorKind::UnknownOpcode, "bad arithmetic opcode");
        }
        push(Value::Int(static_cast<std::int32_t>(r)));
    }
    void execute_div() {
        Value bv = pop();
        Value av = pop();
        std::int32_t a = av.as_int();
        std::int32_t b = bv.as_int();
        if (b == 0) {
            throw Error(ErrorKind::DivideByZero,
                        "System.DivideByZeroException: division by zero");
        }
        std::int32_t q;
        if (a == INT32_MIN && b == -1) {
            q = INT32_MIN;  // wrapping_div overflow case
        } else {
            q = a / b;
        }
        push(Value::Int(q));
    }
    void execute_compare(std::uint8_t sub) {
        Value bv = pop();
        Value av = pop();
        std::int32_t a = av.as_cmp_int();
        std::int32_t b = bv.as_cmp_int();
        std::int32_t r;
        switch (sub) {
            case kCeqByte: r = (a == b) ? 1 : 0; break;
            case kCgtByte: r = (a > b) ? 1 : 0; break;
            case kCltByte: r = (a < b) ? 1 : 0; break;
            default:
                throw Error(ErrorKind::UnknownOpcode, "bad compare opcode");
        }
        push(Value::Int(r));
    }

    std::vector<Slot> stack_;
    std::vector<Slot> locals_;
    std::vector<Slot> args_;
    std::vector<HeapArray> heap_;
    std::vector<std::uint8_t> bytecode_;
    std::size_t pc_ = 0;
    bool halted_ = false;
    std::vector<Method> methods_;
    std::size_t cur_method_ = 0;
    std::vector<Frame> frames_;
};

// ── step() ──────────────────────────────────────────────────────────────────
inline void Simulator::step() {
    if (halted_) {
        throw Error(ErrorKind::Halted, "CLR simulator has halted");
    }
    if (pc_ >= bytecode_.size()) {
        throw Error(ErrorKind::PcOutOfRange, "program counter past end");
    }
    const std::uint8_t op = bytecode_[pc_];

    if (op == kOpNop) {
        pc_ += 1;
    } else if (op >= kOpLdarg0 && op <= kOpLdarg3) {
        ldarg(static_cast<std::size_t>(op - kOpLdarg0));
        pc_ += 1;
    } else if (op == kOpLdargS) {
        std::uint8_t idx = read_u8(1);
        ldarg(idx);
        pc_ += 2;
    } else if (op == kOpLdnull) {
        push(Value::Null());
        pc_ += 1;
    } else if (op == kOpDup) {
        if (stack_.empty()) {
            throw Error(ErrorKind::StackUnderflow, "dup on empty stack");
        }
        if (!stack_.back().has_value()) {
            throw Error(ErrorKind::NullOperand, "dup of uninitialized slot");
        }
        push(*stack_.back());
        pc_ += 1;
    } else if (op >= kOpLdcI4_0 && op <= kOpLdcI4_8) {
        push(Value::Int(static_cast<std::int32_t>(op - kOpLdcI4_0)));
        pc_ += 1;
    } else if (op == kOpLdcI4S) {
        std::uint8_t b = read_u8(1);
        push(Value::Int(static_cast<std::int32_t>(static_cast<std::int8_t>(b))));
        pc_ += 2;
    } else if (op == kOpLdcI4) {
        std::int32_t n = read_i32_operand();
        push(Value::Int(n));
        pc_ += 5;
    } else if (op >= kOpLdloc0 && op <= kOpLdloc3) {
        ldloc(static_cast<std::size_t>(op - kOpLdloc0));
        pc_ += 1;
    } else if (op == kOpLdlocS) {
        std::uint8_t idx = read_u8(1);
        ldloc(idx);
        pc_ += 2;
    } else if (op >= kOpStloc0 && op <= kOpStloc3) {
        stloc(static_cast<std::size_t>(op - kOpStloc0));
        pc_ += 1;
    } else if (op == kOpStlocS) {
        std::uint8_t idx = read_u8(1);
        stloc(idx);
        pc_ += 2;
    } else if (op == kOpNewarr) {
        need_bytes(5);  // type token operand
        std::int32_t n = pop().as_int();
        std::size_t count = (n > 0) ? static_cast<std::size_t>(n) : 0;
        if (count > kMaxArrayLen) {
            throw Error(ErrorKind::ArrayTooLarge, "array length exceeds cap");
        }
        push(Value::Ref(heap_.size()));
        heap_.emplace_back(count, Value::Null());
        pc_ += 5;
    } else if (op == kOpStelemRef) {
        Value val = pop();
        std::int32_t idx = pop().as_int();
        Value arrv = pop();
        if (arrv.is_int()) {
            throw Error(ErrorKind::ExpectedArray, "stelem on a non-array");
        }
        if (!arrv.ref().has_value()) {
            throw Error(ErrorKind::NullReference, "stelem on null");
        }
        std::size_t h = *arrv.ref();
        if (h >= heap_.size()) {
            throw Error(ErrorKind::IndexOutOfRange, "bad heap reference");
        }
        HeapArray& arr = heap_[h];
        if (idx < 0 || static_cast<std::size_t>(idx) >= arr.size()) {
            throw Error(ErrorKind::IndexOutOfRange, "array index out of range");
        }
        arr[static_cast<std::size_t>(idx)] = val;
        pc_ += 1;
    } else if (op == kOpLdelemRef) {
        std::int32_t idx = pop().as_int();
        Value arrv = pop();
        if (arrv.is_int()) {
            throw Error(ErrorKind::ExpectedArray, "ldelem on a non-array");
        }
        if (!arrv.ref().has_value()) {
            throw Error(ErrorKind::NullReference, "ldelem on null");
        }
        std::size_t h = *arrv.ref();
        if (h >= heap_.size()) {
            throw Error(ErrorKind::IndexOutOfRange, "bad heap reference");
        }
        HeapArray& arr = heap_[h];
        if (idx < 0 || static_cast<std::size_t>(idx) >= arr.size()) {
            throw Error(ErrorKind::IndexOutOfRange, "array index out of range");
        }
        push(arr[static_cast<std::size_t>(idx)]);
        pc_ += 1;
    } else if (op == kOpBox || op == kOpUnboxAny) {
        need_bytes(5);  // type token operand; box/unbox are identity here
        if (stack_.empty()) {
            throw Error(ErrorKind::StackUnderflow, "box/unbox on empty stack");
        }
        pc_ += 5;
    } else if (op == kOpIsinst) {
        need_bytes(5);
        Value v = pop();
        if (v.is_ref() && v.ref().has_value()) {
            push(v);
        } else {
            push(Value::Null());
        }
        pc_ += 5;
    } else if (op == kOpAdd || op == kOpSub || op == kOpMul || op == kOpXor) {
        execute_binop(op);
        pc_ += 1;
    } else if (op == kOpDiv) {
        execute_div();
        pc_ += 1;
    } else if (op == kOpPrefixFe) {
        std::uint8_t sub = read_u8(1);
        execute_compare(sub);
        pc_ += 2;
    } else if (op == kOpCall) {
        std::int32_t token = read_i32_operand();
        std::uint32_t ordinal = static_cast<std::uint32_t>(token) & 0x00FFFFFFu;
        if (ordinal == 0) {
            throw Error(ErrorKind::InvalidToken, "method token 0");
        }
        std::size_t callee_idx = static_cast<std::size_t>(ordinal - 1);
        if (callee_idx >= methods_.size()) {
            throw Error(ErrorKind::InvalidToken, "method token out of range");
        }
        if (frames_.size() >= kMaxCallDepth) {
            throw Error(ErrorKind::CallDepthExceeded, "call depth exceeded");
        }
        const Method& callee = methods_[callee_idx];
        if (stack_.size() < callee.num_args) {
            throw Error(ErrorKind::StackUnderflow, "not enough args for call");
        }
        std::vector<Slot> new_args(callee.num_args, std::nullopt);
        for (std::size_t k = 0; k < callee.num_args; ++k) {
            std::size_t dst = callee.num_args - 1 - k;
            new_args[dst] = stack_.back();
            stack_.pop_back();
        }
        Frame fr;
        fr.return_pc = pc_ + 5;
        fr.return_method = cur_method_;
        fr.return_bytecode = std::move(bytecode_);
        fr.return_locals = std::move(locals_);
        fr.return_args = std::move(args_);
        frames_.push_back(std::move(fr));
        bytecode_ = callee.body;
        locals_.assign(callee.num_locals, std::nullopt);
        args_ = std::move(new_args);
        cur_method_ = callee_idx;
        pc_ = 0;
    } else if (op == kOpRet) {
        if (frames_.empty()) {
            halted_ = true;
            return;
        }
        Frame fr = std::move(frames_.back());
        frames_.pop_back();
        bytecode_ = std::move(fr.return_bytecode);
        locals_ = std::move(fr.return_locals);
        args_ = std::move(fr.return_args);
        cur_method_ = fr.return_method;
        pc_ = fr.return_pc;
    } else if (op == kOpBrS) {
        std::int32_t offset =
            static_cast<std::int32_t>(static_cast<std::int8_t>(read_u8(1)));
        std::int64_t target =
            static_cast<std::int64_t>(pc_) + 2 + offset;
        if (target < 0 ||
            static_cast<std::uint64_t>(target) > bytecode_.size()) {
            throw Error(ErrorKind::PcOutOfRange, "branch target out of range");
        }
        pc_ = static_cast<std::size_t>(target);
    } else if (op == kOpBrfalseS || op == kOpBrtrueS) {
        std::int32_t offset =
            static_cast<std::int32_t>(static_cast<std::int8_t>(read_u8(1)));
        bool truthy = pop().is_truthy();
        if ((op == kOpBrtrueS && truthy) || (op == kOpBrfalseS && !truthy)) {
            std::int64_t target =
                static_cast<std::int64_t>(pc_) + 2 + offset;
            if (target < 0 ||
                static_cast<std::uint64_t>(target) > bytecode_.size()) {
                throw Error(ErrorKind::PcOutOfRange,
                            "branch target out of range");
            }
            pc_ = static_cast<std::size_t>(target);
        } else {
            pc_ += 2;
        }
    } else {
        throw Error(ErrorKind::UnknownOpcode, "unknown opcode");
    }
}

}  // namespace clr_simulator
}  // namespace ca

#endif  // CLR_SIMULATOR_HPP

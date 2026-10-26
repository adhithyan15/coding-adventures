// Bounded JSON: syntax is a tree, ownership is a list.
// Every value is linked into its document immediately after construction. This
// separate ownership list lets error cleanup walk iteratively, even when only
// half of a deeply nested JSON tree exists. Children never own one another.
#ifndef CA_CPP_BOUNDED_JSON_HPP
#define CA_CPP_BOUNDED_JSON_HPP

#include <charconv>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <memory>
#include <memory_resource>
#include <new>
#include <stdexcept>
#include <string>
#include <string_view>
#include <system_error>
#include <utility>

namespace ca { namespace bounded_json {

inline constexpr std::size_t max_bytes = 4194304;
inline constexpr std::size_t max_depth = 128;
inline constexpr std::size_t max_nodes = 65536;
inline constexpr std::size_t max_memory = 16777216;

enum class Status { ok, argument, syntax, utf8, number, byte_limit,
                    depth_limit, node_limit, memory_limit, allocation };
enum class Kind { null, boolean, integer, floating, string, array, object };
struct Error { Status kind = Status::ok; std::size_t offset = 0; };
struct Options {
    std::size_t bytes = max_bytes, depth = max_depth, nodes = max_nodes;
    std::size_t memory = max_memory;
    // Optional allocator test seam. It must outlive the returned document and
    // obey memory_resource's alignment, allocation and nonthrowing release rules.
    std::pmr::memory_resource* upstream = std::pmr::new_delete_resource();
};

namespace detail {
struct MemoryLimit {};
class Budget final : public std::pmr::memory_resource {
    std::pmr::memory_resource* upstream_;
    std::size_t limit_, live_ = 0, peak_ = 0;
    void* do_allocate(std::size_t bytes, std::size_t alignment) override {
        // Subtraction before addition is the overflow-safe budget test. Count
        // actual allocation requests, including string/container capacity.
        if (bytes > limit_ - live_) throw MemoryLimit{};
        void* result = upstream_->allocate(bytes, alignment);
        live_ += bytes;
        if (live_ > peak_) peak_ = live_;
        return result;
    }
    void do_deallocate(void* p, std::size_t bytes, std::size_t alignment) override {
        upstream_->deallocate(p, bytes, alignment);
        live_ -= bytes;
    }
    bool do_is_equal(const std::pmr::memory_resource& other) const noexcept override {
        return this == &other;
    }
public:
    Budget(std::size_t limit, std::pmr::memory_resource* upstream)
        : upstream_(upstream), limit_(limit) {}
    std::size_t peak() const noexcept { return peak_; }
};
class Parser;
} // namespace detail

class Document;
class Value {
    friend class Document;
    friend class detail::Parser;
    Kind kind_ = Kind::null;
    bool boolean_ = false;
    std::int64_t integer_ = 0;
    double number_ = 0;
    std::pmr::string text_, key_;
    Value *owned_next_ = nullptr, *first_ = nullptr, *last_ = nullptr;
    Value* sibling_ = nullptr;
    std::size_t size_ = 0;
    explicit Value(std::pmr::memory_resource* resource) : text_(resource), key_(resource) {}
    void require(Kind expected) const {
        if (kind_ != expected) throw std::logic_error("JSON value kind mismatch");
    }
public:
    Value(const Value&) = delete;
    Value& operator=(const Value&) = delete;
    Kind kind() const noexcept { return kind_; }
    bool boolean() const { require(Kind::boolean); return boolean_; }
    std::int64_t integer() const { require(Kind::integer); return integer_; }
    double number() const {
        if (kind_ != Kind::integer && kind_ != Kind::floating)
            throw std::logic_error("JSON value is not numeric");
        return kind_ == Kind::integer ? static_cast<double>(integer_) : number_;
    }
    std::string_view string() const {
        require(Kind::string); return {text_.data(), text_.size()};
    }
    std::size_t size() const {
        if (kind_ != Kind::array && kind_ != Kind::object)
            throw std::logic_error("JSON value is not a container");
        return size_;
    }
    const Value& at(std::size_t index) const {
        require(Kind::array); return child(index);
    }
    std::pair<std::string_view, const Value*> member(std::size_t index) const {
        require(Kind::object);
        const Value& value = child(index);
        return {{value.key_.data(), value.key_.size()}, &value};
    }
private:
    const Value& child(std::size_t index) const {
        if (index >= size_) throw std::out_of_range("JSON member index");
        const Value* value = first_;
        while (index-- != 0) value = value->sibling_;
        return *value;
    }
};

class Document {
    friend class detail::Parser;
    detail::Budget budget_;
    Value* owned_ = nullptr;
    Value* root_ = nullptr;
    std::size_t nodes_ = 0;
    explicit Document(const Options& options) : budget_(options.memory, options.upstream) {}
    Value* create() {
        void* storage = budget_.allocate(sizeof(Value), alignof(Value));
        Value* value;
        try { value = new(storage) Value(&budget_); }
        catch (...) { budget_.deallocate(storage, sizeof(Value), alignof(Value)); throw; }
        value->owned_next_ = owned_;
        owned_ = value;
        ++nodes_;
        return value;
    }
public:
    Document(const Document&) = delete;
    Document& operator=(const Document&) = delete;
    // Documents themselves never move. Moving their unique_ptr preserves every
    // view; destruction invalidates all views. There are no mutable public nodes.
    ~Document() {
        while (owned_) {
            Value* next = owned_->owned_next_;
            owned_->~Value();
            budget_.deallocate(owned_, sizeof(Value), alignof(Value));
            owned_ = next;
        }
    }
    const Value& root() const noexcept { return *root_; }
    std::size_t node_count() const noexcept { return nodes_; }
    std::size_t peak_allocation_bytes() const noexcept { return budget_.peak(); }
};

struct Result {
    std::unique_ptr<Document> document;
    Error error;
    explicit operator bool() const noexcept { return document != nullptr; }
};

namespace detail {
class Parser {
    std::string_view input_;
    Options options_;
    std::size_t pos_ = 0;
    Document* doc_ = nullptr;
    [[noreturn]] void fail(Status kind) const { throw Error{kind, pos_}; }
    bool take(char c) {
        if (pos_ < input_.size() && input_[pos_] == c) { ++pos_; return true; }
        return false;
    }
    void whitespace() {
        while (pos_ < input_.size()) {
            const char c = input_[pos_];
            if (c != ' ' && c != '\t' && c != '\r' && c != '\n') break;
            ++pos_;
        }
    }
    static bool digit(char c) noexcept { return c >= '0' && c <= '9'; }
    unsigned hex4() {
        unsigned result = 0;
        for (unsigned i = 0; i < 4; ++i) {
            if (pos_ == input_.size()) fail(Status::syntax);
            const char c = input_[pos_++];
            unsigned nibble;
            if (c >= '0' && c <= '9') nibble = static_cast<unsigned>(c - '0');
            else if (c >= 'a' && c <= 'f') nibble = static_cast<unsigned>(c - 'a') + 10;
            else if (c >= 'A' && c <= 'F') nibble = static_cast<unsigned>(c - 'A') + 10;
            else fail(Status::syntax);
            result = result * 16 + nibble;
        }
        return result;
    }
    static void emit(std::pmr::string& out, unsigned point) {
        if (point < 0x80) out.push_back(static_cast<char>(point));
        else if (point < 0x800) {
            out.push_back(static_cast<char>(0xc0 | (point >> 6)));
            out.push_back(static_cast<char>(0x80 | (point & 63)));
        } else if (point < 0x10000) {
            out.push_back(static_cast<char>(0xe0 | (point >> 12)));
            out.push_back(static_cast<char>(0x80 | ((point >> 6) & 63)));
            out.push_back(static_cast<char>(0x80 | (point & 63)));
        } else {
            out.push_back(static_cast<char>(0xf0 | (point >> 18)));
            out.push_back(static_cast<char>(0x80 | ((point >> 12) & 63)));
            out.push_back(static_cast<char>(0x80 | ((point >> 6) & 63)));
            out.push_back(static_cast<char>(0x80 | (point & 63)));
        }
    }
    void raw_utf8(std::pmr::string& out) {
        const std::size_t start = pos_;
        const unsigned lead = static_cast<unsigned char>(input_[pos_++]);
        unsigned point, remaining, minimum;
        if (lead >= 0xc2 && lead <= 0xdf) { point = lead & 31; remaining = 1; minimum = 0x80; }
        else if (lead >= 0xe0 && lead <= 0xef) { point = lead & 15; remaining = 2; minimum = 0x800; }
        else if (lead >= 0xf0 && lead <= 0xf4) { point = lead & 7; remaining = 3; minimum = 0x10000; }
        else fail(Status::utf8);
        for (unsigned i = 0; i < remaining; ++i) {
            if (pos_ == input_.size()) fail(Status::utf8);
            const unsigned next = static_cast<unsigned char>(input_[pos_++]);
            if (next < 0x80 || next > 0xbf) fail(Status::utf8);
            point = (point << 6) | (next & 63);
        }
        if (point < minimum || point > 0x10ffff || (point >= 0xd800 && point <= 0xdfff))
            fail(Status::utf8);
        out.append(input_.data() + start, pos_ - start);
    }
    void string(std::pmr::string& out) {
        if (!take('"')) fail(Status::syntax);
        while (pos_ < input_.size()) {
            const unsigned c = static_cast<unsigned char>(input_[pos_]);
            if (c == '"') { ++pos_; return; }
            if (c < 32) fail(Status::syntax);
            if (c >= 128) { raw_utf8(out); continue; }
            ++pos_;
            if (c != '\\') { out.push_back(static_cast<char>(c)); continue; }
            if (pos_ == input_.size()) fail(Status::syntax);
            const char escape = input_[pos_++];
            switch (escape) {
                case '"': case '\\': case '/': out.push_back(escape); break;
                case 'b': out.push_back('\b'); break;
                case 'f': out.push_back('\f'); break;
                case 'n': out.push_back('\n'); break;
                case 'r': out.push_back('\r'); break;
                case 't': out.push_back('\t'); break;
                case 'u': {
                    unsigned point = hex4();
                    if (point >= 0xd800 && point <= 0xdbff) {
                        if (!take('\\') || !take('u')) fail(Status::utf8);
                        const unsigned low = hex4();
                        if (low < 0xdc00 || low > 0xdfff) fail(Status::utf8);
                        point = 0x10000 + ((point - 0xd800) << 10) + low - 0xdc00;
                    } else if (point >= 0xdc00 && point <= 0xdfff) fail(Status::utf8);
                    emit(out, point); break;
                }
                default: fail(Status::syntax);
            }
        }
        fail(Status::syntax);
    }
    void number(Value& value) {
        const std::size_t begin = pos_;
        take('-');
        if (!take('0')) {
            if (pos_ == input_.size() || input_[pos_] < '1' || input_[pos_] > '9') fail(Status::syntax);
            while (pos_ < input_.size() && digit(input_[pos_])) ++pos_;
        }
        bool lexical_integer = true;
        if (take('.')) {
            lexical_integer = false;
            const std::size_t fraction = pos_;
            while (pos_ < input_.size() && digit(input_[pos_])) ++pos_;
            if (pos_ == fraction) fail(Status::syntax);
        }
        if (take('e') || take('E')) {
            lexical_integer = false;
            if (!take('+')) take('-');
            const std::size_t exponent = pos_;
            while (pos_ < input_.size() && digit(input_[pos_])) ++pos_;
            if (pos_ == exponent) fail(Status::syntax);
        }
        const char* first = input_.data() + begin;
        const char* last = input_.data() + pos_;
        if (lexical_integer) {
            const auto parsed = std::from_chars(first, last, value.integer_);
            if (parsed.ec == std::errc{} && parsed.ptr == last) { value.kind_ = Kind::integer; return; }
        }
        const auto parsed = std::from_chars(first, last, value.number_, std::chars_format::general);
        const double bound = (std::numeric_limits<double>::max)();
        if (parsed.ec != std::errc{} || parsed.ptr != last ||
            !(value.number_ >= -bound && value.number_ <= bound)) fail(Status::number);
        value.kind_ = Kind::floating;
    }
    void literal(std::string_view word) {
        if (input_.substr(pos_, word.size()) != word) fail(Status::syntax);
        pos_ += word.size();
    }
    Value* value(std::size_t depth, Value* existing = nullptr) {
        whitespace();
        if (pos_ == input_.size()) fail(Status::syntax);
        Value* result = existing;
        if (!result) {
            if (doc_->nodes_ == options_.nodes) fail(Status::node_limit);
            result = doc_->create();
        }
        const char token = input_[pos_];
        if (token == '"') { result->kind_ = Kind::string; string(result->text_); }
        else if (token == 'n') literal("null");
        else if (token == 't' || token == 'f') {
            result->kind_ = Kind::boolean; result->boolean_ = token == 't';
            literal(token == 't' ? "true" : "false");
        } else if (token == '[' || token == '{') {
            if (depth == options_.depth) fail(Status::depth_limit);
            ++pos_;
            const bool object = token == '{';
            const char closing = object ? '}' : ']';
            result->kind_ = object ? Kind::object : Kind::array;
            whitespace();
            if (take(closing)) return result;
            do {
                whitespace();
                if (doc_->nodes_ == options_.nodes) fail(Status::node_limit);
                Value* child = doc_->create();
                if (object) {
                    string(child->key_); whitespace();
                    if (!take(':')) fail(Status::syntax);
                }
                value(depth + 1, child);
                if (result->last_) result->last_->sibling_ = child;
                else result->first_ = child;
                result->last_ = child; ++result->size_;
                whitespace();
                if (take(closing)) return result;
            } while (take(','));
            fail(Status::syntax);
        } else number(*result);
        return result;
    }
public:
    Parser(std::string_view input, Options options) : input_(input), options_(options) {}
    Result run() {
        if (options_.bytes > max_bytes || options_.depth > max_depth ||
            options_.nodes > max_nodes || options_.memory > max_memory || !options_.upstream)
            return {nullptr, {Status::argument, 0}};
        if (input_.size() > options_.bytes) return {nullptr, {Status::byte_limit, 0}};
        std::unique_ptr<Document> document;
        try {
            document.reset(new Document(options_));
            doc_ = document.get();
            doc_->root_ = value(0);
            whitespace();
            if (pos_ != input_.size()) fail(Status::syntax);
            return {std::move(document), {}};
        } catch (const Error& error) { return {nullptr, error}; }
        catch (const MemoryLimit&) { return {nullptr, {Status::memory_limit, pos_}}; }
        catch (const std::bad_alloc&) { return {nullptr, {Status::allocation, pos_}}; }
        catch (const std::length_error&) { return {nullptr, {Status::memory_limit, pos_}}; }
    }
};
} // namespace detail

// string_view supplies an ordinary readable-buffer precondition. No API can
// validate an arbitrary forged address; an empty default view is valid input.
inline Result parse(std::string_view input, Options options = {}) {
    return detail::Parser(input, options).run();
}
}} // namespace ca::bounded_json
#endif

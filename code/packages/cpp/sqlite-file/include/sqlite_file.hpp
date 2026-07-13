// sqlite_file.hpp — a zero-dependency reader for the SQLite on-disk format
// (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// A faithful C++ port of the Rust `sqlite-file` crate, in namespace
// `ca::sqlite_file`.  It decodes the subset of the [SQLite file format] needed
// to read table rows straight out of a database's bytes — no external SQLite
// library, no FFI, no I/O: you hand it a `std::vector<std::uint8_t>` (e.g. the
// `collection.anki2` unpacked from an Anki `.apkg`) and it walks the b-trees.
//
// [SQLite file format]: https://www.sqlite.org/fileformat2.html
//
// ## Layers (leaf-to-root, mirroring the crate)
//
//   1. varint    — the 1–9 byte big-endian base-128 integer used everywhere.
//   2. record    — decode a row's bytes into typed `Value`s.
//   3. header    — parse the 100-byte database header (page size, encoding…).
//   4. pager     — borrow page N's bytes out of the buffer (1-based, zero-copy).
//   5. btree     — walk a table/index b-tree → rows, reassembling overflow
//                  chains, guarding against cycles and amplification DoS.
//   6. schema    — resolve a table name to its root page and read it.
//
// ## Errors
//
// Every input is untrusted.  Where the Rust crate returns
// `Result<_, SqliteError>`, this port throws a `SqliteError` exception carrying
// an `Error` code; a corrupt or hostile file yields a clean throw, never an
// out-of-bounds read, panic, or unbounded loop.  `record::decode` returns a
// `std::optional` (empty on a malformed record), matching the crate's `Option`.
#ifndef CA_SQLITE_FILE_HPP
#define CA_SQLITE_FILE_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <exception>
#include <optional>
#include <set>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace ca {
namespace sqlite_file {

// ------------------------------------------------------------------ //
// Errors                                                             //
// ------------------------------------------------------------------ //

enum class Error {
    BadMagic,        // missing the 16-byte "SQLite format 3\0" magic
    Truncated,       // file shorter than the structure being read
    BadPageSize,     // page-size field not a power of two in 512..=65536
    BadPageNumber,   // page 0, or a page past the file
    Unsupported,     // a valid file using a feature we don't implement
    NoSuchTable,     // requested table name not found in sqlite_schema
    Corrupt          // internally inconsistent (bad type, cycle, cell past page…)
};

class SqliteError : public std::exception {
public:
    explicit SqliteError(Error code) noexcept : code_(code) {}
    Error code() const noexcept { return code_; }
    const char* what() const noexcept override { return "sqlite_file error"; }

private:
    Error code_;
};

// ------------------------------------------------------------------ //
// varint                                                             //
// ------------------------------------------------------------------ //

namespace varint {

// Read a varint from the front of [buf, buf+len).  Returns (value, consumed)
// where `value` is the raw two's-complement i64; empty if the buffer ends
// before the varint does.
inline std::optional<std::pair<std::int64_t, std::size_t>> read(const std::uint8_t* buf,
                                                                std::size_t len) {
    std::uint64_t result = 0;
    for (std::size_t i = 0; i < 8; ++i) {
        if (i >= len) return std::nullopt;
        std::uint8_t byte = buf[i];
        result = (result << 7) | static_cast<std::uint64_t>(byte & 0x7f);
        if ((byte & 0x80) == 0) {
            return std::make_pair(static_cast<std::int64_t>(result), i + 1);
        }
    }
    if (len < 9) return std::nullopt;
    result = (result << 8) | static_cast<std::uint64_t>(buf[8]);
    return std::make_pair(static_cast<std::int64_t>(result), static_cast<std::size_t>(9));
}

// Encode `value` (raw two's-complement i64) into its minimal varint, appending
// to `out`.  Returns the number of bytes written.
inline std::size_t write(std::int64_t value, std::vector<std::uint8_t>& out) {
    std::uint64_t v = static_cast<std::uint64_t>(value);
    if (v > 0x00ffffffffffffffULL) {
        for (int shift = 57; shift >= 8; shift -= 7) {
            out.push_back(static_cast<std::uint8_t>(0x80 | ((v >> shift) & 0x7f)));
        }
        out.push_back(static_cast<std::uint8_t>(v & 0xff));
        return 9;
    }
    std::size_t len = 1;
    int shift = 7;
    while (shift < 63 && (v >> shift) != 0) {
        ++len;
        shift += 7;
    }
    for (std::size_t i = len; i-- > 0;) {
        std::uint8_t group = static_cast<std::uint8_t>((v >> (i * 7)) & 0x7f);
        std::uint8_t cont = (i == 0) ? 0 : 0x80;
        out.push_back(static_cast<std::uint8_t>(cont | group));
    }
    return len;
}

} // namespace varint

// ------------------------------------------------------------------ //
// record                                                             //
// ------------------------------------------------------------------ //

// A single decoded column value — the five SQLite storage classes.  The
// alternatives are, in index order: Null, Int(i64), Real(f64), Text(string),
// Blob(bytes).
using Value = std::variant<std::monostate, std::int64_t, double, std::string,
                           std::vector<std::uint8_t>>;

namespace record {

inline constexpr std::size_t VNull = 0;
inline constexpr std::size_t VInt = 1;
inline constexpr std::size_t VReal = 2;
inline constexpr std::size_t VText = 3;
inline constexpr std::size_t VBlob = 4;

namespace detail {

// How many payload bytes a column of the given serial type occupies.
inline std::size_t content_size(std::uint64_t serial) {
    switch (serial) {
    case 0:
    case 8:
    case 9:
    case 10:
    case 11:
        return 0;
    case 1:
        return 1;
    case 2:
        return 2;
    case 3:
        return 3;
    case 4:
        return 4;
    case 5:
        return 6;
    case 6:
    case 7:
        return 8;
    default:
        return static_cast<std::size_t>((serial - 12) / 2);
    }
}

// Big-endian, two's-complement signed int of `bytes.size()` bytes, widened to
// i64 with sign extension.
inline std::int64_t read_int_be(const std::uint8_t* bytes, std::size_t len) {
    std::uint64_t v = 0;
    for (std::size_t i = 0; i < len; ++i) v = (v << 8) | static_cast<std::uint64_t>(bytes[i]);
    std::size_t bits = len * 8;
    if (bits < 64 && (v & (static_cast<std::uint64_t>(1) << (bits - 1))) != 0) {
        v |= ~((static_cast<std::uint64_t>(1) << bits) - 1);
    }
    return static_cast<std::int64_t>(v);
}

// Rust's String::from_utf8_lossy — copy valid UTF-8, replace each maximal
// invalid subpart with U+FFFD (EF BF BD).  For the valid UTF-8 real databases
// use, this is an identity copy.
inline std::string utf8_lossy(const std::uint8_t* s, std::size_t len) {
    static const char REPLACEMENT[] = "\xEF\xBF\xBD";
    std::string out;
    std::size_t i = 0;
    while (i < len) {
        std::uint8_t b0 = s[i];
        std::size_t seq_len;
        std::size_t lo, hi; // valid range for the first continuation byte
        if (b0 < 0x80) {
            out.push_back(static_cast<char>(b0));
            i += 1;
            continue;
        } else if (b0 >= 0xC2 && b0 <= 0xDF) {
            seq_len = 2; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xE0) {
            seq_len = 3; lo = 0xA0; hi = 0xBF;
        } else if (b0 >= 0xE1 && b0 <= 0xEC) {
            seq_len = 3; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xED) {
            seq_len = 3; lo = 0x80; hi = 0x9F;
        } else if (b0 >= 0xEE && b0 <= 0xEF) {
            seq_len = 3; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xF0) {
            seq_len = 4; lo = 0x90; hi = 0xBF;
        } else if (b0 >= 0xF1 && b0 <= 0xF3) {
            seq_len = 4; lo = 0x80; hi = 0xBF;
        } else if (b0 == 0xF4) {
            seq_len = 4; lo = 0x80; hi = 0x8F;
        } else {
            out.append(REPLACEMENT);
            i += 1;
            continue;
        }
        // Validate the continuation bytes, tracking how many were valid.
        bool ok = true;
        std::size_t consumed = 1;
        for (std::size_t j = 1; j < seq_len; ++j) {
            if (i + j >= len) { ok = false; break; }
            std::uint8_t bj = s[i + j];
            std::size_t blo = (j == 1) ? lo : 0x80;
            std::size_t bhi = (j == 1) ? hi : 0xBF;
            if (bj < blo || bj > bhi) { ok = false; break; }
            ++consumed;
        }
        if (ok) {
            for (std::size_t j = 0; j < seq_len; ++j) out.push_back(static_cast<char>(s[i + j]));
            i += seq_len;
        } else {
            out.append(REPLACEMENT);
            i += consumed; // skip the maximal valid subpart (>= 1 byte)
        }
    }
    return out;
}

inline std::optional<Value> decode_value(std::uint64_t serial, const std::uint8_t* content,
                                         std::size_t content_len) {
    switch (serial) {
    case 0:
        return Value{std::monostate{}};
    case 1:
    case 2:
    case 3:
    case 4:
    case 5:
    case 6:
        return Value{read_int_be(content, content_len)};
    case 7: {
        if (content_len != 8) return std::nullopt;
        std::uint64_t bits = 0;
        for (std::size_t i = 0; i < 8; ++i) bits = (bits << 8) | static_cast<std::uint64_t>(content[i]);
        double d;
        std::memcpy(&d, &bits, sizeof d);
        return Value{d};
    }
    case 8:
        return Value{static_cast<std::int64_t>(0)};
    case 9:
        return Value{static_cast<std::int64_t>(1)};
    case 10:
    case 11:
        return std::nullopt; // reserved — corrupt
    default:
        if (serial % 2 == 0) {
            return Value{std::vector<std::uint8_t>(content, content + content_len)};
        }
        return Value{utf8_lossy(content, content_len)};
    }
}

} // namespace detail

// Decode a complete record (header + payload) into its column values.  Empty on
// any inconsistency (header overrun, truncated payload, reserved serial type).
inline std::optional<std::vector<Value>> decode(const std::uint8_t* record, std::size_t len) {
    auto hdr = varint::read(record, len);
    if (!hdr) return std::nullopt;
    std::int64_t header_len_raw = hdr->first;
    std::size_t header_off = hdr->second;
    if (header_len_raw < 0) return std::nullopt;
    std::size_t header_len = static_cast<std::size_t>(header_len_raw);
    if (header_len > len) return std::nullopt;

    std::vector<Value> values;
    std::size_t payload_off = header_len;
    while (header_off < header_len) {
        auto st = varint::read(record + header_off, len - header_off);
        if (!st) return std::nullopt;
        header_off += st->second;
        if (st->first < 0) return std::nullopt;
        std::uint64_t serial = static_cast<std::uint64_t>(st->first);
        std::size_t size = detail::content_size(serial);
        if (size > len - payload_off) return std::nullopt; // overflow-safe bound
        auto v = detail::decode_value(serial, record + payload_off, size);
        if (!v) return std::nullopt;
        payload_off += size;
        values.push_back(std::move(*v));
    }
    return values;
}

inline std::optional<std::vector<Value>> decode(const std::vector<std::uint8_t>& record) {
    return decode(record.data(), record.size());
}

} // namespace record

// ------------------------------------------------------------------ //
// header                                                             //
// ------------------------------------------------------------------ //

enum class TextEncoding { Utf8, Utf16Le, Utf16Be };

struct Header {
    std::uint32_t page_size = 0;
    std::uint8_t reserved_space = 0;
    std::uint32_t page_count = 0;
    std::uint32_t change_counter = 0;
    std::uint32_t freelist_trunk = 0;
    std::uint32_t freelist_count = 0;
    std::uint32_t schema_cookie = 0;
    std::uint32_t schema_format = 0;
    TextEncoding text_encoding = TextEncoding::Utf8;

    std::uint32_t usable_size() const {
        return page_size - static_cast<std::uint32_t>(reserved_space);
    }
};

namespace detail {

inline const char MAGIC[16] = {'S', 'Q', 'L', 'i', 't', 'e', ' ', 'f',
                               'o', 'r', 'm', 'a', 't', ' ', '3', '\0'};

inline std::uint32_t be_u32(const std::uint8_t* buf, std::size_t off) {
    return (static_cast<std::uint32_t>(buf[off]) << 24) |
           (static_cast<std::uint32_t>(buf[off + 1]) << 16) |
           (static_cast<std::uint32_t>(buf[off + 2]) << 8) |
           static_cast<std::uint32_t>(buf[off + 3]);
}

inline bool is_power_of_two(std::uint32_t x) { return x != 0 && (x & (x - 1)) == 0; }

} // namespace detail

inline Header parse_header(const std::uint8_t* buf, std::size_t len) {
    if (len < 100) throw SqliteError(Error::Truncated);
    if (std::memcmp(buf, detail::MAGIC, 16) != 0) throw SqliteError(Error::BadMagic);

    std::uint16_t raw = static_cast<std::uint16_t>((static_cast<std::uint16_t>(buf[16]) << 8) |
                                                   static_cast<std::uint16_t>(buf[17]));
    std::uint32_t page_size = (raw == 1) ? 65536u : static_cast<std::uint32_t>(raw);
    if (page_size < 512 || !detail::is_power_of_two(page_size)) throw SqliteError(Error::BadPageSize);

    std::uint8_t reserved = buf[20];
    if (static_cast<std::uint32_t>(reserved) >= page_size) throw SqliteError(Error::BadPageSize);

    TextEncoding enc;
    switch (detail::be_u32(buf, 56)) {
    case 1: enc = TextEncoding::Utf8; break;
    case 2: enc = TextEncoding::Utf16Le; break;
    case 3: enc = TextEncoding::Utf16Be; break;
    default: throw SqliteError(Error::Unsupported);
    }

    Header h;
    h.page_size = page_size;
    h.reserved_space = reserved;
    h.page_count = detail::be_u32(buf, 28);
    h.change_counter = detail::be_u32(buf, 24);
    h.freelist_trunk = detail::be_u32(buf, 32);
    h.freelist_count = detail::be_u32(buf, 36);
    h.schema_cookie = detail::be_u32(buf, 40);
    h.schema_format = detail::be_u32(buf, 44);
    h.text_encoding = enc;
    return h;
}

inline Header parse_header(const std::vector<std::uint8_t>& buf) {
    return parse_header(buf.data(), buf.size());
}

// ------------------------------------------------------------------ //
// pager                                                              //
// ------------------------------------------------------------------ //

// A read-only, zero-copy view over a database's bytes.
class Pager {
public:
    Pager(const std::uint8_t* data, std::size_t len, std::size_t page_size)
        : data_(data), len_(len), page_size_(page_size) {}

    // Parse the header and build a pager in one step.
    static std::pair<Header, Pager> open(const std::uint8_t* data, std::size_t len) {
        Header h = parse_header(data, len);
        return {h, Pager(data, len, h.page_size)};
    }
    static std::pair<Header, Pager> open(const std::vector<std::uint8_t>& data) {
        return open(data.data(), data.size());
    }

    // Borrow page `page_no` (1-based); throws BadPageNumber for page 0 or a page
    // past end-of-file.  Returns (ptr, page_size).
    std::pair<const std::uint8_t*, std::size_t> page(std::uint32_t page_no) const {
        if (page_no == 0) throw SqliteError(Error::BadPageNumber);
        std::size_t index = static_cast<std::size_t>(page_no - 1);
        if (page_size_ != 0 && index > (static_cast<std::size_t>(-1)) / page_size_) {
            throw SqliteError(Error::BadPageNumber);
        }
        std::size_t start = index * page_size_;
        if (start > static_cast<std::size_t>(-1) - page_size_) throw SqliteError(Error::BadPageNumber);
        std::size_t end = start + page_size_;
        if (end > len_) throw SqliteError(Error::BadPageNumber);
        return {data_ + start, page_size_};
    }

    std::size_t page_size() const { return page_size_; }
    std::size_t page_count() const { return page_size_ == 0 ? 0 : len_ / page_size_; }

private:
    const std::uint8_t* data_;
    std::size_t len_;
    std::size_t page_size_;
};

// ------------------------------------------------------------------ //
// btree                                                              //
// ------------------------------------------------------------------ //

namespace detail {

inline constexpr std::uint8_t LEAF_TABLE = 0x0D;
inline constexpr std::uint8_t INTERIOR_TABLE = 0x05;
inline constexpr std::uint8_t LEAF_INDEX = 0x0A;
inline constexpr std::uint8_t INTERIOR_INDEX = 0x02;

inline std::optional<std::uint16_t> page_be_u16(const std::uint8_t* page, std::size_t page_len,
                                                std::size_t off) {
    if (off + 1 >= page_len) return std::nullopt;
    return static_cast<std::uint16_t>((static_cast<std::uint16_t>(page[off]) << 8) |
                                      static_cast<std::uint16_t>(page[off + 1]));
}

inline std::optional<std::uint32_t> page_be_u32(const std::uint8_t* page, std::size_t page_len,
                                                std::size_t off) {
    if (off > page_len || page_len - off < 4) return std::nullopt;
    return (static_cast<std::uint32_t>(page[off]) << 24) |
           (static_cast<std::uint32_t>(page[off + 1]) << 16) |
           (static_cast<std::uint32_t>(page[off + 2]) << 8) |
           static_cast<std::uint32_t>(page[off + 3]);
}

inline std::size_t cell_pointer(const std::uint8_t* page, std::size_t page_len,
                                std::size_t ptr_array, std::size_t i) {
    // ptr_array + i*2, overflow-checked.
    if (i > (static_cast<std::size_t>(-1) - ptr_array) / 2) throw SqliteError(Error::Corrupt);
    std::size_t entry = ptr_array + i * 2;
    auto off = page_be_u16(page, page_len, entry);
    if (!off) throw SqliteError(Error::Corrupt);
    return static_cast<std::size_t>(*off);
}

inline std::size_t index_max_local(std::size_t usable) {
    std::size_t a = usable > 12 ? usable - 12 : 0;
    std::size_t v = (a * 64) / 255;
    return v > 23 ? v - 23 : 0;
}

// Walk the overflow-page chain, appending onto `record` until it holds
// `payload_len` bytes.
inline void follow_overflow(const Pager& pager, std::uint32_t first_page, std::size_t payload_len,
                            std::size_t usable, std::size_t file_bytes,
                            std::vector<std::uint8_t>& record) {
    std::uint32_t next = first_page;
    std::set<std::uint32_t> visited;
    while (record.size() < payload_len) {
        if (next == 0) throw SqliteError(Error::Corrupt);
        if (!visited.insert(next).second) throw SqliteError(Error::Corrupt);
        auto pg = pager.page(next);
        const std::uint8_t* page = pg.first;
        std::size_t page_len = pg.second;
        auto next_ptr = page_be_u32(page, page_len, 0);
        if (!next_ptr) throw SqliteError(Error::Corrupt);
        if (usable > page_len || 4 > usable) throw SqliteError(Error::Corrupt);
        std::size_t content_len = usable - 4;
        std::size_t still_needed = payload_len - record.size();
        std::size_t take = still_needed < content_len ? still_needed : content_len;
        record.insert(record.end(), page + 4, page + 4 + take);
        if (record.size() > file_bytes) throw SqliteError(Error::Corrupt);
        next = *next_ptr;
    }
}

// Inline split + overflow reassembly for one leaf cell payload.
inline std::vector<std::uint8_t> split_and_reassemble(const Pager& pager,
                                                      const std::uint8_t* payload,
                                                      std::size_t payload_avail,
                                                      std::size_t payload_len, std::size_t usable,
                                                      std::size_t max_local,
                                                      std::size_t file_bytes) {
    if (payload_len <= max_local) {
        if (payload_len > payload_avail) throw SqliteError(Error::Corrupt);
        return std::vector<std::uint8_t>(payload, payload + payload_len);
    }
    if (payload_len > file_bytes) throw SqliteError(Error::Corrupt);

    std::size_t a = usable > 12 ? usable - 12 : 0;
    std::size_t m = (a * 32) / 255;
    std::size_t min_local = m > 23 ? m - 23 : 0;
    if (usable < 4 || usable - 4 == 0) throw SqliteError(Error::Corrupt);
    std::size_t span = usable - 4;
    std::size_t k = min_local + (payload_len - min_local) % span;
    std::size_t inline_len = (k <= max_local) ? k : min_local;

    // inline bytes then the 4-byte first-overflow pointer must be on this page.
    if (inline_len > payload_avail || payload_avail - inline_len < 4) throw SqliteError(Error::Corrupt);
    std::uint32_t first_overflow = (static_cast<std::uint32_t>(payload[inline_len]) << 24) |
                                   (static_cast<std::uint32_t>(payload[inline_len + 1]) << 16) |
                                   (static_cast<std::uint32_t>(payload[inline_len + 2]) << 8) |
                                   static_cast<std::uint32_t>(payload[inline_len + 3]);
    std::vector<std::uint8_t> record;
    record.reserve(payload_len);
    record.insert(record.end(), payload, payload + inline_len);
    follow_overflow(pager, first_overflow, payload_len, usable, file_bytes, record);
    return record;
}

} // namespace detail

// Walk the table b-tree rooted at `root_page`, returning every (rowid, record
// bytes) in rowid order.
inline std::vector<std::pair<std::int64_t, std::vector<std::uint8_t>>> walk_table(
    const Pager& pager, const Header& header, std::uint32_t root_page) {
    using namespace detail;
    std::size_t usable = header.usable_size();
    std::size_t max_local = usable > 35 ? usable - 35 : 0;
    std::size_t file_bytes = pager.page_count() * pager.page_size();
    std::size_t emitted = 0;

    std::vector<std::pair<std::int64_t, std::vector<std::uint8_t>>> rows;
    std::vector<std::uint32_t> stack{root_page};
    std::set<std::uint32_t> visited;

    while (!stack.empty()) {
        std::uint32_t page_no = stack.back();
        stack.pop_back();
        if (!visited.insert(page_no).second) throw SqliteError(Error::Corrupt);
        auto pg = pager.page(page_no);
        const std::uint8_t* page = pg.first;
        std::size_t page_len = pg.second;
        std::size_t header_off = (page_no == 1) ? 100 : 0;

        if (header_off >= page_len) throw SqliteError(Error::Truncated);
        std::uint8_t page_type = page[header_off];
        auto cc = page_be_u16(page, page_len, header_off + 3);
        if (!cc) throw SqliteError(Error::Truncated);
        std::size_t cell_count = *cc;

        if (page_type == LEAF_TABLE) {
            std::size_t ptr_array = header_off + 8;
            for (std::size_t i = 0; i < cell_count; ++i) {
                std::size_t cell_off = cell_pointer(page, page_len, ptr_array, i);
                if (cell_off > page_len) throw SqliteError(Error::Corrupt);
                const std::uint8_t* cell = page + cell_off;
                std::size_t cell_avail = page_len - cell_off;
                auto pl = varint::read(cell, cell_avail);
                if (!pl) throw SqliteError(Error::Corrupt);
                if (pl->first < 0) throw SqliteError(Error::Corrupt);
                std::size_t payload_len = static_cast<std::size_t>(pl->first);
                std::size_t n1 = pl->second;
                auto rid = varint::read(cell + n1, cell_avail - n1);
                if (!rid) throw SqliteError(Error::Corrupt);
                std::int64_t rowid = rid->first;
                std::size_t n2 = rid->second;
                std::size_t poff = n1 + n2;
                auto record = split_and_reassemble(pager, cell + poff, cell_avail - poff,
                                                   payload_len, usable, max_local, file_bytes);
                emitted += record.size();
                if (emitted > file_bytes) throw SqliteError(Error::Corrupt);
                rows.emplace_back(rowid, std::move(record));
            }
        } else if (page_type == INTERIOR_TABLE) {
            std::size_t ptr_array = header_off + 12;
            for (std::size_t i = 0; i < cell_count; ++i) {
                std::size_t cell_off = cell_pointer(page, page_len, ptr_array, i);
                auto child = page_be_u32(page, page_len, cell_off);
                if (!child) throw SqliteError(Error::Corrupt);
                stack.push_back(*child);
            }
            auto rightmost = page_be_u32(page, page_len, header_off + 8);
            if (!rightmost) throw SqliteError(Error::Truncated);
            stack.push_back(*rightmost);
        } else {
            throw SqliteError(Error::Corrupt);
        }
    }

    std::sort(rows.begin(), rows.end(),
              [](const auto& a, const auto& b) { return a.first < b.first; });
    return rows;
}

// Walk the index b-tree rooted at `root_page`, returning every entry's record
// bytes (used for indexes and WITHOUT ROWID tables).
inline std::vector<std::vector<std::uint8_t>> walk_index(const Pager& pager, const Header& header,
                                                         std::uint32_t root_page) {
    using namespace detail;
    std::size_t usable = header.usable_size();
    std::size_t max_local = index_max_local(usable);
    std::size_t file_bytes = pager.page_count() * pager.page_size();
    std::size_t emitted = 0;

    std::vector<std::vector<std::uint8_t>> records;
    std::vector<std::uint32_t> stack{root_page};
    std::set<std::uint32_t> visited;

    while (!stack.empty()) {
        std::uint32_t page_no = stack.back();
        stack.pop_back();
        if (!visited.insert(page_no).second) throw SqliteError(Error::Corrupt);
        auto pg = pager.page(page_no);
        const std::uint8_t* page = pg.first;
        std::size_t page_len = pg.second;
        std::size_t header_off = (page_no == 1) ? 100 : 0;

        if (header_off >= page_len) throw SqliteError(Error::Truncated);
        std::uint8_t page_type = page[header_off];
        auto cc = page_be_u16(page, page_len, header_off + 3);
        if (!cc) throw SqliteError(Error::Truncated);
        std::size_t cell_count = *cc;

        std::size_t ptr_array;
        std::size_t payload_skip;
        if (page_type == LEAF_INDEX) {
            ptr_array = header_off + 8;
            payload_skip = 0;
        } else if (page_type == INTERIOR_INDEX) {
            ptr_array = header_off + 12;
            payload_skip = 4;
        } else {
            throw SqliteError(Error::Corrupt);
        }

        for (std::size_t i = 0; i < cell_count; ++i) {
            std::size_t cell_off = cell_pointer(page, page_len, ptr_array, i);
            if (payload_skip == 4) {
                auto child = page_be_u32(page, page_len, cell_off);
                if (!child) throw SqliteError(Error::Corrupt);
                stack.push_back(*child);
            }
            if (cell_off > page_len || page_len - cell_off < payload_skip) throw SqliteError(Error::Corrupt);
            const std::uint8_t* payload = page + cell_off + payload_skip;
            std::size_t avail = page_len - cell_off - payload_skip;
            auto pl = varint::read(payload, avail);
            if (!pl) throw SqliteError(Error::Corrupt);
            if (pl->first < 0) throw SqliteError(Error::Corrupt);
            std::size_t payload_len = static_cast<std::size_t>(pl->first);
            std::size_t n1 = pl->second;
            auto record = split_and_reassemble(pager, payload + n1, avail - n1, payload_len, usable,
                                               max_local, file_bytes);
            emitted += record.size();
            if (emitted > file_bytes) throw SqliteError(Error::Corrupt);
            records.push_back(std::move(record));
        }

        if (page_type == INTERIOR_INDEX) {
            auto rightmost = page_be_u32(page, page_len, header_off + 8);
            if (!rightmost) throw SqliteError(Error::Truncated);
            stack.push_back(*rightmost);
        }
    }
    return records;
}

// ------------------------------------------------------------------ //
// schema                                                             //
// ------------------------------------------------------------------ //

struct SchemaEntry {
    std::string object_type;
    std::string name;
    std::string table_name;
    std::optional<std::uint32_t> root_page;
    std::optional<std::string> sql;

    bool operator==(const SchemaEntry& o) const {
        return object_type == o.object_type && name == o.name && table_name == o.table_name &&
               root_page == o.root_page && sql == o.sql;
    }
};

namespace detail {

inline std::string expect_text(const Value& v) {
    if (v.index() != record::VText) throw SqliteError(Error::Corrupt);
    return std::get<std::string>(v);
}

inline SchemaEntry decode_schema_row(const std::vector<std::uint8_t>& rec) {
    auto cols = record::decode(rec);
    if (!cols) throw SqliteError(Error::Corrupt);
    if (cols->size() != 5) throw SqliteError(Error::Corrupt);
    const std::vector<Value>& c = *cols;

    SchemaEntry e;
    e.object_type = expect_text(c[0]);
    e.name = expect_text(c[1]);
    e.table_name = expect_text(c[2]);
    if (c[3].index() == record::VNull) {
        e.root_page = std::nullopt;
    } else if (c[3].index() == record::VInt) {
        std::int64_t n = std::get<std::int64_t>(c[3]);
        if (n == 0) {
            e.root_page = std::nullopt;
        } else if (n < 0 || n > 0xffffffffLL) {
            throw SqliteError(Error::Corrupt);
        } else {
            e.root_page = static_cast<std::uint32_t>(n);
        }
    } else {
        throw SqliteError(Error::Corrupt);
    }
    if (c[4].index() == record::VNull) {
        e.sql = std::nullopt;
    } else if (c[4].index() == record::VText) {
        e.sql = std::get<std::string>(c[4]);
    } else {
        throw SqliteError(Error::Corrupt);
    }
    return e;
}

inline std::vector<SchemaEntry> read_schema_from(const Pager& pager, const Header& header) {
    auto rows = walk_table(pager, header, 1);
    std::vector<SchemaEntry> out;
    out.reserve(rows.size());
    for (auto& r : rows) out.push_back(decode_schema_row(r.second));
    return out;
}

inline std::uint32_t table_root_page_from(const Pager& pager, const Header& header,
                                          const std::string& name) {
    for (const SchemaEntry& e : read_schema_from(pager, header)) {
        if (e.object_type == "table" && e.name == name) {
            if (!e.root_page) throw SqliteError(Error::Corrupt);
            return *e.root_page;
        }
    }
    throw SqliteError(Error::NoSuchTable);
}

} // namespace detail

inline std::vector<SchemaEntry> read_schema(const std::vector<std::uint8_t>& data) {
    auto ho = Pager::open(data);
    return detail::read_schema_from(ho.second, ho.first);
}

inline std::uint32_t table_root_page(const std::vector<std::uint8_t>& data, const std::string& name) {
    auto ho = Pager::open(data);
    return detail::table_root_page_from(ho.second, ho.first, name);
}

// Read a table by name → (rowid, decoded columns) in rowid order.
inline std::vector<std::pair<std::int64_t, std::vector<Value>>> read_table(
    const std::vector<std::uint8_t>& data, const std::string& name) {
    auto ho = Pager::open(data);
    std::uint32_t root = detail::table_root_page_from(ho.second, ho.first, name);
    auto rows = walk_table(ho.second, ho.first, root);
    std::vector<std::pair<std::int64_t, std::vector<Value>>> out;
    out.reserve(rows.size());
    for (auto& r : rows) {
        auto cols = record::decode(r.second);
        if (!cols) throw SqliteError(Error::Corrupt);
        out.emplace_back(r.first, std::move(*cols));
    }
    return out;
}

// Read a WITHOUT ROWID table by name → each row's decoded columns.
inline std::vector<std::vector<Value>> read_without_rowid_table(
    const std::vector<std::uint8_t>& data, const std::string& name) {
    auto ho = Pager::open(data);
    std::uint32_t root = detail::table_root_page_from(ho.second, ho.first, name);
    auto recs = walk_index(ho.second, ho.first, root);
    std::vector<std::vector<Value>> out;
    out.reserve(recs.size());
    for (auto& rec : recs) {
        auto cols = record::decode(rec);
        if (!cols) throw SqliteError(Error::Corrupt);
        out.push_back(std::move(*cols));
    }
    return out;
}

} // namespace sqlite_file
} // namespace ca

#endif // CA_SQLITE_FILE_HPP

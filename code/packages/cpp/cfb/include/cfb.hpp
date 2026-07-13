// cfb.hpp — OLE2 / Compound File Binary Format reader, header-only C++17.
// ============================================================================
//
// A faithful port of the Rust `cfb` crate, in namespace `ca::cfb`: a
// from-scratch reader for the Microsoft Compound File Binary Format ([MS-CFB]),
// the container inside legacy `.xls`/`.doc`/`.ppt` files. It is the read
// counterpart to the ported `cfb-writer`.
//
// A CFB file is a FAT filesystem in one file: fixed-size sectors chained by a
// File Allocation Table, a directory stream naming streams (files) and storages
// (folders), and a mini-stream packing tiny streams. CFB files arrive as
// attachments, so every chain walk is cycle-guarded, every offset bounds-checked
// with overflow-safe arithmetic, and output capped at 256 MiB.
//
// Where the Rust `open` returns `Result`, this port throws `CfbError`. Pure ISO
// C++17.

#ifndef CFB_HPP
#define CFB_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace cfb {

enum class CfbError {
    BadSignature,
    Truncated,
    UnsupportedSectorSize,
    BadSectorChain,
    CycleDetected,
    OutputTooLarge,
    BadDirectory,
    NotAStream,
};

inline const char* to_string(CfbError e) {
    switch (e) {
    case CfbError::BadSignature:
        return "not a Compound File (bad signature)";
    case CfbError::Truncated:
        return "input truncated";
    case CfbError::UnsupportedSectorSize:
        return "unsupported sector shift";
    case CfbError::BadSectorChain:
        return "sector chain out of bounds";
    case CfbError::CycleDetected:
        return "cycle detected in sector chain";
    case CfbError::OutputTooLarge:
        return "assembled output exceeds safety cap";
    case CfbError::BadDirectory:
        return "malformed directory";
    case CfbError::NotAStream:
        return "directory entry is not a stream";
    }
    return "unknown error";
}

enum class EntryKind { Stream, Storage, RootStorage };

struct Entry {
    std::string name;
    std::uint64_t size = 0;
    EntryKind kind = EntryKind::Stream;
    std::uint32_t id = 0;
};

namespace detail {
constexpr std::uint32_t FREESECT = 0xFFFFFFFFu;
constexpr std::uint32_t ENDOFCHAIN = 0xFFFFFFFEu;
constexpr std::uint32_t FATSECT = 0xFFFFFFFDu;
constexpr std::uint32_t DIFSECT = 0xFFFFFFFCu;
constexpr std::uint32_t NOSTREAM = 0xFFFFFFFFu;
constexpr std::size_t HEADER_LEN = 512;
constexpr std::size_t DIR_ENTRY_SIZE = 128;
constexpr std::size_t HEADER_DIFAT_COUNT = 109;
constexpr std::size_t HEADER_DIFAT_OFFSET = 76;
constexpr std::uint64_t MAX_OUTPUT = std::uint64_t(256) * 1024 * 1024;

inline std::optional<std::uint16_t> rd_u16(const std::vector<std::uint8_t>& b,
                                           std::size_t off) {
    if (off > b.size() || b.size() - off < 2) {
        return std::nullopt;
    }
    return static_cast<std::uint16_t>(b[off] |
                                      (static_cast<std::uint16_t>(b[off + 1])
                                       << 8));
}
inline std::optional<std::uint32_t> rd_u32(const std::vector<std::uint8_t>& b,
                                           std::size_t off) {
    if (off > b.size() || b.size() - off < 4) {
        return std::nullopt;
    }
    return static_cast<std::uint32_t>(b[off]) |
           (static_cast<std::uint32_t>(b[off + 1]) << 8) |
           (static_cast<std::uint32_t>(b[off + 2]) << 16) |
           (static_cast<std::uint32_t>(b[off + 3]) << 24);
}
inline std::optional<std::uint64_t> rd_u64(const std::vector<std::uint8_t>& b,
                                           std::size_t off) {
    auto lo = rd_u32(b, off);
    auto hi = rd_u32(b, off + 4);
    if (!lo || !hi) {
        return std::nullopt;
    }
    return static_cast<std::uint64_t>(*lo) |
           (static_cast<std::uint64_t>(*hi) << 32);
}

inline void utf8_put(std::string& out, std::uint32_t cp) {
    if (cp < 0x80) {
        out.push_back(static_cast<char>(cp));
    } else if (cp < 0x800) {
        out.push_back(static_cast<char>(0xC0 | (cp >> 6)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else if (cp < 0x10000) {
        out.push_back(static_cast<char>(0xE0 | (cp >> 12)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    } else {
        out.push_back(static_cast<char>(0xF0 | (cp >> 18)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 12) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | ((cp >> 6) & 0x3F)));
        out.push_back(static_cast<char>(0x80 | (cp & 0x3F)));
    }
}

// Decode a UTF-16LE directory name from the 64-byte name field. `name_len` is
// the byte length including the 2-byte NUL terminator. Lossy.
inline std::string decode_utf16le_name(const std::vector<std::uint8_t>& b,
                                       std::size_t field_off,
                                       std::size_t name_len) {
    std::string out;
    std::size_t usable = std::min<std::size_t>(name_len, 64);
    std::size_t chars = usable >= 2 ? usable - 2 : 0;
    std::size_t i = 0;
    while (i + 2 <= 64 && i < chars) {
        std::uint32_t u = b[field_off + i] |
                          (static_cast<std::uint32_t>(b[field_off + i + 1]) << 8);
        i += 2;
        if (u >= 0xD800 && u <= 0xDBFF) {
            if (i + 2 <= 64 && i < chars) {
                std::uint32_t lo =
                    b[field_off + i] |
                    (static_cast<std::uint32_t>(b[field_off + i + 1]) << 8);
                if (lo >= 0xDC00 && lo <= 0xDFFF) {
                    i += 2;
                    utf8_put(out,
                             0x10000 + ((u - 0xD800) << 10) + (lo - 0xDC00));
                    continue;
                }
            }
            utf8_put(out, 0xFFFD);
        } else if (u >= 0xDC00 && u <= 0xDFFF) {
            utf8_put(out, 0xFFFD);
        } else {
            utf8_put(out, u);
        }
    }
    return out;
}

inline bool ascii_ci_eq(const std::string& a, const std::string& b) {
    if (a.size() != b.size()) {
        return false;
    }
    for (std::size_t i = 0; i < a.size(); ++i) {
        unsigned char ca = static_cast<unsigned char>(a[i]);
        unsigned char cb = static_cast<unsigned char>(b[i]);
        if (ca >= 'A' && ca <= 'Z') ca = static_cast<unsigned char>(ca + 32);
        if (cb >= 'A' && cb <= 'Z') cb = static_cast<unsigned char>(cb + 32);
        if (ca != cb) {
            return false;
        }
    }
    return true;
}

struct DirEntry {
    std::string name;
    std::uint8_t object_type = 0;
    std::uint32_t left = 0, right = 0, child = 0, start_sector = 0;
    std::uint64_t size = 0;
};
}  // namespace detail

class CompoundFile {
  public:
    // Parse a CFB file. Throws CfbError on failure.
    static CompoundFile open(const std::vector<std::uint8_t>& bytes) {
        using namespace detail;
        CompoundFile cf;
        if (bytes.size() < HEADER_LEN) {
            throw CfbError::Truncated;
        }
        static const std::uint8_t sig[8] = {0xD0, 0xCF, 0x11, 0xE0,
                                            0xA1, 0xB1, 0x1A, 0xE1};
        if (std::memcmp(bytes.data(), sig, 8) != 0) {
            throw CfbError::BadSignature;
        }
        auto sector_shift = rd_u16(bytes, 30);
        if (!sector_shift) {
            throw CfbError::Truncated;
        }
        if (*sector_shift == 0x0009) {
            cf.sector_size_ = 512;
        } else if (*sector_shift == 0x000C) {
            cf.sector_size_ = 4096;
        } else {
            throw CfbError::UnsupportedSectorSize;
        }
        auto mini_shift = rd_u16(bytes, 32);
        if (!mini_shift) {
            throw CfbError::Truncated;
        }
        if (*mini_shift != 0x0006) {
            throw CfbError::UnsupportedSectorSize;
        }
        cf.mini_sector_size_ = std::size_t(1) << *mini_shift;

        auto num_fat = rd_u32(bytes, 44);
        auto first_dir = rd_u32(bytes, 48);
        auto cutoff = rd_u32(bytes, 56);
        auto first_minifat = rd_u32(bytes, 60);
        auto num_minifat = rd_u32(bytes, 64);
        auto first_difat = rd_u32(bytes, 68);
        auto num_difat = rd_u32(bytes, 72);
        if (!num_fat || !first_dir || !cutoff || !first_minifat ||
            !num_minifat || !first_difat || !num_difat) {
            throw CfbError::Truncated;
        }
        cf.mini_cutoff_ = *cutoff;
        cf.data_ = bytes;
        std::size_t total_sectors = (bytes.size() - HEADER_LEN) / cf.sector_size_;

        auto fat_ids =
            cf.collect_difat(*first_difat, *num_difat, *num_fat, total_sectors);
        cf.assemble_fat(fat_ids, total_sectors);

        if (*first_minifat != ENDOFCHAIN && *num_minifat != 0) {
            auto raw = cf.read_fat_chain(*first_minifat, std::nullopt);
            for (std::size_t k = 0; k + 4 <= raw.size(); k += 4) {
                cf.mini_fat_.push_back(static_cast<std::uint32_t>(raw[k]) |
                                       (static_cast<std::uint32_t>(raw[k + 1])
                                        << 8) |
                                       (static_cast<std::uint32_t>(raw[k + 2])
                                        << 16) |
                                       (static_cast<std::uint32_t>(raw[k + 3])
                                        << 24));
            }
        }

        cf.read_directory(*first_dir);
        if (cf.dir_.empty()) {
            throw CfbError::BadDirectory;
        }
        if (cf.dir_[0].object_type != 5) {
            throw CfbError::BadDirectory;
        }
        if (cf.dir_[0].size != 0) {
            auto ms = cf.read_fat_chain(cf.dir_[0].start_sector, std::nullopt);
            std::uint64_t want = cf.dir_[0].size;
            if (want > ms.size()) {
                throw CfbError::BadDirectory;
            }
            ms.resize(static_cast<std::size_t>(want));
            cf.mini_stream_ = std::move(ms);
        }

        cf.enumerate_entries();
        return cf;
    }

    std::size_t sector_size() const { return sector_size_; }
    const std::vector<Entry>& entries() const { return entries_; }

    std::vector<std::string> stream_names() const {
        std::vector<std::string> names;
        for (const auto& e : entries_) {
            if (e.kind == EntryKind::Stream) {
                names.push_back(e.name);
            }
        }
        return names;
    }

    // Read a top-level stream by name (ASCII case-insensitive). nullopt if no
    // such stream exists or it could not be read.
    std::optional<std::vector<std::uint8_t>> read_stream(
        const std::string& name) const {
        for (const auto& e : entries_) {
            if (e.kind == EntryKind::Stream &&
                detail::ascii_ci_eq(e.name, name)) {
                try {
                    return read_stream_by_id(e.id);
                } catch (CfbError&) {
                    return std::nullopt;
                }
            }
        }
        return std::nullopt;
    }

    // Read a stream precisely by directory id. Throws CfbError.
    std::vector<std::uint8_t> read_stream_by_id(std::uint32_t id) const {
        using namespace detail;
        if (id >= dir_.size()) {
            throw CfbError::BadDirectory;
        }
        const auto& entry = dir_[id];
        if (entry.object_type != 2) {
            throw CfbError::NotAStream;
        }
        std::uint64_t size = entry.size;
        if (size > MAX_OUTPUT) {
            throw CfbError::OutputTooLarge;
        }
        if (size == 0) {
            return {};
        }
        std::vector<std::uint8_t> bytes =
            size < static_cast<std::uint64_t>(mini_cutoff_)
                ? read_mini_chain(entry.start_sector, size)
                : read_fat_chain(entry.start_sector, size);
        if (size > bytes.size()) {
            throw CfbError::BadSectorChain;
        }
        bytes.resize(static_cast<std::size_t>(size));
        return bytes;
    }

  private:
    // Compute a sector's absolute byte range with overflow-safe bounds checks;
    // returns false (rather than throwing) so callers can map to their error.
    bool sector_ok(std::uint32_t n, std::size_t& start, std::size_t& end) const {
        using namespace detail;
        if (n >= FREESECT - 4) {
            return false;
        }
        if (static_cast<std::size_t>(n) >
            (static_cast<std::size_t>(-1)) / sector_size_) {
            return false;
        }
        std::size_t s = static_cast<std::size_t>(n) * sector_size_;
        if (s > static_cast<std::size_t>(-1) - HEADER_LEN) {
            return false;
        }
        s += HEADER_LEN;
        if (s > static_cast<std::size_t>(-1) - sector_size_) {
            return false;
        }
        std::size_t e = s + sector_size_;
        if (e > data_.size()) {
            return false;
        }
        start = s;
        end = e;
        return true;
    }

    std::vector<std::uint8_t> read_fat_chain(
        std::uint32_t start, std::optional<std::uint64_t> hint) const {
        using namespace detail;
        std::vector<std::uint8_t> out;
        std::uint32_t current = start;
        std::size_t cap_steps = (fat_.empty() ? 1 : fat_.size()) + 1;
        std::size_t steps = 0;
        while (current != ENDOFCHAIN) {
            if (current == FREESECT || current == FATSECT ||
                current == DIFSECT) {
                throw CfbError::BadSectorChain;
            }
            if (steps >= cap_steps) {
                throw CfbError::CycleDetected;
            }
            ++steps;
            std::size_t s, e;
            if (!sector_ok(current, s, e)) {
                throw CfbError::BadSectorChain;
            }
            out.insert(out.end(), data_.begin() + static_cast<std::ptrdiff_t>(s),
                       data_.begin() + static_cast<std::ptrdiff_t>(e));
            if (static_cast<std::uint64_t>(out.size()) > MAX_OUTPUT) {
                throw CfbError::OutputTooLarge;
            }
            if (hint && static_cast<std::uint64_t>(out.size()) >= *hint) {
                break;
            }
            if (static_cast<std::size_t>(current) >= fat_.size()) {
                throw CfbError::BadSectorChain;
            }
            current = fat_[current];
        }
        return out;
    }

    std::vector<std::uint8_t> read_mini_chain(std::uint32_t start,
                                              std::uint64_t size) const {
        using namespace detail;
        if (size > MAX_OUTPUT) {
            throw CfbError::OutputTooLarge;
        }
        std::vector<std::uint8_t> out;
        std::uint32_t current = start;
        std::size_t cap_steps = (mini_fat_.empty() ? 1 : mini_fat_.size()) + 1;
        std::size_t steps = 0;
        while (current != ENDOFCHAIN) {
            if (current == FREESECT || current == FATSECT ||
                current == DIFSECT) {
                throw CfbError::BadSectorChain;
            }
            if (steps >= cap_steps) {
                throw CfbError::CycleDetected;
            }
            ++steps;
            if (static_cast<std::size_t>(current) >
                (static_cast<std::size_t>(-1)) / mini_sector_size_) {
                throw CfbError::BadSectorChain;
            }
            std::size_t off = static_cast<std::size_t>(current) *
                              mini_sector_size_;
            if (off > static_cast<std::size_t>(-1) - mini_sector_size_) {
                throw CfbError::BadSectorChain;
            }
            std::size_t end = off + mini_sector_size_;
            if (end > mini_stream_.size()) {
                throw CfbError::BadSectorChain;
            }
            out.insert(out.end(),
                       mini_stream_.begin() + static_cast<std::ptrdiff_t>(off),
                       mini_stream_.begin() + static_cast<std::ptrdiff_t>(end));
            if (static_cast<std::uint64_t>(out.size()) > MAX_OUTPUT) {
                throw CfbError::OutputTooLarge;
            }
            if (static_cast<std::uint64_t>(out.size()) >= size) {
                break;
            }
            if (static_cast<std::size_t>(current) >= mini_fat_.size()) {
                throw CfbError::BadSectorChain;
            }
            current = mini_fat_[current];
        }
        return out;
    }

    std::vector<std::uint32_t> collect_difat(std::uint32_t first_difat,
                                             std::uint32_t num_difat,
                                             std::uint32_t num_fat_sectors,
                                             std::size_t total_sectors) const {
        using namespace detail;
        std::vector<std::uint32_t> ids;
        for (std::size_t i = 0; i < HEADER_DIFAT_COUNT; ++i) {
            auto v = rd_u32(data_, HEADER_DIFAT_OFFSET + i * 4);
            if (!v) {
                throw CfbError::Truncated;
            }
            if (*v != FREESECT) {
                ids.push_back(*v);
            }
        }
        if (first_difat != ENDOFCHAIN && num_difat > 0) {
            std::size_t per_sector = sector_size_ / 4;
            std::uint32_t current = first_difat;
            std::size_t bound = total_sectors > 1 ? total_sectors : 1;
            std::size_t cap_steps =
                std::min<std::size_t>(num_difat, bound) + 1;
            std::size_t steps = 0;
            while (current != ENDOFCHAIN && current != FREESECT) {
                if (steps >= cap_steps || steps > total_sectors) {
                    throw CfbError::CycleDetected;
                }
                ++steps;
                std::size_t s, e;
                if (!sector_ok(current, s, e)) {
                    throw CfbError::BadSectorChain;
                }
                std::vector<std::uint8_t> sec(data_.begin() +
                                                  static_cast<std::ptrdiff_t>(s),
                                              data_.begin() +
                                                  static_cast<std::ptrdiff_t>(e));
                for (std::size_t k = 0; k + 1 < per_sector; ++k) {
                    auto v = rd_u32(sec, k * 4);
                    if (!v) {
                        throw CfbError::Truncated;
                    }
                    if (*v != FREESECT) {
                        ids.push_back(*v);
                    }
                }
                auto nxt = rd_u32(sec, (per_sector - 1) * 4);
                if (!nxt) {
                    throw CfbError::Truncated;
                }
                current = *nxt;
            }
        }
        if (static_cast<std::size_t>(num_fat_sectors) < ids.size()) {
            ids.resize(num_fat_sectors);
        }
        return ids;
    }

    void assemble_fat(const std::vector<std::uint32_t>& ids,
                      std::size_t total_sectors) {
        std::size_t per_sector = sector_size_ / 4;
        if (ids.size() > total_sectors + 1) {
            throw CfbError::BadSectorChain;
        }
        for (std::uint32_t sid : ids) {
            std::size_t s, e;
            if (!sector_ok(sid, s, e)) {
                throw CfbError::BadSectorChain;
            }
            std::vector<std::uint8_t> sec(data_.begin() +
                                              static_cast<std::ptrdiff_t>(s),
                                          data_.begin() +
                                              static_cast<std::ptrdiff_t>(e));
            for (std::size_t k = 0; k < per_sector; ++k) {
                auto v = detail::rd_u32(sec, k * 4);
                if (!v) {
                    throw CfbError::Truncated;
                }
                fat_.push_back(*v);
            }
        }
    }

    void read_directory(std::uint32_t first_dir_sector) {
        using namespace detail;
        auto raw = read_fat_chain(first_dir_sector, std::nullopt);
        if (raw.empty()) {
            throw CfbError::BadDirectory;
        }
        std::size_t count = raw.size() / DIR_ENTRY_SIZE;
        if (count == 0) {
            throw CfbError::BadDirectory;
        }
        for (std::size_t i = 0; i < count; ++i) {
            std::size_t base = i * DIR_ENTRY_SIZE;
            DirEntry de;
            auto name_len = rd_u16(raw, base + 64);
            de.object_type = raw[base + 66];
            auto left = rd_u32(raw, base + 68);
            auto right = rd_u32(raw, base + 72);
            auto child = rd_u32(raw, base + 76);
            auto start = rd_u32(raw, base + 116);
            auto size = rd_u64(raw, base + 120);
            if (!name_len || !left || !right || !child || !start || !size) {
                throw CfbError::Truncated;
            }
            de.left = *left;
            de.right = *right;
            de.child = *child;
            de.start_sector = *start;
            de.size = *size;
            de.name = decode_utf16le_name(raw, base, *name_len);
            dir_.push_back(std::move(de));
        }
    }

    void enumerate_entries() {
        using namespace detail;
        const auto& root = dir_[0];
        entries_.push_back(Entry{root.name, root.size, EntryKind::RootStorage,
                                 0});
        if (root.child == NOSTREAM) {
            return;
        }
        std::vector<bool> visited(dir_.size(), false);
        std::vector<std::uint32_t> stack{root.child};
        while (!stack.empty()) {
            std::uint32_t id = stack.back();
            stack.pop_back();
            if (id == NOSTREAM) {
                continue;
            }
            if (id >= dir_.size()) {
                throw CfbError::BadDirectory;
            }
            if (visited[id]) {
                throw CfbError::CycleDetected;
            }
            visited[id] = true;
            const auto& de = dir_[id];
            std::optional<EntryKind> kind;
            switch (de.object_type) {
            case 1:
                kind = EntryKind::Storage;
                break;
            case 2:
                kind = EntryKind::Stream;
                break;
            case 5:
                kind = EntryKind::RootStorage;
                break;
            default:
                break;
            }
            if (kind) {
                entries_.push_back(Entry{de.name, de.size, *kind, id});
                if (*kind == EntryKind::Storage) {
                    stack.push_back(de.child);
                }
            }
            stack.push_back(de.left);
            stack.push_back(de.right);
        }
    }

    std::vector<std::uint8_t> data_;
    std::size_t sector_size_ = 0;
    std::size_t mini_sector_size_ = 0;
    std::uint32_t mini_cutoff_ = 0;
    std::vector<std::uint32_t> fat_;
    std::vector<std::uint32_t> mini_fat_;
    std::vector<detail::DirEntry> dir_;
    std::vector<std::uint8_t> mini_stream_;
    std::vector<Entry> entries_;
};

}  // namespace cfb
}  // namespace ca

#endif  // CFB_HPP

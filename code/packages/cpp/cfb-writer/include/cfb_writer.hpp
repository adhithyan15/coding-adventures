// cfb_writer.hpp — Compound File Binary Format writer, header-only ISO C++17.
// ==========================================================================
//
// A faithful port of the Rust `cfb-writer` crate, in namespace `ca::cfb_writer`:
// a from-scratch, zero-dependency writer for the OLE2 / Compound File Binary
// Format ([MS-CFB]) — the container inside legacy .xls / .doc / .ppt files. You
// hand it named streams; it produces a byte buffer a conforming CFB reader (and
// real Office tooling) accepts.
//
// A CFB file is a FAT filesystem in one file: a 512-byte header, then 512-byte
// sectors linked by a File Allocation Table. A directory of 128-byte entries
// names the objects; streams smaller than the 4096-byte cutoff are packed into a
// mini-stream of 64-byte mini-sectors chained by a parallel mini-FAT. Output is
// version 3 and fully deterministic (CLSIDs/timestamps zeroed).
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef CFB_WRITER_HPP
#define CFB_WRITER_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace cfb_writer {

namespace detail {

inline constexpr std::uint8_t kSignature[8] = {0xD0, 0xCF, 0x11, 0xE0,
                                               0xA1, 0xB1, 0x1A, 0xE1};
inline constexpr std::uint32_t kFreeSect = 0xFFFFFFFFu;
inline constexpr std::uint32_t kEndOfChain = 0xFFFFFFFEu;
inline constexpr std::uint32_t kFatSect = 0xFFFFFFFDu;
inline constexpr std::uint32_t kNoStream = 0xFFFFFFFFu;
inline constexpr std::size_t kHeaderLen = 512;
inline constexpr std::size_t kSectorSize = 512;
inline constexpr std::size_t kMiniSectorSize = 64;
inline constexpr std::uint32_t kMiniCutoff = 4096;
inline constexpr std::size_t kDirEntrySize = 128;
inline constexpr std::size_t kHeaderDifatCount = 109;
inline constexpr std::size_t kHeaderDifatOffset = 76;
inline constexpr std::size_t kFatEntriesPerSector = kSectorSize / 4;
inline constexpr std::size_t kMinifatEntriesPerSector = kSectorSize / 4;
inline constexpr std::size_t kMaxNameUnits = 31;
inline constexpr std::uint8_t kObjStream = 0x02;
inline constexpr std::uint8_t kObjRoot = 0x05;
inline constexpr std::uint8_t kColorBlack = 0x01;

inline void put_u16(std::uint8_t *buf, std::size_t off, std::uint16_t v) {
    buf[off] = static_cast<std::uint8_t>(v & 0xff);
    buf[off + 1] = static_cast<std::uint8_t>((v >> 8) & 0xff);
}
inline void put_u32(std::uint8_t *buf, std::size_t off, std::uint32_t v) {
    for (int i = 0; i < 4; ++i)
        buf[off + static_cast<std::size_t>(i)] =
            static_cast<std::uint8_t>((v >> (i * 8)) & 0xff);
}
inline void put_u64(std::uint8_t *buf, std::size_t off, std::uint64_t v) {
    for (int i = 0; i < 8; ++i)
        buf[off + static_cast<std::size_t>(i)] =
            static_cast<std::uint8_t>((v >> (i * 8)) & 0xff);
}

// Round `n` up to a whole number of `unit`s (unit != 0), overflow-safe.
inline std::uint64_t div_round_up(std::uint64_t n, std::uint64_t unit) {
    return n == 0 ? 0 : (n - 1) / unit + 1;
}

// Decode UTF-8 `name` into UTF-16 units, truncated to kMaxNameUnits.
inline std::vector<std::uint16_t> utf8_to_utf16_truncated(
    const std::string &name) {
    std::vector<std::uint16_t> out;
    const auto *s = reinterpret_cast<const unsigned char *>(name.data());
    const auto *end = s + name.size();
    while (s < end && out.size() < kMaxNameUnits) {
        std::uint32_t cp;
        unsigned char b0 = s[0];
        int extra;
        bool ok = true;
        if (b0 < 0x80) {
            cp = b0;
            extra = 0;
        } else if ((b0 & 0xE0) == 0xC0) {
            cp = b0 & 0x1Fu;
            extra = 1;
        } else if ((b0 & 0xF0) == 0xE0) {
            cp = b0 & 0x0Fu;
            extra = 2;
        } else if ((b0 & 0xF8) == 0xF0) {
            cp = b0 & 0x07u;
            extra = 3;
        } else {
            cp = 0xFFFD;
            extra = 0;
            ok = false;
        }
        for (int k = 1; ok && k <= extra; ++k) {
            if (s + k >= end || (s[k] & 0xC0) != 0x80) {
                cp = 0xFFFD;
                extra = 0;
                ok = false;
                break;
            }
            cp = (cp << 6) | static_cast<std::uint32_t>(s[k] & 0x3F);
        }
        s += 1 + (ok ? extra : 0);
        if (cp <= 0xFFFF) {
            out.push_back(static_cast<std::uint16_t>(cp));
        } else {
            std::uint32_t c = cp - 0x10000;
            if (out.size() + 2 > kMaxNameUnits) break;
            out.push_back(static_cast<std::uint16_t>(0xD800 + (c >> 10)));
            out.push_back(static_cast<std::uint16_t>(0xDC00 + (c & 0x3FF)));
        }
    }
    return out;
}

struct DirEntry {
    std::string name;  // UTF-8
    std::uint8_t object_type = 0;
    std::uint32_t right = kNoStream;
    std::uint32_t child = kNoStream;
    std::uint32_t start_sector = kEndOfChain;
    std::uint64_t size = 0;
};

inline std::array<std::uint8_t, kDirEntrySize> encode_dir_entry(
    const DirEntry &e) {
    std::array<std::uint8_t, kDirEntrySize> buf{};
    std::vector<std::uint16_t> units = utf8_to_utf16_truncated(e.name);
    for (std::size_t i = 0; i < units.size(); ++i)
        put_u16(buf.data(), i * 2, units[i]);
    put_u16(buf.data(), 64,
            static_cast<std::uint16_t>((units.size() + 1) * 2));
    buf[66] = e.object_type;
    buf[67] = kColorBlack;
    put_u32(buf.data(), 68, kNoStream);  // left: unused
    put_u32(buf.data(), 72, e.right);
    put_u32(buf.data(), 76, e.child);
    put_u32(buf.data(), 116, e.start_sector);
    put_u64(buf.data(), 120, e.size);
    return buf;
}

inline void append(std::vector<std::uint8_t> &v, const std::uint8_t *p,
                   std::size_t n) {
    v.insert(v.end(), p, p + n);
}
inline void pad_to_sector(std::vector<std::uint8_t> &v) {
    std::size_t rem = v.size() % kSectorSize;
    if (rem != 0) v.resize(v.size() + (kSectorSize - rem), 0);
}

inline std::vector<std::uint8_t> encode_directory(
    const std::vector<DirEntry> &dir) {
    std::vector<std::uint8_t> out;
    for (const auto &e : dir) {
        auto entry = encode_dir_entry(e);
        append(out, entry.data(), entry.size());
    }
    std::size_t rem = out.size() % kSectorSize;
    if (rem != 0) {
        std::size_t n_entries = (kSectorSize - rem) / kDirEntrySize;
        for (std::size_t i = 0; i < n_entries; ++i) {
            std::array<std::uint8_t, kDirEntrySize> e{};
            put_u32(e.data(), 68, kNoStream);
            put_u32(e.data(), 72, kNoStream);
            put_u32(e.data(), 76, kNoStream);
            append(out, e.data(), e.size());
        }
    }
    return out;
}

inline std::vector<std::uint8_t> encode_fat_like(
    const std::vector<std::uint32_t> &entries, std::size_t entries_per_sector) {
    if (entries.empty()) return {};
    std::size_t sectors = static_cast<std::size_t>(
        div_round_up(entries.size(), entries_per_sector));
    std::size_t total_slots = sectors * entries_per_sector;
    std::vector<std::uint8_t> out(total_slots * 4, 0);
    for (std::size_t i = 0; i < entries.size(); ++i)
        put_u32(out.data(), i * 4, entries[i]);
    for (std::size_t i = entries.size(); i < total_slots; ++i)
        put_u32(out.data(), i * 4, kFreeSect);
    return out;
}

}  // namespace detail

// An accumulating set of named streams, in insertion order.
class CfbWriter {
   public:
    CfbWriter() = default;

    // Add a named stream (UTF-8 name, transcoded to UTF-16LE and truncated to
    // 31 code units on disk). Copies the data.
    void add_stream(const std::string &name, const std::vector<std::uint8_t> &data) {
        streams_.emplace_back(name, data);
    }
    void add_stream(const std::string &name, const std::uint8_t *data,
                    std::size_t len) {
        streams_.emplace_back(name, std::vector<std::uint8_t>(data, data + len));
    }

    // Serialise everything into a finished CFB byte buffer.
    std::vector<std::uint8_t> finish() const {
        using namespace detail;
        std::size_t n = streams_.size();

        // 1. Partition + 2. mini-stream / mini-FAT.
        enum Kind { Empty, Mini, Large };
        std::vector<Kind> place(n, Empty);
        std::vector<std::size_t> bucket(n, 0);
        std::vector<std::uint32_t> mini_start_of, large_start_of;
        std::vector<std::uint8_t> mini_stream;
        std::vector<std::uint32_t> minifat;

        for (std::size_t i = 0; i < n; ++i) {
            const auto &data = streams_[i].second;
            if (data.empty()) {
                place[i] = Empty;
            } else if (static_cast<std::uint64_t>(data.size()) < kMiniCutoff) {
                std::uint32_t start_mini =
                    static_cast<std::uint32_t>(minifat.size());
                place[i] = Mini;
                bucket[i] = mini_start_of.size();
                mini_start_of.push_back(start_mini);
                std::size_t n_mini = static_cast<std::size_t>(
                    div_round_up(data.size(), kMiniSectorSize));
                append(mini_stream, data.data(), data.size());
                mini_stream.resize(mini_stream.size() +
                                       (n_mini * kMiniSectorSize - data.size()),
                                   0);
                for (std::size_t j = 0; j < n_mini; ++j)
                    minifat.push_back(j + 1 < n_mini
                                          ? start_mini +
                                                static_cast<std::uint32_t>(j) + 1
                                          : kEndOfChain);
            } else {
                place[i] = Large;
                bucket[i] = large_start_of.size();
                large_start_of.push_back(0);  // filled below
            }
        }

        std::uint64_t mini_stream_size = mini_stream.size();
        pad_to_sector(mini_stream);
        std::vector<std::uint8_t> minifat_bytes =
            encode_fat_like(minifat, kMinifatEntriesPerSector);

        // 3. Directory entries.
        std::vector<DirEntry> dir;
        dir.reserve(n + 1);
        {
            DirEntry root;
            root.name = "Root Entry";
            root.object_type = kObjRoot;
            root.right = kNoStream;
            root.child = n == 0 ? kNoStream : 1u;
            root.start_sector = kEndOfChain;
            root.size = mini_stream_size;
            dir.push_back(root);
        }
        for (std::size_t i = 0; i < n; ++i) {
            DirEntry e;
            e.name = streams_[i].first;
            e.object_type = kObjStream;
            e.right = (i + 1 < n) ? static_cast<std::uint32_t>(i + 2)
                                  : kNoStream;
            e.child = kNoStream;
            e.size = streams_[i].second.size();
            if (place[i] == Empty)
                e.start_sector = kEndOfChain;
            else if (place[i] == Mini)
                e.start_sector = mini_start_of[bucket[i]];
            else
                e.start_sector = 0;  // patched below
            dir.push_back(e);
        }

        // 4. Assign regular sectors.
        std::uint32_t next_sector = 0;
        std::size_t dir_sector_count = static_cast<std::size_t>(div_round_up(
            static_cast<std::uint64_t>(n + 1) * kDirEntrySize, kSectorSize));
        std::uint32_t first_dir_sector = next_sector;
        next_sector += static_cast<std::uint32_t>(dir_sector_count);

        std::size_t minifat_sector_count = minifat_bytes.size() / kSectorSize;
        std::uint32_t first_minifat_sector, num_minifat_sectors;
        if (minifat_sector_count == 0) {
            first_minifat_sector = kEndOfChain;
            num_minifat_sectors = 0;
        } else {
            first_minifat_sector = next_sector;
            next_sector += static_cast<std::uint32_t>(minifat_sector_count);
            num_minifat_sectors =
                static_cast<std::uint32_t>(minifat_sector_count);
        }

        std::size_t mini_stream_sector_count = mini_stream.size() / kSectorSize;
        std::uint32_t mini_stream_start;
        if (mini_stream_sector_count == 0) {
            mini_stream_start = kEndOfChain;
        } else {
            mini_stream_start = next_sector;
            next_sector += static_cast<std::uint32_t>(mini_stream_sector_count);
        }

        for (std::size_t i = 0; i < n; ++i) {
            if (place[i] == Large) {
                std::size_t sc = static_cast<std::size_t>(
                    div_round_up(streams_[i].second.size(), kSectorSize));
                large_start_of[bucket[i]] = next_sector;
                next_sector += static_cast<std::uint32_t>(sc);
            }
        }
        std::size_t data_sectors = next_sector;

        // 4b. FAT chains for the data sectors.
        std::vector<std::uint32_t> fat(data_sectors, kFreeSect);
        auto chain = [&](std::uint32_t start, std::size_t count) {
            for (std::size_t k = 0; k < count; ++k)
                fat[start + k] = (k + 1 < count)
                                     ? start + static_cast<std::uint32_t>(k) + 1
                                     : kEndOfChain;
        };
        chain(first_dir_sector, dir_sector_count);
        if (num_minifat_sectors > 0)
            chain(first_minifat_sector, minifat_sector_count);
        if (mini_stream_sector_count > 0)
            chain(mini_stream_start, mini_stream_sector_count);
        for (std::size_t i = 0; i < n; ++i) {
            if (place[i] == Large) {
                std::size_t sc = static_cast<std::size_t>(
                    div_round_up(streams_[i].second.size(), kSectorSize));
                chain(large_start_of[bucket[i]], sc);
                dir[i + 1].start_sector = large_start_of[bucket[i]];
            }
        }
        dir[0].start_sector = mini_stream_start;

        std::vector<std::uint8_t> directory = encode_directory(dir);

        // 5. Fixed-point the FAT-sector count.
        std::size_t num_fat_sectors = 0;
        for (;;) {
            std::size_t total = data_sectors + num_fat_sectors;
            std::size_t needed = static_cast<std::size_t>(
                div_round_up(total, kFatEntriesPerSector));
            if (needed == num_fat_sectors) break;
            num_fat_sectors = needed;
        }
        fat.resize(data_sectors + num_fat_sectors, kFreeSect);
        for (std::size_t k = 0; k < num_fat_sectors; ++k)
            fat[data_sectors + k] = kFatSect;

        // Serialise.
        std::vector<std::uint8_t> out(kHeaderLen, 0);
        for (int i = 0; i < 8; ++i) out[static_cast<std::size_t>(i)] = kSignature[i];
        put_u16(out.data(), 24, 0x003E);
        put_u16(out.data(), 26, 0x0003);
        put_u16(out.data(), 28, 0xFFFE);
        put_u16(out.data(), 30, 0x0009);
        put_u16(out.data(), 32, 0x0006);
        put_u32(out.data(), 40, 0);
        put_u32(out.data(), 44, static_cast<std::uint32_t>(num_fat_sectors));
        put_u32(out.data(), 48, first_dir_sector);
        put_u32(out.data(), 52, 0);
        put_u32(out.data(), 56, kMiniCutoff);
        put_u32(out.data(), 60, first_minifat_sector);
        put_u32(out.data(), 64, num_minifat_sectors);
        put_u32(out.data(), 68, kEndOfChain);
        put_u32(out.data(), 72, 0);
        for (std::size_t i = 0; i < kHeaderDifatCount; ++i) {
            std::uint32_t v =
                (i < num_fat_sectors)
                    ? static_cast<std::uint32_t>(data_sectors + i)
                    : kFreeSect;
            put_u32(out.data(), kHeaderDifatOffset + i * 4, v);
        }

        append(out, directory.data(), directory.size());
        append(out, minifat_bytes.data(), minifat_bytes.size());
        append(out, mini_stream.data(), mini_stream.size());
        for (std::size_t i = 0; i < n; ++i) {
            if (place[i] == Large) {
                const auto &data = streams_[i].second;
                std::size_t sc = static_cast<std::size_t>(
                    div_round_up(data.size(), kSectorSize));
                append(out, data.data(), data.size());
                out.resize(out.size() + (sc * kSectorSize - data.size()), 0);
            }
        }
        std::vector<std::uint8_t> fat_bytes =
            encode_fat_like(fat, kFatEntriesPerSector);
        append(out, fat_bytes.data(), fat_bytes.size());
        return out;
    }

   private:
    std::vector<std::pair<std::string, std::vector<std::uint8_t>>> streams_;
};

// One-shot convenience: name/data pairs → CFB bytes.
inline std::vector<std::uint8_t> write_cfb(
    const std::vector<std::pair<std::string, std::vector<std::uint8_t>>>
        &streams) {
    CfbWriter w;
    for (const auto &s : streams) w.add_stream(s.first, s.second);
    return w.finish();
}

}  // namespace cfb_writer
}  // namespace ca

#endif  // CFB_WRITER_HPP

#ifndef CA_NAND_FLASH_HPP
#define CA_NAND_FLASH_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace nand_flash {

inline constexpr std::int64_t max_blocks = 1024;
inline constexpr std::int64_t max_pages_per_block = 256;
inline constexpr std::int64_t max_page_size_bytes = 65536;
inline constexpr std::uint64_t max_storage_bytes = std::uint64_t{16} * 1024u * 1024u;
inline constexpr std::uint64_t max_safe_erase_cycles = 9007199254740991ULL;

struct Geometry {
    std::int64_t blocks = 0;
    std::int64_t pages_per_block = 0;
    std::int64_t page_size_bytes = 0;
    bool operator==(const Geometry &other) const noexcept {
        return blocks == other.blocks && pages_per_block == other.pages_per_block &&
               page_size_bytes == other.page_size_bytes;
    }
};

struct Config {
    Geometry geometry;
    std::optional<std::int64_t> max_erase_cycles = std::nullopt;
};

class NandFlashError : public std::runtime_error {
  public:
    explicit NandFlashError(const char *message) : std::runtime_error(message) {}
};

class GeometryError final : public NandFlashError {
  public:
    GeometryError() : NandFlashError("geometry values are outside supported bounds") {}
};

class AddressError final : public NandFlashError {
  public:
    explicit AddressError(const char *message) : NandFlashError(message) {}
};

class PayloadError final : public NandFlashError {
  public:
    PayloadError() : NandFlashError("page data must match page_size_bytes") {}
};

class ProgramError final : public NandFlashError {
  public:
    ProgramError() : NandFlashError("page has already been programmed since erase") {}
};

class EraseLimitError final : public NandFlashError {
  public:
    EraseLimitError() : NandFlashError("block erase limit reached") {}
};

struct Snapshot {
    std::vector<std::vector<std::string>> pages;
    std::vector<std::uint64_t> erase_counts;
    std::vector<std::vector<bool>> programmed;
};

inline bool operator==(const Snapshot &left, const Snapshot &right) {
    return left.pages == right.pages && left.erase_counts == right.erase_counts &&
           left.programmed == right.programmed;
}

class Device final {
  public:
    explicit Device(Config config);
    Device(const Device &) = delete;
    Device &operator=(const Device &) = delete;
    Device(Device &&) = delete;
    Device &operator=(Device &&) = delete;

    Geometry geometry() const noexcept { return geometry_; }
    std::vector<std::uint8_t> read(std::int64_t block, std::int64_t page) const;
    void program(std::int64_t block, std::int64_t page, const std::vector<std::uint8_t> &data);
    void erase_block(std::int64_t block);
    std::uint64_t erase_count(std::int64_t block) const;
    bool is_programmed(std::int64_t block, std::int64_t page) const;
    Snapshot snapshot() const;

  private:
    std::size_t block_index(std::int64_t block) const;
    std::size_t page_index(std::int64_t block, std::int64_t page) const;

    Geometry geometry_;
    std::size_t blocks_ = 0;
    std::size_t pages_per_block_ = 0;
    std::size_t page_size_bytes_ = 0;
    bool has_erase_limit_ = false;
    std::uint64_t max_erase_cycles_ = 0;
    std::vector<std::uint8_t> pages_;
    std::vector<bool> programmed_;
    std::vector<std::uint64_t> erase_counts_;
};

} // namespace nand_flash
} // namespace ca

#endif

#include "nand_flash.hpp"
#include "erase_limit.hpp"

#include <algorithm>
#include <limits>

namespace ca {
namespace nand_flash {
namespace {

std::size_t checked_size(std::uint64_t value) {
    if constexpr (sizeof(std::size_t) < sizeof(std::uint64_t)) {
        if (value > static_cast<std::uint64_t>((std::numeric_limits<std::size_t>::max)())) {
            throw GeometryError();
        }
    }
    return static_cast<std::size_t>(value);
}

std::string hex_page(const std::uint8_t *bytes, std::size_t length) {
    static constexpr char digits[] = "0123456789abcdef";
    std::string result;
    result.reserve(length * 2);
    for (std::size_t i = 0; i < length; ++i) {
        result.push_back(digits[bytes[i] >> 4]);
        result.push_back(digits[bytes[i] & 0x0f]);
    }
    return result;
}

} // namespace

Device::Device(Config config) : geometry_(config.geometry) {
    const Geometry g = config.geometry;
    if (g.blocks < 1 || g.blocks > max_blocks || g.pages_per_block < 1 ||
        g.pages_per_block > max_pages_per_block || g.page_size_bytes < 1 ||
        g.page_size_bytes > max_page_size_bytes ||
        (config.max_erase_cycles &&
         (*config.max_erase_cycles < 0 ||
          static_cast<std::uint64_t>(*config.max_erase_cycles) > max_safe_erase_cycles))) {
        throw GeometryError();
    }

    const auto block_count = static_cast<std::uint64_t>(g.blocks);
    const auto pages_per_block = static_cast<std::uint64_t>(g.pages_per_block);
    const auto page_size = static_cast<std::uint64_t>(g.page_size_bytes);
    const std::uint64_t page_count = block_count * pages_per_block;
    if (page_count > max_storage_bytes / page_size)
        throw GeometryError();
    const std::uint64_t storage_bytes = page_count * page_size;

    blocks_ = checked_size(block_count);
    pages_per_block_ = checked_size(pages_per_block);
    page_size_bytes_ = checked_size(page_size);
    const std::size_t page_count_size = checked_size(page_count);
    const std::size_t storage_size = checked_size(storage_bytes);
    has_erase_limit_ = config.max_erase_cycles.has_value();
    max_erase_cycles_ = has_erase_limit_ ? static_cast<std::uint64_t>(*config.max_erase_cycles) : 0;
    pages_.assign(storage_size, UINT8_C(0xFF));
    programmed_.assign(page_count_size, 0);
    erase_counts_.assign(blocks_, 0);
}

std::size_t Device::block_index(std::int64_t block) const {
    if (block < 0 || block >= geometry_.blocks) {
        throw AddressError("block index out of range");
    }
    return static_cast<std::size_t>(block);
}

std::size_t Device::page_index(std::int64_t block, std::int64_t page) const {
    const std::size_t b = block_index(block);
    if (page < 0 || page >= geometry_.pages_per_block) {
        throw AddressError("page index out of range");
    }
    return b * pages_per_block_ + static_cast<std::size_t>(page);
}

std::vector<std::uint8_t> Device::read(std::int64_t block, std::int64_t page) const {
    const std::size_t index = page_index(block, page);
    const std::size_t start = index * page_size_bytes_;
    return std::vector<std::uint8_t>(pages_.begin() + static_cast<std::ptrdiff_t>(start),
                                     pages_.begin() +
                                         static_cast<std::ptrdiff_t>(start + page_size_bytes_));
}

void Device::program(std::int64_t block, std::int64_t page, const std::vector<std::uint8_t> &data) {
    const std::size_t index = page_index(block, page);
    if (data.size() != page_size_bytes_)
        throw PayloadError();
    if (programmed_[index] != 0)
        throw ProgramError();
    const std::size_t start = index * page_size_bytes_;
    std::copy(data.begin(), data.end(), pages_.begin() + static_cast<std::ptrdiff_t>(start));
    programmed_[index] = 1;
}

void Device::erase_block(std::int64_t block) {
    const std::size_t index = block_index(block);
    const std::uint64_t count = erase_counts_[index];
    if (!detail::erase_allowed(count, has_erase_limit_, max_erase_cycles_)) {
        throw EraseLimitError();
    }
    const std::size_t first_page = index * pages_per_block_;
    const std::size_t page_end = first_page + pages_per_block_;
    const std::size_t first_byte = first_page * page_size_bytes_;
    const std::size_t byte_end = page_end * page_size_bytes_;
    std::fill(pages_.begin() + static_cast<std::ptrdiff_t>(first_byte),
              pages_.begin() + static_cast<std::ptrdiff_t>(byte_end), std::uint8_t{0xFF});
    std::fill(programmed_.begin() + static_cast<std::ptrdiff_t>(first_page),
              programmed_.begin() + static_cast<std::ptrdiff_t>(page_end), 0);
    erase_counts_[index] = count + 1;
}

std::uint64_t Device::erase_count(std::int64_t block) const {
    return erase_counts_[block_index(block)];
}

bool Device::is_programmed(std::int64_t block, std::int64_t page) const {
    return programmed_[page_index(block, page)] != 0;
}

Snapshot Device::snapshot() const {
    Snapshot result;
    result.pages.resize(blocks_);
    result.erase_counts = erase_counts_;
    result.programmed.resize(blocks_);
    for (std::size_t block = 0; block < blocks_; ++block) {
        result.pages[block].reserve(pages_per_block_);
        result.programmed[block].reserve(pages_per_block_);
        for (std::size_t page = 0; page < pages_per_block_; ++page) {
            const std::size_t index = block * pages_per_block_ + page;
            const std::size_t start = index * page_size_bytes_;
            result.pages[block].push_back(hex_page(pages_.data() + start, page_size_bytes_));
            result.programmed[block].push_back(programmed_[index] != 0);
        }
    }
    return result;
}

} // namespace nand_flash
} // namespace ca

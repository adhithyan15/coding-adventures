#ifndef CA_NAND_FLASH_INDEPENDENT_MODEL_HPP
#define CA_NAND_FLASH_INDEPENDENT_MODEL_HPP

#include "nand_flash.hpp"

#include <cstdint>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace nand_test {

enum class ModelError { block, page, payload_size, programmed, erase_limit };

class Model final {
  public:
    explicit Model(ca::nand_flash::Config config)
        : geometry_(config.geometry), max_erase_cycles_(config.max_erase_cycles) {
        const auto blocks = static_cast<std::size_t>(geometry_.blocks);
        const auto pages = static_cast<std::size_t>(geometry_.pages_per_block);
        const auto bytes = static_cast<std::size_t>(geometry_.page_size_bytes);
        pages_.assign(blocks, std::vector<std::vector<std::uint8_t>>(
                                  pages, std::vector<std::uint8_t>(bytes, UINT8_C(0xFF))));
        programmed_.assign(blocks, std::vector<bool>(pages, false));
        erase_counts_.assign(blocks, 0);
    }

    std::vector<std::uint8_t> read(std::int64_t block, std::int64_t page) const {
        check_address(block, page);
        return pages_[static_cast<std::size_t>(block)][static_cast<std::size_t>(page)];
    }

    void program(std::int64_t block, std::int64_t page, const std::vector<std::uint8_t> &data) {
        check_address(block, page);
        if (data.size() != static_cast<std::size_t>(geometry_.page_size_bytes)) {
            throw ModelError::payload_size;
        }
        const auto b = static_cast<std::size_t>(block);
        const auto p = static_cast<std::size_t>(page);
        if (programmed_[b][p])
            throw ModelError::programmed;
        pages_[b][p] = data;
        programmed_[b][p] = true;
    }

    void erase_block(std::int64_t block) {
        if (block < 0 || block >= geometry_.blocks)
            throw ModelError::block;
        const auto b = static_cast<std::size_t>(block);
        const auto count = erase_counts_[b];
        if ((max_erase_cycles_ && count >= static_cast<std::uint64_t>(*max_erase_cycles_)) ||
            count >= ca::nand_flash::max_safe_erase_cycles)
            throw ModelError::erase_limit;
        for (std::size_t page = 0; page < pages_[b].size(); ++page) {
            pages_[b][page].assign(static_cast<std::size_t>(geometry_.page_size_bytes),
                                   UINT8_C(0xFF));
            programmed_[b][page] = false;
        }
        erase_counts_[b] = count + 1;
    }

    std::uint64_t erase_count(std::int64_t block) const {
        if (block < 0 || block >= geometry_.blocks)
            throw ModelError::block;
        return erase_counts_[static_cast<std::size_t>(block)];
    }

    bool is_programmed(std::int64_t block, std::int64_t page) const {
        check_address(block, page);
        return programmed_[static_cast<std::size_t>(block)][static_cast<std::size_t>(page)];
    }

    ca::nand_flash::Snapshot snapshot() const {
        static constexpr char digits[] = "0123456789abcdef";
        ca::nand_flash::Snapshot result;
        result.erase_counts = erase_counts_;
        result.pages.resize(pages_.size());
        result.programmed = programmed_;
        for (std::size_t block = 0; block < pages_.size(); ++block) {
            for (const auto &page : pages_[block]) {
                std::string hex;
                for (const auto byte : page) {
                    hex.push_back(digits[byte >> 4]);
                    hex.push_back(digits[byte & 0x0f]);
                }
                result.pages[block].push_back(std::move(hex));
            }
        }
        return result;
    }

  private:
    void check_address(std::int64_t block, std::int64_t page) const {
        if (block < 0 || block >= geometry_.blocks)
            throw ModelError::block;
        if (page < 0 || page >= geometry_.pages_per_block)
            throw ModelError::page;
    }

    ca::nand_flash::Geometry geometry_;
    std::optional<std::int64_t> max_erase_cycles_;
    std::vector<std::vector<std::vector<std::uint8_t>>> pages_;
    std::vector<std::vector<bool>> programmed_;
    std::vector<std::uint64_t> erase_counts_;
};

} // namespace nand_test

#endif

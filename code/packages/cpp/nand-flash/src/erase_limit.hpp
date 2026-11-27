#ifndef CA_NAND_FLASH_ERASE_LIMIT_HPP
#define CA_NAND_FLASH_ERASE_LIMIT_HPP
#include "nand_flash.hpp"
namespace ca {
namespace nand_flash {
namespace detail {
inline bool erase_allowed(std::uint64_t count, bool has_limit, std::uint64_t limit) noexcept {
    return count < max_safe_erase_cycles && (!has_limit || count < limit);
}
} // namespace detail
} // namespace nand_flash
} // namespace ca
#endif

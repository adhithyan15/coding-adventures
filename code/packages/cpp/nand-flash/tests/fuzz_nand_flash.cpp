#include "independent_model.hpp"
#include "nand_flash.hpp"
#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace {
std::string model_error(nand_test::ModelError e) {
    using nand_test::ModelError;
    switch (e) {
    case ModelError::block:
        return "AddressError:block index out of range";
    case ModelError::page:
        return "AddressError:page index out of range";
    case ModelError::payload_size:
        return "PayloadError:page data must match page_size_bytes";
    case ModelError::programmed:
        return "ProgramError:page has already been programmed since erase";
    case ModelError::erase_limit:
        return "EraseLimitError:block erase limit reached";
    }
    return {};
}
std::string actual_error(const ca::nand_flash::NandFlashError &e) {
    using namespace ca::nand_flash;
    if (dynamic_cast<const AddressError *>(&e))
        return std::string("AddressError:") + e.what();
    if (dynamic_cast<const PayloadError *>(&e))
        return std::string("PayloadError:") + e.what();
    if (dynamic_cast<const ProgramError *>(&e))
        return std::string("ProgramError:") + e.what();
    if (dynamic_cast<const EraseLimitError *>(&e))
        return std::string("EraseLimitError:") + e.what();
    if (dynamic_cast<const GeometryError *>(&e))
        return std::string("GeometryError:") + e.what();
    return std::string("NandFlashError:") + e.what();
}
} // namespace
extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t *data, std::size_t size) {
    if (size > 4096)
        return 0;
    using namespace ca::nand_flash;
    Config cfg{{2, 2, 4}, 2};
    Device device(cfg);
    nand_test::Model model(cfg);
    for (std::size_t i = 0; i + 3 < size; i += 4) {
        const std::int64_t b = static_cast<std::int64_t>(data[i] % 4) - 1;
        const std::int64_t p = static_cast<std::int64_t>(data[i + 1] % 4) - 1;
        const auto op = data[i + 2] % 4;
        std::vector<std::uint8_t> bytes(4, data[i + 3]), expected_read, actual_read;
        std::string expected_error, got_error;
        try {
            if (op == 0)
                expected_read = model.read(b, p);
            else if (op == 1)
                model.program(b, p, bytes);
            else if (op == 2)
                model.erase_block(b);
            else
                model.program(b, p, std::vector<std::uint8_t>{data[i + 3]});
        } catch (nand_test::ModelError e) {
            expected_error = model_error(e);
        }
        try {
            if (op == 0)
                actual_read = device.read(b, p);
            else if (op == 1)
                device.program(b, p, bytes);
            else if (op == 2)
                device.erase_block(b);
            else
                device.program(b, p, std::vector<std::uint8_t>{data[i + 3]});
        } catch (const NandFlashError &e) {
            got_error = actual_error(e);
        }
        if (expected_error != got_error || (op == 0 && expected_read != actual_read) ||
            !(device.snapshot() == model.snapshot()))
            __builtin_trap();
    }
    return 0;
}

#ifndef CA_NAND_FLASH_MODEL_TEST_HPP
#define CA_NAND_FLASH_MODEL_TEST_HPP
#include "independent_model.hpp"
#include "iso_test.h"
#include "nand_flash.hpp"
#include <cstdint>
#include <string>
#include <vector>

namespace nand_test {
using namespace ca::nand_flash;
inline std::string model_error_text(ModelError e) {
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
    return "";
}
inline std::string device_error_text(const NandFlashError &e) {
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
inline void bounded_histories() {
    for (std::uint32_t history = 0; history < 4096; ++history) {
        Config cfg{{1, 1, 1}, 1};
        Model model(cfg);
        Device device(cfg);
        std::uint32_t digits = history;
        for (int step = 0; step < 6; ++step) {
            const int op = digits & 3u;
            digits >>= 2;
            std::vector<std::uint8_t> data{static_cast<std::uint8_t>(op == 2 ? 0 : 0xff)};
            std::string expected_error, actual_error;
            std::vector<std::uint8_t> expected_read, actual_read;
            try {
                if (op == 0)
                    expected_read = model.read(0, 0);
                else if (op == 1)
                    model.program(0, 0, data);
                else if (op == 2)
                    model.program(0, 0, data);
                else
                    model.erase_block(0);
            } catch (ModelError e) {
                expected_error = model_error_text(e);
            }
            try {
                if (op == 0)
                    actual_read = device.read(0, 0);
                else if (op == 1)
                    device.program(0, 0, data);
                else if (op == 2)
                    device.program(0, 0, data);
                else
                    device.erase_block(0);
            } catch (const NandFlashError &e) {
                actual_error = device_error_text(e);
            }
            ISO_CHECK(expected_error == actual_error);
            if (op == 0)
                ISO_CHECK(expected_read == actual_read);
            ISO_CHECK(device.snapshot() == model.snapshot());
        }
    }
}
inline void seeded_model() {
    Config cfg{{3, 2, 3}, 2};
    Model model(cfg);
    Device device(cfg);
    std::uint32_t state = 0x6d2b79f5u;
    for (unsigned i = 0; i < 3000; ++i) {
        state = state * 1664525u + 1013904223u;
        const int op = (state >> 27) % 4;
        const std::int64_t b = static_cast<std::int64_t>((state >> 8) % 5) - 1;
        const std::int64_t p = static_cast<std::int64_t>((state >> 16) % 4) - 1;
        std::vector<std::uint8_t> bytes(3, static_cast<std::uint8_t>(state));
        std::string expected_error, actual_error;
        std::vector<std::uint8_t> expected_read, actual_read;
        try {
            if (op == 0)
                expected_read = model.read(b, p);
            else if (op == 1)
                model.program(b, p, bytes);
            else if (op == 2)
                model.erase_block(b);
            else
                model.program(b, p, {1});
        } catch (ModelError e) {
            expected_error = model_error_text(e);
        }
        try {
            if (op == 0)
                actual_read = device.read(b, p);
            else if (op == 1)
                device.program(b, p, bytes);
            else if (op == 2)
                device.erase_block(b);
            else
                device.program(b, p, {1});
        } catch (const NandFlashError &e) {
            actual_error = device_error_text(e);
        }
        ISO_CHECK(expected_error == actual_error);
        if (op == 0)
            ISO_CHECK(expected_read == actual_read);
        ISO_CHECK(device.snapshot() == model.snapshot());
    }
}
} // namespace nand_test
#endif

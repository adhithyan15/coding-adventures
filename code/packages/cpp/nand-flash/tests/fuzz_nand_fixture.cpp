#define main nand_flash_native_tests_main
#include "nand_flash_test.cpp"
#undef main

#include <cstdlib>
#include <string_view>

extern "C" int LLVMFuzzerTestOneInput(const std::uint8_t *data,
                                      std::size_t size) {
  if (size > 65537)
    return 0;

  const std::string_view text =
      size == 0 ? std::string_view{}
                : std::string_view(reinterpret_cast<const char *>(data), size);
  try {
    // parse_fixture first checks every declaration with the independent
    // model. Only a fully admitted document reaches the device replay.
    const auto fixture = parse_fixture(text);
    iso_test_checks_run = 0;
    iso_test_checks_failed = 0;
    exercise(fixture);
    if (iso_test_checks_failed != 0)
      std::abort();
  } catch (const FixtureError &) {
    // Malformed or model-inconsistent generated inputs are expected refusals.
    return 0;
  } catch (...) {
    std::abort();
  }
  return 0;
}

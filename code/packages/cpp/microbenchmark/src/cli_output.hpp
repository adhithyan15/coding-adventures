#ifndef CA_MICROBENCHMARK_CLI_OUTPUT_HPP
#define CA_MICROBENCHMARK_CLI_OUTPUT_HPP

#include <ostream>
#include <stdexcept>
#include <string_view>

namespace ca { namespace microbenchmark { namespace detail {

inline void write_and_flush(std::ostream& output, std::string_view text) {
    output.write(text.data(), static_cast<std::streamsize>(text.size()));
    output.flush();
    if (!output) throw std::runtime_error("failed to write CLI output");
}

}}} // namespace ca::microbenchmark::detail

#endif

// Historical Web profiles: CERN HTTP/0.9 and the November 1992 HTML Tags list.
#ifndef HISTORICAL_WEB_HPP
#define HISTORICAL_WEB_HPP

#include <cstddef>
#include <limits>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace coding_adventures::historical_web {

inline constexpr std::size_t no_index = std::numeric_limits<std::size_t>::max();

enum class ErrorKind { invalid_address, invalid_request, invalid_response, invalid_utf8 };

class ProfileError : public std::runtime_error {
public:
    ProfileError(ErrorKind kind, std::size_t byte_offset, const char *message);
    ErrorKind kind() const noexcept { return kind_; }
    std::size_t byte_offset() const noexcept { return byte_offset_; }

private:
    ErrorKind kind_;
    std::size_t byte_offset_;
};

enum class NodeKind { text, element };

struct HtmlNode {
    NodeKind kind = NodeKind::text;
    std::size_t parent_index = no_index;
    std::size_t first_child_index = no_index;
    std::size_t next_sibling_index = no_index;
    std::string tag;
    std::string text;
    std::size_t first_attribute_index = no_index;
    std::size_t attribute_count = 0;
};

struct HtmlAttribute {
    std::string name;
    std::optional<std::string> value;
};

class HtmlParser;

class HtmlDocument {
public:
    HtmlDocument() = default;
    HtmlDocument(const HtmlDocument &) = default;
    HtmlDocument &operator=(const HtmlDocument &other);
    HtmlDocument(HtmlDocument &&other) noexcept;
    HtmlDocument &operator=(HtmlDocument &&other) noexcept;

    const std::vector<HtmlNode> &nodes() const noexcept { return nodes_; }
    const std::vector<HtmlAttribute> &attributes() const noexcept { return attributes_; }
    std::size_t first_root() const noexcept;
    std::size_t root_count() const noexcept { return root_count_; }
    std::size_t node_count() const noexcept { return nodes_.size(); }
    std::size_t attribute_count() const noexcept { return attributes_.size(); }
    const HtmlNode &node_at(std::size_t index) const;
    const HtmlAttribute &attribute_at(std::size_t index) const;

private:
    friend class HtmlParser;
    std::vector<HtmlNode> nodes_;
    std::vector<HtmlAttribute> attributes_;
    std::size_t root_count_ = 0;
    void swap(HtmlDocument &other) noexcept;
};

std::string encode_http09_request(std::string_view address);
std::string parse_http09_request(std::string_view wire);
std::string decode_http09_response(std::string_view body);
HtmlDocument parse_html_tags_1992(std::string_view source);

} // namespace coding_adventures::historical_web

#endif // HISTORICAL_WEB_HPP

// An in-memory implementation of the two HWEB00 profiles.
// The parser publishes a flat arena, so even very deep input has iterative
// construction and destruction rather than a recursively owned tree.
#include "historical_web.hpp"

#include <algorithm>
#include <array>
#include <cstdint>
#include <string>
#include <utility>

namespace coding_adventures::historical_web {
namespace {

enum Tag : std::size_t {
    unknown, title, address, anchor, h1, h2, h3, h4, h5, h6,
    ul, menu, dir, li, dl, dt, dd, paragraph, isindex, xmp, listing,
    plaintext, tag_count
};
constexpr std::array<std::string_view, tag_count> tag_names = {
    "", "title", "address", "a", "h1", "h2", "h3", "h4", "h5", "h6",
    "ul", "menu", "dir", "li", "dl", "dt", "dd", "p", "isindex",
    "xmp", "listing", "plaintext"};
constexpr std::array<std::string_view, 3> entities = {"&lt;", "&gt;", "&amp;"};
constexpr std::array<char, 3> replacements = {'<', '>', '&'};

bool ascii_alpha(char c) noexcept {
    return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
}
bool name_char(char c) noexcept {
    return ascii_alpha(c) || (c >= '0' && c <= '9') || c == '-';
}
bool space(char c) noexcept {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f';
}
char lower_ascii(char c) noexcept {
    return c >= 'A' && c <= 'Z' ? static_cast<char>(c + ('a' - 'A')) : c;
}
bool equal_ascii_ci(std::string_view a, std::string_view b) noexcept {
    if (a.size() != b.size()) {
        return false;
    }
    for (std::size_t i = 0; i < a.size(); ++i) {
        if (lower_ascii(a[i]) != lower_ascii(b[i])) {
            return false;
        }
    }
    return true;
}
Tag identify(std::string_view name) noexcept {
    for (std::size_t i = 1; i < tag_names.size(); ++i)
        if (equal_ascii_ci(name, tag_names[i])) return static_cast<Tag>(i);
    return unknown;
}
bool heading(Tag t) noexcept { return t >= h1 && t <= h6; }
bool empty(Tag t) noexcept { return t == paragraph || t == isindex; }

bool continuation(unsigned char b) noexcept {
    return b >= 0x80u && b <= 0xbfu;
}
bool valid_utf8(std::string_view s, std::size_t &bad) noexcept {
    std::size_t i = 0;
    while (i < s.size()) {
        const auto a = static_cast<unsigned char>(s[i]);
        const std::size_t left = s.size() - i;
        if (a <= 0x7f) {
            ++i;
            continue;
        }
        if (a >= 0xc2 && a <= 0xdf) {
            if (left < 2) {
                bad = s.size();
                return false;
            }
            if (!continuation(static_cast<unsigned char>(s[i + 1]))) {
                bad = i + 1;
                return false;
            }
            i += 2;
            continue;
        }
        if (a >= 0xe0 && a <= 0xef) {
            if (left < 2) {
                bad = s.size();
                return false;
            }
            const auto b = static_cast<unsigned char>(s[i + 1]);
            if ((a == 0xe0 && (b < 0xa0 || b > 0xbf)) ||
                (a == 0xed && (b < 0x80 || b > 0x9f)) ||
                ((a != 0xe0 && a != 0xed) && !continuation(b))) {
                bad = i + 1;
                return false;
            }
            if (left < 3) {
                bad = s.size();
                return false;
            }
            if (!continuation(static_cast<unsigned char>(s[i + 2]))) {
                bad = i + 2;
                return false;
            }
            i += 3;
            continue;
        }
        if (a >= 0xf0 && a <= 0xf4) {
            if (left < 2) {
                bad = s.size();
                return false;
            }
            const auto b = static_cast<unsigned char>(s[i + 1]);
            if ((a == 0xf0 && (b < 0x90 || b > 0xbf)) ||
                (a == 0xf4 && (b < 0x80 || b > 0x8f)) ||
                ((a != 0xf0 && a != 0xf4) && !continuation(b))) {
                bad = i + 1;
                return false;
            }
            if (left < 3) {
                bad = s.size();
                return false;
            }
            if (!continuation(static_cast<unsigned char>(s[i + 2]))) {
                bad = i + 2;
                return false;
            }
            if (left < 4) {
                bad = s.size();
                return false;
            }
            if (!continuation(static_cast<unsigned char>(s[i + 3]))) {
                bad = i + 3;
                return false;
            }
            i += 4;
            continue;
        }
        bad = i;
        return false;
    }
    return true;
}

std::string decode(std::string_view source) {
    std::string result;
    result.reserve(source.size());
    std::size_t i = 0;
    while (i < source.size()) {
        if (source[i] == '&') {
            bool matched = false;
            for (std::size_t e = 0; e < entities.size(); ++e) {
                if (source.substr(i, entities[e].size()) == entities[e]) {
                    result.push_back(replacements[e]); i += entities[e].size(); matched = true; break;
                }
            }
            if (matched) continue;
        }
        result.push_back(source[i++]);
    }
    return result;
}

struct ParsedTag { Tag id; bool closing; std::string_view attrs; std::size_t end; };
bool parse_tag(std::string_view source, std::size_t start, ParsedTag &out) {
    std::size_t i = start + 1;
    bool closing = i < source.size() && source[i] == '/';
    if (closing) ++i;
    if (i == source.size() || !ascii_alpha(source[i])) return false;
    const std::size_t name_start = i++;
    while (i < source.size() && name_char(source[i])) ++i;
    const Tag id = identify(source.substr(name_start, i - name_start));
    if (i == source.size() || source[i] == '<') return false;
    if (source[i] == '>') { out = {id, closing, {}, i + 1}; return true; }
    if (!space(source[i])) return false;
    const std::size_t attrs_start = i;
    while (i < source.size()) {
        if (source[i] == '>') { out = {id, closing, source.substr(attrs_start, i - attrs_start), i + 1}; return true; }
        if (source[i] == '<') return false;
        if (source[i] == '\'' || source[i] == '"') {
            const char quote = source[i++];
            while (i < source.size() && source[i] != quote && source[i] != '<') ++i;
            if (i == source.size() || source[i] != quote) return false;
        }
        ++i;
    }
    return false;
}

struct ParsedAttribute { std::string_view name; std::optional<std::string_view> value; std::size_t next; };
bool parse_attribute(std::string_view source, std::size_t cursor, ParsedAttribute &out) {
    const auto initial = cursor;
    while (cursor < source.size() && space(source[cursor])) ++cursor;
    if (cursor == initial || cursor == source.size() || !ascii_alpha(source[cursor])) return false;
    const std::size_t name_start = cursor++;
    while (cursor < source.size() && name_char(source[cursor])) ++cursor;
    const auto name = source.substr(name_start, cursor - name_start);
    const auto value_start = cursor;
    while (cursor < source.size() && space(source[cursor])) ++cursor;
    if (cursor == source.size() || source[cursor] != '=') { out = {name, std::nullopt, value_start}; return true; }
    ++cursor;
    while (cursor < source.size() && space(source[cursor])) ++cursor;
    if (cursor == source.size()) { out = {name, std::nullopt, value_start}; return true; }
    if (source[cursor] == '\'' || source[cursor] == '"') {
        const char quote = source[cursor++];
        const auto start = cursor;
        while (cursor < source.size() && source[cursor] != quote) ++cursor;
        if (cursor == source.size()) return false;
        out = {name, source.substr(start, cursor - start), cursor + 1}; return true;
    }
    const auto start = cursor;
    while (cursor < source.size() && !space(source[cursor]) && source[cursor] != '"' && source[cursor] != '\'' &&
           source[cursor] != '=' && source[cursor] != '<' && source[cursor] != '>') ++cursor;
    if (cursor == start) { out = {name, std::nullopt, value_start}; return true; }
    out = {name, source.substr(start, cursor - start), cursor}; return true;
}

} // namespace

ProfileError::ProfileError(ErrorKind kind, std::size_t byte_offset, const char *message)
    : std::runtime_error(message), kind_(kind), byte_offset_(byte_offset) {}

void HtmlDocument::swap(HtmlDocument &other) noexcept {
    nodes_.swap(other.nodes_); attributes_.swap(other.attributes_); std::swap(root_count_, other.root_count_);
}
HtmlDocument &HtmlDocument::operator=(const HtmlDocument &other) {
    if (this != &other) { HtmlDocument copy(other); swap(copy); }
    return *this;
}
HtmlDocument::HtmlDocument(HtmlDocument &&other) noexcept { swap(other); }
HtmlDocument &HtmlDocument::operator=(HtmlDocument &&other) noexcept {
    if (this != &other) { HtmlDocument moved(std::move(other)); swap(moved); }
    return *this;
}
std::size_t HtmlDocument::first_root() const noexcept {
    return nodes_.empty() ? no_index : 0;
}
const HtmlNode &HtmlDocument::node_at(std::size_t index) const { return nodes_.at(index); }
const HtmlAttribute &HtmlDocument::attribute_at(std::size_t index) const { return attributes_.at(index); }

std::string encode_http09_request(std::string_view address) {
    if (address.empty()) {
        throw ProfileError(ErrorKind::invalid_address, no_index,
                           "invalid HTTP/0.9 address");
    }
    for (std::size_t i = 0; i < address.size(); ++i) {
        const auto c = static_cast<unsigned char>(address[i]);
        if (c < 0x21 || c > 0x7e) {
            throw ProfileError(ErrorKind::invalid_address, i,
                               "invalid HTTP/0.9 address");
        }
    }
    std::string result("GET ");
    result.append(address.data(), address.size());
    result.append("\r\n", 2);
    return result;
}
std::string parse_http09_request(std::string_view wire) {
    constexpr std::string_view prefix = "GET ";
    const auto prefix_length = std::min(wire.size(), prefix.size());
    for (std::size_t i = 0; i < prefix_length; ++i) {
        if (wire[i] != prefix[i]) {
            throw ProfileError(ErrorKind::invalid_request, i,
                               "invalid HTTP/0.9 request");
        }
    }
    if (wire.size() < prefix.size()) {
        throw ProfileError(ErrorKind::invalid_request, wire.size(),
                           "invalid HTTP/0.9 request");
    }
    std::size_t end = wire.size();
    if (end >= 2 && wire.substr(end - 2) == "\r\n") {
        end -= 2;
    } else if (end >= 1 && wire[end - 1] == '\n') {
        --end;
    } else {
        throw ProfileError(ErrorKind::invalid_request, wire.size(),
                           "invalid HTTP/0.9 request");
    }
    const auto address = wire.substr(prefix.size(), end - prefix.size());
    if (address.empty()) {
        throw ProfileError(ErrorKind::invalid_request, prefix.size(),
                           "invalid HTTP/0.9 request");
    }
    for (std::size_t i = 0; i < address.size(); ++i) {
        const auto c = static_cast<unsigned char>(address[i]);
        if (c < 0x21 || c > 0x7e) {
            throw ProfileError(ErrorKind::invalid_request, prefix.size() + i,
                               "invalid HTTP/0.9 request");
        }
    }
    return std::string(address);
}
std::string decode_http09_response(std::string_view body) {
    if (body.empty()) {
        return {};
    }
    for (std::size_t i = 0; i < body.size(); ++i) {
        if (static_cast<unsigned char>(body[i]) > 0x7f) {
            throw ProfileError(ErrorKind::invalid_response, i,
                               "non-ASCII HTTP/0.9 response");
        }
    }
    return std::string(body);
}

class HtmlParser {
public:
    explicit HtmlParser(std::string_view original) {
        std::size_t bad = 0;
        if (!valid_utf8(original, bad)) {
            throw ProfileError(ErrorKind::invalid_utf8, bad,
                               "invalid UTF-8 HTML source");
        }
        source_.reserve(original.size());
        for (std::size_t i = 0; i < original.size(); ++i) {
            if (original[i] == '\r' && i + 1 < original.size() && original[i + 1] == '\n') {
                source_.push_back('\n');
                ++i;
            } else {
                source_.push_back(original[i]);
            }
        }
        latest_.fill(no_index);
    }

    HtmlDocument run() {
        std::size_t cursor = 0;
        while (cursor < source_.size()) {
            if (raw_ != unknown) {
                if (raw_ == plaintext) {
                    append_text(std::string_view(source_).substr(cursor), false);
                    break;
                }
                const auto found = raw_end(cursor, raw_);
                if (!found) {
                    append_text(std::string_view(source_).substr(cursor), false);
                    break;
                }
                append_text(std::string_view(source_).substr(cursor, found->first - cursor), false);
                cursor = found->second;
                const Tag ended = raw_;
                raw_ = unknown;
                handle_tag(ended, true, {});
                continue;
            }
            const auto start = source_.find('<', cursor);
            if (start == std::string::npos) {
                append_text(std::string_view(source_).substr(cursor), true);
                break;
            }
            if (start > cursor) {
                append_text(std::string_view(source_).substr(cursor, start - cursor), true);
            }
            if (source_.compare(start, 4, "<!--") == 0) {
                const auto end = source_.find("-->", start + 4);
                if (end == std::string::npos) {
                    break;
                }
                cursor = end + 3;
                continue;
            }
            ParsedTag token{};
            if (!parse_tag(source_, start, token)) {
                append_text("<", true);
                cursor = start + 1;
                continue;
            }
            cursor = token.end;
            if (token.id == unknown || (token.closing && empty(token.id))) {
                continue;
            }
            handle_tag(token.id, token.closing, token.attrs);
            if (!token.closing && token.id == plaintext) raw_ = plaintext;
            else if (!token.closing && (token.id == xmp || token.id == listing)) {
                raw_ = token.id;
            }
        }
        flush_text();
        return std::move(document_);
    }

private:
    // The published document owns values. These parser-only indexes let us
    // find matching open tags and last children without walking old nodes.
    struct Meta {
        std::size_t previous = no_index;
        std::size_t slot = no_index;
        std::size_t last_child = no_index;
        Tag tag = unknown;
    };
    std::string source_;
    HtmlDocument document_;
    std::vector<Meta> meta_;
    std::vector<std::size_t> stack_;
    std::array<std::size_t, tag_count> latest_{};
    std::string pending_text_;
    Tag raw_ = unknown;
    // A bounded search only accepts `</name>`; a short prefix or a larger
    // tag-name candidate is examined as ordinary literal text instead.

    std::optional<std::pair<std::size_t, std::size_t>> raw_end(std::size_t cursor, Tag tag) const {
        const std::string needle = "</" + std::string(tag_names[tag]) + ">";
        while (true) {
            const auto start = source_.find("</", cursor);
            if (start == std::string::npos) return std::nullopt;
            if (needle.size() <= source_.size() - start &&
                equal_ascii_ci(std::string_view(source_).substr(start, needle.size()), needle)) {
                return std::make_pair(start, start + needle.size());
            }
            cursor = start + 2;
        }
    }

    std::size_t append_node(HtmlNode node, Tag tag = unknown) {
        const std::size_t index = document_.nodes_.size();
        const std::size_t parent = node.parent_index;
        // Keep the parallel metadata arena exactly aligned with the node arena.
        document_.nodes_.push_back(std::move(node));
        meta_.push_back(Meta{latest_[tag], stack_.size(), no_index, tag});
        if (parent != no_index) {
            auto &parent_node = document_.nodes_[parent];
            if (parent_node.first_child_index == no_index) {
                parent_node.first_child_index = index;
            } else {
                document_.nodes_[meta_[parent].last_child].next_sibling_index = index;
            }
            meta_[parent].last_child = index;
        } else {
            if (root_last_ != no_index) {
                document_.nodes_[root_last_].next_sibling_index = index;
            }
            root_last_ = index;
            ++document_.root_count_;
        }
        if (tag != unknown && !empty(tag)) {
            latest_[tag] = index;
        }
        return index;
    }
    std::size_t root_last_ = no_index;

    void append_text(std::string_view bytes, bool entities_on) {
        if (bytes.empty()) {
            return;
        }
        if (entities_on) {
            pending_text_ += decode(bytes);
        } else {
            pending_text_.append(bytes.data(), bytes.size());
        }
    }
    void flush_text() {
        if (pending_text_.empty()) {
            return;
        }
        const std::size_t parent = stack_.empty() ? no_index : stack_.back();
        const std::size_t last = parent == no_index ? root_last_ : meta_[parent].last_child;
        if (last != no_index && document_.nodes_[last].kind == NodeKind::text) {
            document_.nodes_[last].text += pending_text_;
        } else {
            HtmlNode node;
            node.kind = NodeKind::text;
            node.parent_index = parent;
            node.text = std::move(pending_text_);
            pending_text_.clear();
            append_node(std::move(node));
        }
        pending_text_.clear();
    }

    std::size_t matching(Tag tag, Tag boundary1 = unknown, Tag boundary2 = unknown, Tag boundary3 = unknown) const {
        const auto candidate = latest_[tag];
        if (candidate == no_index) {
            return no_index;
        }
        // Sentinels are not positions: choose only present scopes before
        // comparing stack slots. This is the implied-end scope invariant.
        std::size_t boundary = no_index;
        for (const Tag b : {boundary1, boundary2, boundary3}) {
            if (b != unknown && latest_[b] != no_index &&
                (boundary == no_index || meta_[latest_[b]].slot > meta_[boundary].slot)) {
                boundary = latest_[b];
            }
        }
        if (boundary != no_index && meta_[candidate].slot <= meta_[boundary].slot) {
            return no_index;
        }
        return meta_[candidate].slot;
    }
    void close_to(std::size_t slot) {
        while (stack_.size() > slot) {
            const auto index = stack_.back();
            stack_.pop_back();
            latest_[meta_[index].tag] = meta_[index].previous;
        }
    }

    std::vector<HtmlAttribute> anchor_attrs(std::string_view attrs) {
        std::vector<HtmlAttribute> result;
        std::array<bool, 3> seen{};
        std::size_t cursor = 0;
        while (cursor < attrs.size()) {
            ParsedAttribute parsed{};
            if (!parse_attribute(attrs, cursor, parsed)) break;
            const auto name = parsed.name;
            std::size_t slot = 3;
            if (equal_ascii_ci(name, "href")) slot = 0;
            else if (equal_ascii_ci(name, "name")) slot = 1;
            else if (equal_ascii_ci(name, "type")) slot = 2;
            if (slot < 3 && !seen[slot]) {
                seen[slot] = true;
                HtmlAttribute attr;
                attr.name = slot == 0 ? "href" : slot == 1 ? "name" : "type";
                if (parsed.value) attr.value = decode(*parsed.value);
                result.push_back(std::move(attr));
            }
            if (parsed.next <= cursor) break;
            cursor = parsed.next;
        }
        return result;
    }

    void handle_tag(Tag tag, bool closing, std::string_view attrs) {
        std::size_t close_slot = no_index;
        if (closing) close_slot = matching(tag);
        else if (tag == li) close_slot = matching(li, ul, menu, dir);
        else if (tag == dt || tag == dd) {
            const auto a = matching(dt, dl); const auto b = matching(dd, dl);
            close_slot = a == no_index ? b : b == no_index ? a : std::max(a, b);
        } else if (heading(tag)) {
            for (Tag h = h1; h <= h6; h = static_cast<Tag>(h + 1)) {
                const auto candidate = matching(h);
                if (candidate != no_index && (close_slot == no_index || candidate > close_slot)) close_slot = candidate;
            }
        }
        if (closing && close_slot == no_index) return;
        flush_text();
        if (close_slot != no_index) close_to(close_slot);
        if (closing) return;
        HtmlNode node; node.kind = NodeKind::element; node.parent_index = stack_.empty() ? no_index : stack_.back();
        node.tag = std::string(tag_names[tag]);
        auto attributes = tag == anchor ? anchor_attrs(attrs) : std::vector<HtmlAttribute>{};
        if (!attributes.empty()) {
            node.first_attribute_index = document_.attributes_.size();
            node.attribute_count = attributes.size();
            for (auto &attribute : attributes) document_.attributes_.push_back(std::move(attribute));
        }
        const auto index = append_node(std::move(node), empty(tag) ? unknown : tag);
        if (!empty(tag)) stack_.push_back(index);
    }
};

HtmlDocument parse_html_tags_1992(std::string_view source) { return HtmlParser(source).run(); }

} // namespace coding_adventures::historical_web

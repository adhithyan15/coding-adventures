// format_doc.hpp — Wadler-style document algebra for pretty-printers.
// ====================================================================
//
// A faithful, header-only port of the Rust `format-doc` crate (namespace
// `ca::format_doc`). Build a backend-neutral pretty-printing document (`Doc`)
// from primitives, then realise it into a `LayoutTree` of positioned text
// spans, or flatten to a plain `std::string`.
//
//   text        emit literal text (embedded \n auto-split into hardlines)
//   concat      emit child docs in sequence (flattens, drops nil)
//   group       print flat if it fits, else broken
//   indent      add indentation for broken lines inside
//   line        space when flat, newline when broken
//   softline    empty when flat, newline when broken
//   hardline    always newline (forces the enclosing group to break)
//   if_break    emit `broken` in broken mode, else `flat`
//   annotate    attach metadata to emitted spans without changing layout
//   nil         the empty document
//   join        join docs by a separator
//
// `Doc` values are immutable and cheaply copyable (structural sharing via
// `std::shared_ptr`, mirroring the Rust `Arc`). Rust's `Result`/panic becomes a
// thrown `std::invalid_argument`; the Rust enums become `std::variant`.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef FORMAT_DOC_HPP
#define FORMAT_DOC_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

namespace ca::format_doc {

// ── Annotations ───────────────────────────────────────────────────────────
//
// Rust `enum DocAnnotation { Str(String), Int(i64), Bool(bool), Null }` maps to
// a variant. `Null` needs a distinct tag type so it is not confused with the
// other alternatives.
struct Null {};
inline bool operator==(const Null&, const Null&) noexcept { return true; }
inline bool operator!=(const Null&, const Null&) noexcept { return false; }

using DocAnnotation = std::variant<std::string, std::int64_t, bool, Null>;

// Convenience constructors matching the Rust variant names.
inline DocAnnotation ann_str(std::string s) { return DocAnnotation{std::move(s)}; }
inline DocAnnotation ann_int(std::int64_t v) { return DocAnnotation{v}; }
inline DocAnnotation ann_bool(bool v) { return DocAnnotation{v}; }
inline DocAnnotation ann_null() { return DocAnnotation{Null{}}; }

enum class LineMode { Soft, Normal, Hard };

constexpr std::size_t kDefaultIndentWidth = 2;
constexpr std::size_t kDefaultLineHeight = 1;

// ── Documents ─────────────────────────────────────────────────────────────

namespace detail {
struct Node;  // forward declaration — the shared payload of a Doc
}  // namespace detail

// An immutable pretty-printing document. Copying a `Doc` shares the underlying
// tree (ref-counted), so composing bottom-up is cheap.
class Doc {
   public:
    Doc() = default;
    explicit Doc(std::shared_ptr<const detail::Node> node)
        : node_(std::move(node)) {}
    const detail::Node* raw() const noexcept { return node_.get(); }

   private:
    std::shared_ptr<const detail::Node> node_;
};

namespace detail {
struct Nil {};
struct Text {
    std::string value;
};
struct Concat {
    std::vector<Doc> parts;
};
struct Group {
    Doc content;
};
struct Indent {
    std::size_t levels;
    Doc content;
};
struct LineNode {
    LineMode mode;
};
struct IfBreak {
    Doc broken;
    Doc flat;
};
struct Annotate {
    DocAnnotation annotation;
    Doc content;
};

struct Node {
    std::variant<Nil, Text, Concat, Group, Indent, LineNode, IfBreak, Annotate>
        v;
};

inline Doc make(Node n) {
    return Doc{std::make_shared<const Node>(std::move(n))};
}

// UTF-8 code-point count (matches Rust `chars().count()`).
inline std::size_t visible_width(const std::string& s) {
    std::size_t count = 0;
    for (unsigned char c : s)
        if ((c & 0xC0) != 0x80) ++count;
    return count;
}
}  // namespace detail

// ── Builders ──────────────────────────────────────────────────────────────

inline Doc nil() { return detail::make(detail::Node{detail::Nil{}}); }

inline Doc hardline() {
    return detail::make(detail::Node{detail::LineNode{LineMode::Hard}});
}
inline Doc line() {
    return detail::make(detail::Node{detail::LineNode{LineMode::Normal}});
}
inline Doc softline() {
    return detail::make(detail::Node{detail::LineNode{LineMode::Soft}});
}

inline Doc concat(std::vector<Doc> parts);  // fwd (used by text)

// Emit literal text. Embedded newlines are normalised (CRLF / CR -> LF) and the
// text is split into single-line pieces separated by hardlines, so a span never
// contains a newline. Empty text collapses to `nil`.
inline Doc text(const std::string& value) {
    if (value.empty()) return nil();
    if (value.find('\n') == std::string::npos &&
        value.find('\r') == std::string::npos) {
        return detail::make(detail::Node{detail::Text{value}});
    }
    // Normalise line endings.
    std::string norm;
    norm.reserve(value.size());
    for (std::size_t i = 0; i < value.size(); ++i) {
        if (value[i] == '\r') {
            norm.push_back('\n');
            if (i + 1 < value.size() && value[i + 1] == '\n') ++i;
        } else {
            norm.push_back(value[i]);
        }
    }
    // Split on '\n' into Text / hardline / Text / …
    std::vector<Doc> parts;
    std::size_t start = 0;
    bool first = true;
    for (std::size_t i = 0; i <= norm.size(); ++i) {
        if (i == norm.size() || norm[i] == '\n') {
            if (!first) parts.push_back(hardline());
            first = false;
            if (i > start)
                parts.push_back(detail::make(
                    detail::Node{detail::Text{norm.substr(start, i - start)}}));
            start = i + 1;
        }
    }
    return concat(std::move(parts));
}

// Sequence child docs, flattening nested concats and dropping nils. 0 parts ->
// nil, 1 part -> that part, else a Concat.
inline Doc concat(std::vector<Doc> parts) {
    std::vector<Doc> flat;
    for (Doc& part : parts) {
        const detail::Node* n = part.raw();
        if (n == nullptr || std::holds_alternative<detail::Nil>(n->v)) continue;
        if (const auto* c = std::get_if<detail::Concat>(&n->v)) {
            for (const Doc& child : c->parts) {
                const detail::Node* cn = child.raw();
                if (cn != nullptr && !std::holds_alternative<detail::Nil>(cn->v))
                    flat.push_back(child);
            }
        } else {
            flat.push_back(std::move(part));
        }
    }
    if (flat.empty()) return nil();
    if (flat.size() == 1) return flat.front();
    return detail::make(detail::Node{detail::Concat{std::move(flat)}});
}

// Join `parts` with copies of `separator`.
inline Doc join(const Doc& separator, std::vector<Doc> parts) {
    if (parts.empty()) return nil();
    std::vector<Doc> out;
    out.reserve(parts.size() * 2);
    for (std::size_t i = 0; i < parts.size(); ++i) {
        if (i > 0) out.push_back(separator);
        out.push_back(std::move(parts[i]));
    }
    return concat(std::move(out));
}

inline Doc group(Doc content) {
    return detail::make(detail::Node{detail::Group{std::move(content)}});
}

// Indent inner broken lines by `levels` extra units. 0 levels is a no-op.
inline Doc indent(Doc content, std::size_t levels) {
    if (levels == 0) return content;
    return detail::make(
        detail::Node{detail::Indent{levels, std::move(content)}});
}

inline Doc if_break(Doc broken, Doc flat) {
    return detail::make(
        detail::Node{detail::IfBreak{std::move(broken), std::move(flat)}});
}

inline Doc annotate(DocAnnotation annotation, Doc content) {
    return detail::make(detail::Node{
        detail::Annotate{std::move(annotation), std::move(content)}});
}

// ── Layout ────────────────────────────────────────────────────────────────

struct LayoutSpan {
    std::size_t column;
    std::string text;
    std::vector<DocAnnotation> annotations;
};

struct LayoutLine {
    std::size_t row;
    std::size_t indent_columns;
    std::size_t width;
    std::vector<LayoutSpan> spans;
};

struct LayoutOptions {
    std::size_t print_width = 80;  // must be > 0
    std::size_t indent_width = kDefaultIndentWidth;
    std::size_t line_height = kDefaultLineHeight;
};

struct LayoutTree {
    std::size_t print_width;
    std::size_t indent_width;
    std::size_t line_height;
    std::size_t width;   // max column reached
    std::size_t height;  // lines.size() * line_height
    std::vector<LayoutLine> lines;
};

namespace detail {

enum class Mode { Flat, Break };

struct Command {
    std::size_t indent_levels;
    Mode mode;
    std::vector<DocAnnotation> annotations;
    const Node* doc;
};

// Look-ahead ignores annotations, so it carries a lighter command.
struct FitCommand {
    std::size_t indent_levels;
    Mode mode;
    const Node* doc;
};

inline void push_text(LayoutLine& line, std::size_t& column,
                      std::size_t& max_column, const std::string& value,
                      const std::vector<DocAnnotation>& anns) {
    if (value.empty()) return;
    const std::size_t w = visible_width(value);
    if (!line.spans.empty()) {
        LayoutSpan& last = line.spans.back();
        if (last.annotations == anns &&
            last.column + visible_width(last.text) == column) {
            last.text += value;
            column += w;
            max_column = std::max(max_column, column);
            return;
        }
    }
    line.spans.push_back(LayoutSpan{column, value, anns});
    column += w;
    max_column = std::max(max_column, column);
}

// Can the remaining content stay on the current line, starting in flat mode?
// Borrows the parent `stack` rather than cloning it, so the whole layout is
// O(work) rather than O(depth^2).
inline bool fits(long budget, const std::vector<Command>& stack, FitCommand next,
                 std::vector<FitCommand>& pending) {
    pending.clear();
    pending.push_back(next);
    std::size_t stack_idx = stack.size();

    while (budget >= 0) {
        FitCommand cmd;
        if (!pending.empty()) {
            cmd = pending.back();
            pending.pop_back();
        } else if (stack_idx == 0) {
            return true;
        } else {
            --stack_idx;
            const Command& c = stack[stack_idx];
            cmd = FitCommand{c.indent_levels, c.mode, c.doc};
        }
        const Node* n = cmd.doc;
        if (std::holds_alternative<Nil>(n->v)) {
            // nothing
        } else if (const auto* t = std::get_if<Text>(&n->v)) {
            budget -= static_cast<long>(visible_width(t->value));
        } else if (const auto* co = std::get_if<Concat>(&n->v)) {
            for (auto it = co->parts.rbegin(); it != co->parts.rend(); ++it)
                pending.push_back(
                    FitCommand{cmd.indent_levels, cmd.mode, it->raw()});
        } else if (const auto* g = std::get_if<Group>(&n->v)) {
            pending.push_back(
                FitCommand{cmd.indent_levels, Mode::Flat, g->content.raw()});
        } else if (const auto* in = std::get_if<Indent>(&n->v)) {
            pending.push_back(FitCommand{cmd.indent_levels + in->levels,
                                         cmd.mode, in->content.raw()});
        } else if (const auto* ln = std::get_if<LineNode>(&n->v)) {
            if (ln->mode == LineMode::Hard) return false;
            if (ln->mode == LineMode::Normal) {
                if (cmd.mode == Mode::Flat)
                    budget -= 1;
                else
                    return true;
            } else {  // Soft
                if (cmd.mode == Mode::Break) return true;
            }
        } else if (const auto* ib = std::get_if<IfBreak>(&n->v)) {
            const Doc& chosen = cmd.mode == Mode::Flat ? ib->flat : ib->broken;
            pending.push_back(
                FitCommand{cmd.indent_levels, cmd.mode, chosen.raw()});
        } else if (const auto* an = std::get_if<Annotate>(&n->v)) {
            pending.push_back(
                FitCommand{cmd.indent_levels, cmd.mode, an->content.raw()});
        }
    }
    return false;
}

}  // namespace detail

// Realise `doc` into a layout tree. Throws `std::invalid_argument` if
// print_width == 0 (mirrors the Rust `assert!(print_width > 0)`).
inline LayoutTree layout_doc(const Doc& doc, const LayoutOptions& options) {
    using namespace detail;
    if (options.print_width == 0)
        throw std::invalid_argument("print_width > 0");

    std::vector<LayoutLine> lines;
    lines.push_back(LayoutLine{0, 0, 0, {}});
    std::size_t current = 0, column = 0, max_column = 0;

    std::vector<Command> stack;
    std::vector<FitCommand> pending;
    stack.push_back(Command{0, Mode::Break, {}, doc.raw()});

    auto break_line = [&](std::size_t indent_levels) {
        const std::size_t indent_columns = indent_levels * options.indent_width;
        lines.push_back(LayoutLine{lines.size(), indent_columns, 0, {}});
        current = lines.size() - 1;
        column = indent_columns;
        max_column = std::max(max_column, column);
    };

    while (!stack.empty()) {
        Command cmd = std::move(stack.back());
        stack.pop_back();
        const Node* n = cmd.doc;

        if (std::holds_alternative<Nil>(n->v)) {
            // nothing
        } else if (const auto* t = std::get_if<Text>(&n->v)) {
            push_text(lines[current], column, max_column, t->value,
                      cmd.annotations);
        } else if (const auto* co = std::get_if<Concat>(&n->v)) {
            for (auto it = co->parts.rbegin(); it != co->parts.rend(); ++it)
                stack.push_back(Command{cmd.indent_levels, cmd.mode,
                                        cmd.annotations, it->raw()});
        } else if (const auto* g = std::get_if<Group>(&n->v)) {
            long remaining = static_cast<long>(options.print_width) -
                             static_cast<long>(column);
            if (remaining < 0) remaining = 0;
            const bool pick_flat =
                cmd.mode == Mode::Flat ||
                fits(remaining, stack,
                     FitCommand{cmd.indent_levels, Mode::Flat, g->content.raw()},
                     pending);
            stack.push_back(Command{cmd.indent_levels,
                                    pick_flat ? Mode::Flat : Mode::Break,
                                    std::move(cmd.annotations),
                                    g->content.raw()});
        } else if (const auto* in = std::get_if<Indent>(&n->v)) {
            stack.push_back(Command{cmd.indent_levels + in->levels, cmd.mode,
                                    std::move(cmd.annotations),
                                    in->content.raw()});
        } else if (const auto* ln = std::get_if<LineNode>(&n->v)) {
            if (ln->mode == LineMode::Hard) {
                break_line(cmd.indent_levels);
            } else if (ln->mode == LineMode::Normal) {
                if (cmd.mode == Mode::Flat)
                    push_text(lines[current], column, max_column, " ",
                              cmd.annotations);
                else
                    break_line(cmd.indent_levels);
            } else {  // Soft
                if (cmd.mode == Mode::Break) break_line(cmd.indent_levels);
            }
        } else if (const auto* ib = std::get_if<IfBreak>(&n->v)) {
            const Doc& chosen = cmd.mode == Mode::Flat ? ib->flat : ib->broken;
            stack.push_back(Command{cmd.indent_levels, cmd.mode,
                                    std::move(cmd.annotations), chosen.raw()});
        } else if (const auto* an = std::get_if<Annotate>(&n->v)) {
            std::vector<DocAnnotation> next = cmd.annotations;
            next.push_back(an->annotation);
            stack.push_back(Command{cmd.indent_levels, cmd.mode,
                                    std::move(next), an->content.raw()});
        }
    }

    // Finalise per-line widths and tree dimensions.
    for (LayoutLine& l : lines) {
        if (!l.spans.empty()) {
            const LayoutSpan& last = l.spans.back();
            l.width = last.column + visible_width(last.text);
        } else {
            l.width = l.indent_columns;
        }
    }

    LayoutTree tree;
    tree.print_width = options.print_width;
    tree.indent_width = options.indent_width;
    tree.line_height = options.line_height;
    tree.width = max_column;
    tree.height = lines.size() * options.line_height;
    tree.lines = std::move(lines);
    return tree;
}

// Flatten a layout tree to a string: indent spaces + spans, '\n'-joined, no
// trailing newline.
inline std::string render_text(const LayoutTree& tree) {
    std::string out;
    for (std::size_t i = 0; i < tree.lines.size(); ++i) {
        const LayoutLine& line = tree.lines[i];
        if (i > 0) out.push_back('\n');
        out.append(line.indent_columns, ' ');
        std::size_t col = line.indent_columns;
        for (const LayoutSpan& sp : line.spans) {
            while (col < sp.column) {
                out.push_back(' ');
                ++col;
            }
            out += sp.text;
            col += detail::visible_width(sp.text);
        }
    }
    return out;
}

}  // namespace ca::format_doc

#endif  // FORMAT_DOC_HPP

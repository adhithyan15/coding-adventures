// type_declarations.hpp — a language-agnostic type-declaration format,
// header-only in pure ISO C++17 (namespace ca::type_declarations). A faithful
// port of the Rust `type-declarations` crate.
// ===========================================================================
//
// The analogue of TypeScript `.d.ts` files: a parser emits named type
// declarations (record / union / alias), global binding kinds, and a
// typed-mode setting; a generic checker consumes them to infer a `KindDecl`
// for every expression.
//
//   KindDecl = Int | Bool | Nil | Symbol | Str | List | Named(name) |
//              Function(arity) | Any
//
// Each kind maps to an IIR `type_hint` (`to_iir_hint`): Int->"i64",
// Bool->"bool", Str->"str", Function->"closure", everything else->"any".
//
// `TypeDeclarations::resolve` follows alias chains (depth-limited to 32,
// returning Any on a cycle); `union_variants` lists a union's variant names.
// `AnnotatedNode` is the checker's output tree.
//
// DIVERGENCE FROM RUST. `resolve` / `union_variants` return owned values /
// std::optional as in Rust. AnnotatedNode child sub-trees are held by
// std::shared_ptr, so copying a node shares its (build-once, then read)
// children rather than deep-cloning — a documented simplification.
//
// PORTABILITY. Pure ISO C++17 — standard library only, no compiler extensions.
#ifndef CA_TYPE_DECLARATIONS_HPP
#define CA_TYPE_DECLARATIONS_HPP

#include <cstddef>
#include <memory>
#include <optional>
#include <string>
#include <unordered_map>
#include <variant>
#include <vector>

namespace ca {
namespace type_declarations {

// ── KindDecl ─────────────────────────────────────────────────────────────────

enum class KindTag { Int, Bool, Nil, Symbol, Str, List, Named, Function, Any };

struct KindDecl {
    KindTag tag = KindTag::Any;
    std::string named;        // valid iff tag == Named
    std::size_t arity = 0;    // valid iff tag == Function

    static KindDecl Int() { return {KindTag::Int, "", 0}; }
    static KindDecl Bool() { return {KindTag::Bool, "", 0}; }
    static KindDecl Nil() { return {KindTag::Nil, "", 0}; }
    static KindDecl Symbol() { return {KindTag::Symbol, "", 0}; }
    static KindDecl Str() { return {KindTag::Str, "", 0}; }
    static KindDecl List() { return {KindTag::List, "", 0}; }
    static KindDecl Any() { return {KindTag::Any, "", 0}; }
    static KindDecl Named(std::string name) {
        return {KindTag::Named, std::move(name), 0};
    }
    static KindDecl Function(std::size_t arity) {
        return {KindTag::Function, "", arity};
    }

    // IIR type_hint string.
    const char* to_iir_hint() const {
        switch (tag) {
            case KindTag::Int: return "i64";
            case KindTag::Bool: return "bool";
            case KindTag::Str: return "str";
            case KindTag::Function: return "closure";
            default: return "any";  // Nil, Symbol, List, Named, Any
        }
    }
    bool is_concrete_hint() const {
        return std::string(to_iir_hint()) != "any";
    }

    bool operator==(const KindDecl& o) const {
        if (tag != o.tag) return false;
        if (tag == KindTag::Named) return named == o.named;
        if (tag == KindTag::Function) return arity == o.arity;
        return true;
    }
    bool operator!=(const KindDecl& o) const { return !(*this == o); }
};

// ── FieldDecl / VariantDecl / NamedTypeDecl ──────────────────────────────────

struct FieldDecl {
    std::string name;
    KindDecl kind;
};

struct VariantDecl {
    std::string name;
    std::vector<FieldDecl> fields;
};

struct RecordType {
    std::vector<FieldDecl> fields;
};
struct UnionType {
    std::vector<VariantDecl> variants;
};
struct AliasType {
    KindDecl target;
};

using NamedTypeDecl = std::variant<RecordType, UnionType, AliasType>;

enum class TypedModeDecl { Off, Lenient, Strict };

// ── TypeDeclarations ─────────────────────────────────────────────────────────

class TypeDeclarations {
public:
    std::string language;
    std::unordered_map<std::string, NamedTypeDecl> named_types;
    std::unordered_map<std::string, KindDecl> globals;
    std::optional<TypedModeDecl> typed_mode;

    explicit TypeDeclarations(std::string lang) : language(std::move(lang)) {}

    // Resolve through alias chains, depth-limited to 32 (Any on a cycle).
    KindDecl resolve(const KindDecl& kind) const {
        return resolve_depth(kind, 0);
    }

    // Variant names of a named union, or std::nullopt if it is not a union.
    std::optional<std::vector<std::string>> union_variants(
        const std::string& name) const {
        auto it = named_types.find(name);
        if (it == named_types.end()) return std::nullopt;
        if (auto* u = std::get_if<UnionType>(&it->second)) {
            std::vector<std::string> names;
            names.reserve(u->variants.size());
            for (const VariantDecl& v : u->variants) names.push_back(v.name);
            return names;
        }
        return std::nullopt;
    }

private:
    KindDecl resolve_depth(const KindDecl& kind, std::size_t depth) const {
        if (depth > 32) return KindDecl::Any();  // cycle guard
        if (kind.tag == KindTag::Named) {
            auto it = named_types.find(kind.named);
            if (it != named_types.end())
                if (auto* a = std::get_if<AliasType>(&it->second))
                    return resolve_depth(a->target, depth + 1);
            return kind;
        }
        return kind;
    }
};

// ── AnnotatedNode / AnnotatedChild ───────────────────────────────────────────

struct AnnotatedNode;

struct TokenChild {
    std::string text;
    std::size_t line = 0;
    std::size_t column = 0;
};

// A child is either a nested annotated node (shared, build-once) or a token.
struct AnnotatedChild {
    // Exactly one of these is engaged.
    std::shared_ptr<AnnotatedNode> node;  // engaged iff non-null
    TokenChild token;

    bool is_node() const { return static_cast<bool>(node); }
};

struct AnnotatedNode {
    std::string rule_name;
    KindDecl kind;
    std::vector<AnnotatedChild> children;
    std::optional<std::size_t> start_line;
    std::optional<std::size_t> start_column;
    std::optional<std::size_t> end_line;
    std::optional<std::size_t> end_column;

    AnnotatedNode(std::string rule, KindDecl k)
        : rule_name(std::move(rule)), kind(std::move(k)) {}

    void add_child(AnnotatedNode child) {
        AnnotatedChild c;
        c.node = std::make_shared<AnnotatedNode>(std::move(child));
        children.push_back(std::move(c));
    }
    void add_token(std::string text, std::size_t line, std::size_t column) {
        AnnotatedChild c;
        c.token = TokenChild{std::move(text), line, column};
        children.push_back(std::move(c));
    }
    void set_position(std::size_t sl, std::size_t sc, std::size_t el,
                      std::size_t ec) {
        start_line = sl;
        start_column = sc;
        end_line = el;
        end_column = ec;
    }

    // IIR type_hint for the value this node produces.
    const char* iir_hint() const { return kind.to_iir_hint(); }

    // First child node with the given rule name, or nullptr (borrowed).
    const AnnotatedNode* child_node(const std::string& rule) const {
        for (const AnnotatedChild& c : children)
            if (c.is_node() && c.node->rule_name == rule) return c.node.get();
        return nullptr;
    }

    // Immediate child nodes (token leaves excluded), as borrowed pointers.
    std::vector<const AnnotatedNode*> node_children() const {
        std::vector<const AnnotatedNode*> out;
        for (const AnnotatedChild& c : children)
            if (c.is_node()) out.push_back(c.node.get());
        return out;
    }

    // Source position (start_line, start_column), each falling back to 0.
    std::pair<std::size_t, std::size_t> position() const {
        return {start_line.value_or(0), start_column.value_or(0)};
    }
};

}  // namespace type_declarations
}  // namespace ca

#endif  // CA_TYPE_DECLARATIONS_HPP

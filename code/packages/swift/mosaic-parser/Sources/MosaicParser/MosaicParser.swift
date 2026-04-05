// MosaicParser.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MosaicParser — Recursive-descent parser for the Mosaic component language
// ============================================================================
//
// This module consumes a stream of MosaicLexer.Token values and produces an
// ASTNode tree. The grammar being implemented is exactly the one in
// code/grammars/mosaic.grammar:
//
//   file           = { import_decl } component_decl ;
//   import_decl    = IMPORT NAME [ AS NAME ] FROM STRING SEMICOLON ;
//   component_decl = COMPONENT NAME LBRACE { slot_decl } node_tree RBRACE ;
//   slot_decl      = SLOT NAME COLON slot_type [ EQUALS default_value ] SEMICOLON ;
//   slot_type      = KEYWORD | NAME | list_type ;
//   list_type      = KEYWORD(list) LANGLE slot_type RANGLE ;
//   node_tree      = node_element ;
//   node_element   = NAME LBRACE { node_content } RBRACE ;
//   node_content   = property_assignment | child_node | slot_reference
//                  | when_block | each_block ;
//   property_assignment = (NAME|KEYWORD) COLON property_value SEMICOLON ;
//   property_value = slot_ref | STRING | NUMBER | DIMENSION | HEX_COLOR
//                  | KEYWORD | NAME | enum_value ;
//   slot_ref       = AT NAME ;
//   enum_value     = NAME DOT NAME ;
//   slot_reference = AT NAME SEMICOLON ;
//   when_block     = WHEN AT NAME LBRACE { node_content } RBRACE ;
//   each_block     = EACH AT NAME AS NAME LBRACE { node_content } RBRACE ;
//
// ASTNode is an indirect enum so that the compiler allows recursive node trees.
//
// ============================================================================

import MosaicLexer

// ============================================================================
// ASTNode — the parse tree representation
// ============================================================================

/// The result of parsing a Mosaic source file.
///
/// Every production rule in the grammar maps to one or more cases here.
/// The `indirect` modifier enables recursive nesting (e.g., child nodes inside
/// node bodies, or list types inside list types).
///
/// Example tree for:
///   component Label {
///     slot text: text;
///     Text { content: @text; }
///   }
///
///   .component(name: "Label",
///     slots: [.slot(name: "text", type: .slotType("text"))],
///     body: .node(type: "Text",
///       properties: [.property(name: "content", value: .slotRef(name: "text"))],
///       children: []))
///
public indirect enum ASTNode: Equatable {
    /// A full component declaration.
    case component(name: String, slots: [ASTNode], body: ASTNode)

    /// A slot declaration: `slot title: text;`
    case slot(name: String, type: ASTNode)

    /// A visual node element: `Row { ... }`
    case node(type: String, properties: [ASTNode], children: [ASTNode])

    /// A property assignment: `padding: 16dp;`
    case property(name: String, value: ASTNode)

    /// A `when @flag { ... }` conditional block
    case whenBlock(slot: String, body: [ASTNode])

    /// An `each @items as item { ... }` iteration block
    case eachBlock(slot: String, item: String, body: [ASTNode])

    /// A slot reference used as a child: `@actions;`
    case slotRef(name: String)

    /// A string, keyword-ident, or enum-member literal value
    case literal(String)

    /// A numeric value with optional unit (nil = pure number, "dp"/"sp"/"%" = dimension)
    case number(Double, String?)

    /// A parsed RGBA color
    case color(Int, Int, Int, Int)

    /// A primitive slot type keyword: text / number / bool / image / color / node
    case slotType(String)

    /// A list<T> slot type
    case listType(ASTNode)

    /// A file-level import declaration
    case importDecl(name: String, alias: String?, path: String)
}

// ============================================================================
// ParseError
// ============================================================================

/// Thrown when the parser encounters a token that does not match the grammar.
public struct ParseError: Error, CustomStringConvertible {
    public let message: String
    public let line: Int
    public let column: Int

    public var description: String { "ParseError at \(line):\(column): \(message)" }
}

// ============================================================================
// Public API
// ============================================================================

/// Parse Mosaic source text into an ASTNode tree.
///
/// This is the primary entry point for the parser. It lexes the source and
/// then performs recursive-descent parsing according to the Mosaic grammar.
///
/// - Parameter source: Contents of a `.mosaic` file.
/// - Returns: An `ASTNode.component(...)` for the single component in the file.
///   If imports are present they are returned inside a synthetic wrapper (currently
///   the parser returns only the component — imports are attached to it via the
///   analyzer stage).
/// - Throws: `LexError` if the source cannot be tokenized; `ParseError` if the
///   token stream does not match the grammar.
///
/// Example:
///
///     let ast = try parse("""
///       component Button {
///         slot label: text;
///         Text { content: @label; }
///       }
///     """)
///
public func parse(_ source: String) throws -> ASTNode {
    let tokens = try tokenize(source)
    var p = Parser(tokens: tokens)
    return try p.parseFile()
}

// ============================================================================
// Parser — recursive-descent implementation
// ============================================================================

private struct Parser {
    private let tokens: [Token]
    private var pos: Int = 0

    init(tokens: [Token]) {
        self.tokens = tokens
    }

    // -------------------------------------------------------------------------
    // file = { import_decl } component_decl
    // -------------------------------------------------------------------------

    mutating func parseFile() throws -> ASTNode {
        var imports: [ASTNode] = []
        while !isAtEnd() && check("IMPORT") {
            imports.append(try parseImportDecl())
        }
        let comp = try parseComponentDecl(imports: imports)
        return comp
    }

    // -------------------------------------------------------------------------
    // import_decl = IMPORT NAME [ AS NAME ] FROM STRING SEMICOLON
    // -------------------------------------------------------------------------

    mutating func parseImportDecl() throws -> ASTNode {
        try expect("IMPORT")
        let name = try expectName()
        var alias: String? = nil
        if checkKeyword("as") {
            advance()
            alias = try expectName()
        }
        try expect("FROM")
        let path = try expectString()
        try expect("SEMICOLON")
        return .importDecl(name: name, alias: alias, path: path)
    }

    // -------------------------------------------------------------------------
    // component_decl = COMPONENT NAME LBRACE { slot_decl } node_tree RBRACE
    // -------------------------------------------------------------------------

    mutating func parseComponentDecl(imports: [ASTNode]) throws -> ASTNode {
        try expect("COMPONENT")
        let name = try expectName()
        try expect("LBRACE")

        var slots: [ASTNode] = []
        while !isAtEnd() && check("SLOT") {
            slots.append(try parseSlotDecl())
        }

        let body = try parseNodeElement()
        try expect("RBRACE")

        return .component(name: name, slots: slots, body: body)
    }

    // -------------------------------------------------------------------------
    // slot_decl = SLOT NAME COLON slot_type [ EQUALS default_value ] SEMICOLON
    // -------------------------------------------------------------------------

    mutating func parseSlotDecl() throws -> ASTNode {
        try expect("SLOT")
        let name = try expectName()
        try expect("COLON")
        let slotType = try parseSlotType()
        // optional default value (ignored for now — just consume it)
        if check("EQUALS") {
            advance()
            _ = try parseDefaultValue()
        }
        try expect("SEMICOLON")
        return .slot(name: name, type: slotType)
    }

    // -------------------------------------------------------------------------
    // slot_type = KEYWORD | NAME | list_type
    // -------------------------------------------------------------------------

    mutating func parseSlotType() throws -> ASTNode {
        if check("KEYWORD") {
            let kw = current()
            if kw.value == "list" {
                return try parseListType()
            }
            advance()
            return .slotType(kw.value)
        }
        if check("NAME") {
            let n = current().value
            advance()
            return .slotType(n)
        }
        throw parseErr("Expected slot type keyword or name")
    }

    // list_type = KEYWORD(list) LANGLE slot_type RANGLE
    mutating func parseListType() throws -> ASTNode {
        try expect("KEYWORD") // "list"
        try expect("LANGLE")
        let inner = try parseSlotType()
        try expect("RANGLE")
        return .listType(inner)
    }

    // default_value = STRING | NUMBER | DIMENSION | HEX_COLOR | KEYWORD
    mutating func parseDefaultValue() throws -> ASTNode {
        if check("STRING")    { let v = current().value; advance(); return .literal(v) }
        if check("DIMENSION") { return try parseNumberOrDimension() }
        if check("NUMBER")    { return try parseNumberOrDimension() }
        if check("HEX_COLOR") { let v = current().value; advance(); return parseHexColor(v) }
        if check("KEYWORD")   { let v = current().value; advance(); return .literal(v) }
        if check("TRUE")      { advance(); return .literal("true") }
        if check("FALSE")     { advance(); return .literal("false") }
        throw parseErr("Expected default value")
    }

    // -------------------------------------------------------------------------
    // node_element = NAME LBRACE { node_content } RBRACE
    // -------------------------------------------------------------------------

    mutating func parseNodeElement() throws -> ASTNode {
        guard check("NAME") else {
            throw parseErr("Expected node type name")
        }
        let typeName = current().value
        advance()
        try expect("LBRACE")

        var properties: [ASTNode] = []
        var children: [ASTNode] = []

        while !isAtEnd() && !check("RBRACE") {
            let (p, c) = try parseNodeContent()
            if let p = p { properties.append(p) }
            if let c = c { children.append(c) }
        }

        try expect("RBRACE")
        return .node(type: typeName, properties: properties, children: children)
    }

    // -------------------------------------------------------------------------
    // node_content = property_assignment | child_node | slot_reference
    //              | when_block | each_block
    //
    // Disambiguation:
    //   AT + NAME + SEMICOLON  → slot_reference
    //   AT + NAME + LBRACE    → (not valid Mosaic, but guarded)
    //   WHEN                  → when_block
    //   EACH                  → each_block
    //   NAME + LBRACE         → child_node (node_element)
    //   NAME/KEYWORD + COLON  → property_assignment
    // -------------------------------------------------------------------------

    mutating func parseNodeContent() throws -> (ASTNode?, ASTNode?) {
        if check("WHEN") {
            return (nil, try parseWhenBlock())
        }
        if check("EACH") {
            return (nil, try parseEachBlock())
        }
        if check("AT") {
            // slot_reference = AT NAME SEMICOLON
            advance() // consume @
            let name = try expectName()
            try expect("SEMICOLON")
            return (nil, .slotRef(name: name))
        }
        // Look-ahead: is next NAME followed by LBRACE → child node?
        if check("NAME") && peek(1)?.type == "LBRACE" {
            let child = try parseNodeElement()
            return (nil, child)
        }
        // property_assignment = (NAME | KEYWORD | structural keywords used as prop names) COLON value SEMICOLON
        return (try parsePropertyAssignment(), nil)
    }

    // -------------------------------------------------------------------------
    // property_assignment = (NAME|KEYWORD) COLON property_value SEMICOLON
    // -------------------------------------------------------------------------

    mutating func parsePropertyAssignment() throws -> ASTNode {
        // Property name can be NAME or any KEYWORD (e.g. "color", "text", "node")
        guard let nameTok = currentOpt(), (nameTok.type == "NAME" || nameTok.type == "KEYWORD") else {
            throw parseErr("Expected property name (NAME or KEYWORD)")
        }
        let name = nameTok.value
        advance()
        try expect("COLON")
        let value = try parsePropertyValue()
        try expect("SEMICOLON")
        return .property(name: name, value: value)
    }

    // -------------------------------------------------------------------------
    // property_value = slot_ref | STRING | NUMBER | DIMENSION | HEX_COLOR
    //                | KEYWORD | NAME | enum_value
    // -------------------------------------------------------------------------

    mutating func parsePropertyValue() throws -> ASTNode {
        // slot_ref = AT NAME
        if check("AT") {
            advance()
            let name = try expectName()
            return .slotRef(name: name)
        }
        if check("STRING") {
            let v = current().value
            advance()
            return .literal(v)
        }
        if check("DIMENSION") || check("NUMBER") {
            return try parseNumberOrDimension()
        }
        if check("HEX_COLOR") {
            let v = current().value
            advance()
            return parseHexColor(v)
        }
        if check("KEYWORD") {
            let v = current().value; advance(); return .literal(v)
        }
        if check("TRUE")  { advance(); return .literal("true") }
        if check("FALSE") { advance(); return .literal("false") }
        // NAME — could be enum_value (NAME DOT NAME) or plain ident
        if check("NAME") {
            let firstName = current().value
            advance()
            if check("DOT") {
                advance()
                let secondName = try expectName()
                return .literal("\(firstName).\(secondName)")
            }
            return .literal(firstName)
        }
        throw parseErr("Expected property value")
    }

    // -------------------------------------------------------------------------
    // when_block = WHEN AT NAME LBRACE { node_content } RBRACE
    // -------------------------------------------------------------------------

    mutating func parseWhenBlock() throws -> ASTNode {
        try expect("WHEN")
        try expect("AT")
        let slot = try expectName()
        try expect("LBRACE")
        var body: [ASTNode] = []
        while !isAtEnd() && !check("RBRACE") {
            let (p, c) = try parseNodeContent()
            if let p = p { body.append(p) }
            if let c = c { body.append(c) }
        }
        try expect("RBRACE")
        return .whenBlock(slot: slot, body: body)
    }

    // -------------------------------------------------------------------------
    // each_block = EACH AT NAME AS NAME LBRACE { node_content } RBRACE
    // -------------------------------------------------------------------------

    mutating func parseEachBlock() throws -> ASTNode {
        try expect("EACH")
        try expect("AT")
        let slot = try expectName()
        try expect("AS")
        let item = try expectName()
        try expect("LBRACE")
        var body: [ASTNode] = []
        while !isAtEnd() && !check("RBRACE") {
            let (p, c) = try parseNodeContent()
            if let p = p { body.append(p) }
            if let c = c { body.append(c) }
        }
        try expect("RBRACE")
        return .eachBlock(slot: slot, item: item, body: body)
    }

    // -------------------------------------------------------------------------
    // Value parsers
    // -------------------------------------------------------------------------

    mutating func parseNumberOrDimension() throws -> ASTNode {
        let tok = current()
        advance()
        if tok.type == "DIMENSION" {
            // Split "16dp" → value=16, unit="dp"
            // Walk the string: consume the numeric prefix, the rest is the unit.
            let raw = tok.value
            var numEnd = raw.startIndex
            if numEnd < raw.endIndex && (raw[numEnd] == "-") {
                numEnd = raw.index(after: numEnd)
            }
            while numEnd < raw.endIndex && (raw[numEnd].isNumber || raw[numEnd] == ".") {
                numEnd = raw.index(after: numEnd)
            }
            let numStr = String(raw[raw.startIndex..<numEnd])
            let unit = String(raw[numEnd...])
            let v = Double(numStr) ?? 0
            return .number(v, unit.isEmpty ? nil : unit)
        }
        return .number(Double(tok.value) ?? 0, nil)
    }

    func parseHexColor(_ raw: String) -> ASTNode {
        let h = String(raw.dropFirst()) // strip #
        var r = 0, g = 0, b = 0, a = 255
        switch h.count {
        case 3:
            r = hexByte(String(repeating: String(h[h.startIndex]), count: 2))
            let i1 = h.index(h.startIndex, offsetBy: 1)
            let i2 = h.index(h.startIndex, offsetBy: 2)
            g = hexByte(String(repeating: String(h[i1]), count: 2))
            b = hexByte(String(repeating: String(h[i2]), count: 2))
        case 6:
            r = hexByte(String(h.prefix(2)))
            g = hexByte(String(h.dropFirst(2).prefix(2)))
            b = hexByte(String(h.dropFirst(4).prefix(2)))
        case 8:
            r = hexByte(String(h.prefix(2)))
            g = hexByte(String(h.dropFirst(2).prefix(2)))
            b = hexByte(String(h.dropFirst(4).prefix(2)))
            a = hexByte(String(h.dropFirst(6).prefix(2)))
        default: break
        }
        return .color(r, g, b, a)
    }

    func hexByte(_ s: String) -> Int { Int(s, radix: 16) ?? 0 }

    // -------------------------------------------------------------------------
    // Token navigation utilities
    // -------------------------------------------------------------------------

    func isAtEnd() -> Bool { pos >= tokens.count }
    func current() -> Token { tokens[pos] }
    func currentOpt() -> Token? { isAtEnd() ? nil : tokens[pos] }

    func check(_ type: String) -> Bool {
        guard !isAtEnd() else { return false }
        return tokens[pos].type == type
    }

    /// Check if current token is KEYWORD with given value, or the structural keyword with that name.
    func checkKeyword(_ value: String) -> Bool {
        guard !isAtEnd() else { return false }
        let t = tokens[pos]
        return (t.type == "KEYWORD" || t.type == "AS") && t.value == value
    }

    func peek(_ offset: Int) -> Token? {
        let i = pos + offset
        return i < tokens.count ? tokens[i] : nil
    }

    mutating func advance() { if !isAtEnd() { pos += 1 } }

    @discardableResult
    mutating func expect(_ type: String) throws -> Token {
        guard !isAtEnd() else {
            throw ParseError(message: "Expected \(type) but reached end of input", line: 0, column: 0)
        }
        let t = tokens[pos]
        guard t.type == type else {
            throw ParseError(
                message: "Expected \(type) but got \(t.type)(\(t.value))",
                line: t.line, column: t.column
            )
        }
        advance()
        return t
    }

    mutating func expectName() throws -> String {
        guard !isAtEnd() else {
            throw ParseError(message: "Expected NAME but reached end of input", line: 0, column: 0)
        }
        let t = tokens[pos]
        // Accept NAME tokens and also KEYWORD tokens used as names (e.g. slot text: text)
        guard t.type == "NAME" || t.type == "KEYWORD" else {
            throw ParseError(
                message: "Expected NAME but got \(t.type)(\(t.value))",
                line: t.line, column: t.column
            )
        }
        advance()
        return t.value
    }

    mutating func expectString() throws -> String {
        let t = try expect("STRING")
        // Strip surrounding quotes
        var v = t.value
        if v.hasPrefix("\"") { v = String(v.dropFirst()) }
        if v.hasSuffix("\"") { v = String(v.dropLast()) }
        return v
    }

    func parseErr(_ msg: String) -> ParseError {
        let t = isAtEnd() ? nil : tokens[pos]
        return ParseError(
            message: msg + (t.map { " (got \($0.type)(\($0.value)))" } ?? " (at end of input)"),
            line: t?.line ?? 0, column: t?.column ?? 0
        )
    }
}

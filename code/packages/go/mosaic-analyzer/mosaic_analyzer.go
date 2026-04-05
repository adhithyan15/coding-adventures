// Package mosaicanalyzer walks a Mosaic AST and produces a typed MosaicIR.
//
// The analyzer is the third stage of the Mosaic compiler pipeline:
//
//	Source text → Lexer → Tokens → Parser → ASTNode → Analyzer → MosaicIR
//
// What the Analyzer Does
// ----------------------
//
// The AST produced by the parser is a faithful, unvalidated representation of
// the source text. Every token is preserved, including keywords, semicolons,
// and braces. The analyzer's job is to:
//
//  1. Strip syntax noise — remove keyword/semicolon/brace tokens.
//  2. Resolve types — convert keyword strings ("text", "bool") to typed MosaicType values.
//  3. Normalize values — parse "16dp" → {Kind: "dimension", NumValue: 16, Unit: "dp"}.
//  4. Determine required/optional — slots with defaults are optional; without are required.
//  5. Identify primitives — classify nodes as primitive (Row, Column, Text, etc.) or component.
//
// Usage:
//
//	ir, err := mosaicanalyzer.Analyze(`
//	  component Label {
//	    slot text: text;
//	    Text { content: @text; }
//	  }
//	`)
//	fmt.Println(ir.Name)           // "Label"
//	fmt.Println(ir.Slots[0].Type)  // MosaicType{Kind: "text"}
package mosaicanalyzer

import (
	"fmt"
	"regexp"
	"strconv"

	"github.com/adhithyan15/coding-adventures/code/packages/go/lexer"
	mosaicparser "github.com/adhithyan15/coding-adventures/code/packages/go/mosaic-parser"
	"github.com/adhithyan15/coding-adventures/code/packages/go/parser"
)

// ============================================================================
// IR Types
// ============================================================================

// MosaicIR is the root of the intermediate representation.
// A .mosaic file always declares exactly one component.
type MosaicIR struct {
	// Component is the single component declared in this .mosaic file.
	Component *MosaicComponent

	// Imports holds all import declarations at the top of the file.
	Imports []MosaicImport
}

// MosaicComponent is the unit of UI composition.
// It has a name, typed slots (its data API), and a root node tree.
type MosaicComponent struct {
	// Name is the PascalCase component name, e.g., "ProfileCard".
	Name string

	// Slots is the ordered list of typed slot declarations.
	Slots []MosaicSlot

	// Tree is the root node of the visual hierarchy.
	Tree *MosaicNode
}

// MosaicImport represents an `import X from "..."` declaration.
type MosaicImport struct {
	// Name is the exported component name (the X in `import X from ...`).
	Name string

	// Alias is the optional local alias (Y in `import X as Y from ...`).
	Alias string

	// Path is the relative or absolute path to the .mosaic source file.
	Path string
}

// MosaicSlot is a typed data slot — the "props API" of a Mosaic component.
// Slots are the only way data enters a Mosaic component.
type MosaicSlot struct {
	// Name is the slot name, kebab-case by convention (e.g., "avatar-url").
	Name string

	// Type describes what kind of data this slot accepts.
	Type MosaicType

	// DefaultValue is present only when the slot declaration includes `= value`.
	// nil means no default.
	DefaultValue *MosaicValue

	// Required is true when the slot has no default value.
	Required bool
}

// MosaicType describes the type of a slot or list element.
//
// Kind values:
//   - "text", "number", "bool", "image", "color", "node" — primitives
//   - "component" — a named component type (Name field set)
//   - "list" — a parameterized list (Element field set)
type MosaicType struct {
	// Kind is the type discriminant.
	Kind string

	// Name is set for "component" kind: the referenced component name.
	Name string

	// Element is set for "list" kind: the element type.
	Element *MosaicType
}

// MosaicNode is a visual node in the component tree.
// Nodes correspond to platform-native elements.
type MosaicNode struct {
	// Tag is the element type name, e.g., "Row", "Column", "Text", "Button".
	Tag string

	// IsPrimitive is true for built-in layout/display elements.
	// Primitives: Row, Column, Box, Stack, Text, Image, Icon, Spacer, Divider, Scroll.
	IsPrimitive bool

	// Properties are the name:value pairs on this node.
	Properties []MosaicProperty

	// Children are the direct children: nodes, slot refs, when/each blocks.
	Children []MosaicChild
}

// MosaicProperty is a single property assignment (name: value) on a node.
type MosaicProperty struct {
	// Name is the property name, kebab-case (e.g., "corner-radius").
	Name string

	// Value is the property's value.
	Value MosaicValue
}

// MosaicValue is a property value or slot default value.
//
// Kind values:
//   - "slot_ref" — a @slotName reference (SlotName field set)
//   - "string" — a string literal (StrValue field set)
//   - "number" — a numeric literal (NumValue field set)
//   - "dimension" — a number+unit like 16dp (NumValue + Unit set)
//   - "color_hex" — a hex color like #2563eb (StrValue field set)
//   - "bool" — true/false (BoolValue field set)
//   - "ident" — a bare identifier used as a value, e.g., "center"
//   - "enum" — a dotted value like "align.center" (StrValue="align", Unit="center")
type MosaicValue struct {
	Kind      string
	SlotName  string  // for slot_ref
	StrValue  string  // for string, color_hex, ident; namespace for enum
	NumValue  float64 // for number, dimension
	Unit      string  // for dimension; member for enum
	BoolValue bool    // for bool
}

// MosaicChild is one child of a node.
//
// Kind values:
//   - "node" — a nested node element (Node field set)
//   - "slot_ref" — a slot used as a child like `@header;` (SlotName field set)
//   - "when" — conditional subtree (SlotName + Body set)
//   - "each" — iterating subtree (SlotName + ItemName + Body set)
type MosaicChild struct {
	Kind     string
	Node     *MosaicNode  // for "node"
	SlotName string       // for "slot_ref", "when", "each"
	ItemName string       // for "each": the loop variable name
	Body     []*MosaicNode // for "when", "each": the child nodes
}

// ============================================================================
// Primitive Node Registry
// ============================================================================

// primitiveNodes is the set of built-in layout and display elements.
// When a node's tag name is in this set, IsPrimitive is true.
var primitiveNodes = map[string]bool{
	"Row": true, "Column": true, "Box": true, "Stack": true,
	"Text": true, "Image": true, "Icon": true,
	"Spacer": true, "Divider": true, "Scroll": true,
}

// ============================================================================
// Errors
// ============================================================================

// AnalysisError is returned when the analyzer encounters a structural problem.
// These indicate either a parser bug or a malformed AST.
type AnalysisError struct {
	Message string
}

func (e *AnalysisError) Error() string {
	return fmt.Sprintf("analysis error: %s", e.Message)
}

func analysisErr(msg string) error {
	return &AnalysisError{Message: msg}
}

// ============================================================================
// Public API
// ============================================================================

// Analyze parses and analyzes Mosaic source text, returning a typed MosaicIR.
//
// This is the main entry point. It parses the source, then walks the resulting
// AST to produce a validated intermediate representation.
//
// Returns an error if the source is syntactically invalid or the AST has
// structural problems.
func Analyze(source string) (*MosaicIR, error) {
	ast, err := mosaicparser.Parse(source)
	if err != nil {
		return nil, err
	}
	return AnalyzeAST(ast)
}

// AnalyzeAST analyzes a pre-parsed ASTNode and returns a typed MosaicIR.
//
// Use this variant when you already have an AST and want to avoid re-parsing.
// The ast parameter must have RuleName == "file".
func AnalyzeAST(ast *parser.ASTNode) (*MosaicIR, error) {
	return analyzeFile(ast)
}

// ============================================================================
// File-level Analysis
// ============================================================================

func analyzeFile(ast *parser.ASTNode) (*MosaicIR, error) {
	if ast.RuleName != "file" {
		return nil, analysisErr(fmt.Sprintf("expected root rule 'file', got %q", ast.RuleName))
	}

	var imports []MosaicImport
	var componentDecl *parser.ASTNode

	for _, child := range ast.Children {
		node, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if node.RuleName == "import_decl" {
			imp, err := analyzeImport(node)
			if err != nil {
				return nil, err
			}
			imports = append(imports, imp)
		} else if node.RuleName == "component_decl" {
			componentDecl = node
		}
	}

	if componentDecl == nil {
		return nil, analysisErr("no component declaration found in file")
	}

	component, err := analyzeComponent(componentDecl)
	if err != nil {
		return nil, err
	}

	return &MosaicIR{Component: component, Imports: imports}, nil
}

// ============================================================================
// Import Analysis
// ============================================================================

func analyzeImport(node *parser.ASTNode) (MosaicImport, error) {
	// import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON
	// Tokens: "import" NAME [optional: "as" NAME] "from" STRING ";"
	names := directTokenValues(node, "NAME")
	strings_ := directTokenValues(node, "STRING")

	if len(names) == 0 {
		return MosaicImport{}, analysisErr("import_decl missing component name")
	}
	if len(strings_) == 0 {
		return MosaicImport{}, analysisErr("import_decl missing path")
	}

	imp := MosaicImport{
		Name: names[0],
		Path: strings_[0],
	}
	if len(names) >= 2 {
		imp.Alias = names[1]
	}
	return imp, nil
}

// ============================================================================
// Component Analysis
// ============================================================================

func analyzeComponent(node *parser.ASTNode) (*MosaicComponent, error) {
	// component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE
	names := directTokenValues(node, "NAME")
	if len(names) == 0 {
		return nil, analysisErr("component_decl missing name")
	}

	name := names[0]
	var slots []MosaicSlot
	var treeNode *parser.ASTNode

	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if childNode.RuleName == "slot_decl" {
			slot, err := analyzeSlot(childNode)
			if err != nil {
				return nil, err
			}
			slots = append(slots, slot)
		} else if childNode.RuleName == "node_tree" {
			treeNode = childNode
		}
	}

	if treeNode == nil {
		return nil, analysisErr(fmt.Sprintf("component %q has no node tree", name))
	}

	tree, err := analyzeNodeTree(treeNode)
	if err != nil {
		return nil, err
	}

	return &MosaicComponent{Name: name, Slots: slots, Tree: tree}, nil
}

// ============================================================================
// Slot Analysis
// ============================================================================

func analyzeSlot(node *parser.ASTNode) (MosaicSlot, error) {
	// slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON
	names := directTokenValues(node, "NAME")
	if len(names) == 0 {
		return MosaicSlot{}, analysisErr("slot_decl missing name")
	}

	name := names[0]
	slotTypeNode := findChildByRule(node, "slot_type")
	if slotTypeNode == nil {
		return MosaicSlot{}, analysisErr(fmt.Sprintf("slot %q missing type", name))
	}

	slotType, err := analyzeSlotType(slotTypeNode)
	if err != nil {
		return MosaicSlot{}, err
	}

	var defaultValue *MosaicValue
	defaultNode := findChildByRule(node, "default_value")
	if defaultNode != nil {
		val, err := analyzeDefaultValue(defaultNode)
		if err != nil {
			return MosaicSlot{}, err
		}
		defaultValue = &val
	}

	return MosaicSlot{
		Name:         name,
		Type:         slotType,
		DefaultValue: defaultValue,
		Required:     defaultValue == nil,
	}, nil
}

func analyzeSlotType(node *parser.ASTNode) (MosaicType, error) {
	// slot_type = list_type | KEYWORD | NAME
	listTypeNode := findChildByRule(node, "list_type")
	if listTypeNode != nil {
		return analyzeListType(listTypeNode)
	}

	// KEYWORD or NAME directly under slot_type
	kw := firstDirectTokenValue(node, "KEYWORD")
	if kw != "" {
		return parsePrimitiveType(kw)
	}

	name := firstDirectTokenValue(node, "NAME")
	if name != "" {
		return MosaicType{Kind: "component", Name: name}, nil
	}

	// Also look inside token leaves (for the alternation producing direct tokens)
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			if tok.TypeName == "KEYWORD" {
				return parsePrimitiveType(tok.Value)
			}
			if tok.TypeName == "NAME" {
				return MosaicType{Kind: "component", Name: tok.Value}, nil
			}
		}
	}

	return MosaicType{}, analysisErr("slot_type has no recognizable content")
}

func analyzeListType(node *parser.ASTNode) (MosaicType, error) {
	// list_type = KEYWORD LANGLE slot_type RANGLE
	elementTypeNode := findChildByRule(node, "slot_type")
	if elementTypeNode == nil {
		return MosaicType{}, analysisErr("list_type missing element type")
	}
	elementType, err := analyzeSlotType(elementTypeNode)
	if err != nil {
		return MosaicType{}, err
	}
	return MosaicType{Kind: "list", Element: &elementType}, nil
}

func parsePrimitiveType(keyword string) (MosaicType, error) {
	switch keyword {
	case "text":
		return MosaicType{Kind: "text"}, nil
	case "number":
		return MosaicType{Kind: "number"}, nil
	case "bool":
		return MosaicType{Kind: "bool"}, nil
	case "image":
		return MosaicType{Kind: "image"}, nil
	case "color":
		return MosaicType{Kind: "color"}, nil
	case "node":
		return MosaicType{Kind: "node"}, nil
	default:
		return MosaicType{}, analysisErr(fmt.Sprintf("unknown primitive type keyword: %q", keyword))
	}
}

func analyzeDefaultValue(node *parser.ASTNode) (MosaicValue, error) {
	// default_value = STRING | DIMENSION | NUMBER | COLOR_HEX | KEYWORD
	str := firstDirectTokenValue(node, "STRING")
	if str != "" {
		return MosaicValue{Kind: "string", StrValue: str}, nil
	}

	dim := firstDirectTokenValue(node, "DIMENSION")
	if dim != "" {
		return parseDimension(dim)
	}

	num := firstDirectTokenValue(node, "NUMBER")
	if num != "" {
		n, err := strconv.ParseFloat(num, 64)
		if err != nil {
			return MosaicValue{}, err
		}
		return MosaicValue{Kind: "number", NumValue: n}, nil
	}

	color := firstDirectTokenValue(node, "COLOR_HEX")
	if color != "" {
		return MosaicValue{Kind: "color_hex", StrValue: color}, nil
	}

	kw := firstDirectTokenValue(node, "KEYWORD")
	if kw == "true" {
		return MosaicValue{Kind: "bool", BoolValue: true}, nil
	}
	if kw == "false" {
		return MosaicValue{Kind: "bool", BoolValue: false}, nil
	}

	// Also check direct token children (alternation may wrap tokens directly)
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			switch tok.TypeName {
			case "STRING":
				return MosaicValue{Kind: "string", StrValue: tok.Value}, nil
			case "DIMENSION":
				return parseDimension(tok.Value)
			case "NUMBER":
				n, err := strconv.ParseFloat(tok.Value, 64)
				if err != nil {
					return MosaicValue{}, err
				}
				return MosaicValue{Kind: "number", NumValue: n}, nil
			case "COLOR_HEX":
				return MosaicValue{Kind: "color_hex", StrValue: tok.Value}, nil
			case "KEYWORD":
				if tok.Value == "true" {
					return MosaicValue{Kind: "bool", BoolValue: true}, nil
				}
				if tok.Value == "false" {
					return MosaicValue{Kind: "bool", BoolValue: false}, nil
				}
			}
		}
	}

	return MosaicValue{}, analysisErr("default_value has no recognizable content")
}

// ============================================================================
// Node Tree Analysis
// ============================================================================

func analyzeNodeTree(node *parser.ASTNode) (*MosaicNode, error) {
	// node_tree = node_element
	elem := findChildByRule(node, "node_element")
	if elem == nil {
		return nil, analysisErr("node_tree missing node_element")
	}
	return analyzeNodeElement(elem)
}

func analyzeNodeElement(node *parser.ASTNode) (*MosaicNode, error) {
	// node_element = NAME LBRACE { node_content } RBRACE
	tag := firstDirectTokenValue(node, "NAME")
	if tag == "" {
		// Also check direct token children
		for _, child := range node.Children {
			if tok, ok := child.(lexer.Token); ok && tok.TypeName == "NAME" {
				tag = tok.Value
				break
			}
		}
	}
	if tag == "" {
		return nil, analysisErr("node_element missing tag name")
	}

	isPrimitive := primitiveNodes[tag]
	var properties []MosaicProperty
	var children []MosaicChild

	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if childNode.RuleName == "node_content" {
			prop, childItem, err := analyzeNodeContent(childNode)
			if err != nil {
				return nil, err
			}
			if prop != nil {
				properties = append(properties, *prop)
			}
			if childItem != nil {
				children = append(children, *childItem)
			}
		}
	}

	return &MosaicNode{Tag: tag, IsPrimitive: isPrimitive, Properties: properties, Children: children}, nil
}

func analyzeNodeContent(node *parser.ASTNode) (*MosaicProperty, *MosaicChild, error) {
	// node_content = property_assignment | child_node | slot_reference | when_block | each_block
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}

		switch childNode.RuleName {
		case "property_assignment":
			prop, err := analyzePropertyAssignment(childNode)
			if err != nil {
				return nil, nil, err
			}
			return &prop, nil, nil

		case "child_node":
			elemNode := findChildByRule(childNode, "node_element")
			if elemNode != nil {
				mosaicNode, err := analyzeNodeElement(elemNode)
				if err != nil {
					return nil, nil, err
				}
				return nil, &MosaicChild{Kind: "node", Node: mosaicNode}, nil
			}

		case "slot_reference":
			// slot_reference = AT NAME SEMICOLON
			name := firstDirectTokenValue(childNode, "NAME")
			if name == "" {
				for _, c := range childNode.Children {
					if tok, ok := c.(lexer.Token); ok && tok.TypeName == "NAME" {
						name = tok.Value
						break
					}
				}
			}
			if name != "" {
				return nil, &MosaicChild{Kind: "slot_ref", SlotName: name}, nil
			}

		case "when_block":
			c, err := analyzeWhenBlock(childNode)
			if err != nil {
				return nil, nil, err
			}
			return nil, &c, nil

		case "each_block":
			c, err := analyzeEachBlock(childNode)
			if err != nil {
				return nil, nil, err
			}
			return nil, &c, nil
		}
	}
	return nil, nil, nil
}

// ============================================================================
// Property Analysis
// ============================================================================

func analyzePropertyAssignment(node *parser.ASTNode) (MosaicProperty, error) {
	// property_assignment = (NAME | KEYWORD) COLON property_value SEMICOLON
	name := firstDirectTokenValue(node, "NAME")
	if name == "" {
		name = firstDirectTokenValue(node, "KEYWORD")
	}
	if name == "" {
		for _, child := range node.Children {
			if tok, ok := child.(lexer.Token); ok {
				if tok.TypeName == "NAME" || tok.TypeName == "KEYWORD" {
					name = tok.Value
					break
				}
			}
		}
	}
	if name == "" {
		return MosaicProperty{}, analysisErr("property_assignment missing name")
	}

	valueNode := findChildByRule(node, "property_value")
	if valueNode == nil {
		return MosaicProperty{}, analysisErr(fmt.Sprintf("property %q missing value", name))
	}

	value, err := analyzePropertyValue(valueNode)
	if err != nil {
		return MosaicProperty{}, err
	}
	return MosaicProperty{Name: name, Value: value}, nil
}

func analyzePropertyValue(node *parser.ASTNode) (MosaicValue, error) {
	// property_value = slot_ref | enum_value | STRING | DIMENSION | NUMBER | COLOR_HEX | KEYWORD | NAME
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		switch childNode.RuleName {
		case "slot_ref":
			// slot_ref = AT NAME
			name := firstDirectTokenValue(childNode, "NAME")
			if name == "" {
				for _, c := range childNode.Children {
					if tok, ok := c.(lexer.Token); ok && tok.TypeName == "NAME" {
						name = tok.Value
						break
					}
				}
			}
			if name != "" {
				return MosaicValue{Kind: "slot_ref", SlotName: name}, nil
			}
		case "enum_value":
			// enum_value = NAME DOT NAME
			names := directTokenValues(childNode, "NAME")
			if len(names) >= 2 {
				return MosaicValue{Kind: "enum", StrValue: names[0], Unit: names[1]}, nil
			}
		}
	}

	// Leaf tokens
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			switch tok.TypeName {
			case "STRING":
				return MosaicValue{Kind: "string", StrValue: tok.Value}, nil
			case "DIMENSION":
				return parseDimension(tok.Value)
			case "NUMBER":
				n, err := strconv.ParseFloat(tok.Value, 64)
				if err != nil {
					return MosaicValue{}, err
				}
				return MosaicValue{Kind: "number", NumValue: n}, nil
			case "COLOR_HEX":
				return MosaicValue{Kind: "color_hex", StrValue: tok.Value}, nil
			case "KEYWORD":
				if tok.Value == "true" {
					return MosaicValue{Kind: "bool", BoolValue: true}, nil
				}
				if tok.Value == "false" {
					return MosaicValue{Kind: "bool", BoolValue: false}, nil
				}
				return MosaicValue{Kind: "ident", StrValue: tok.Value}, nil
			case "NAME":
				return MosaicValue{Kind: "ident", StrValue: tok.Value}, nil
			}
		}
	}

	// Try token values at deeper nesting (when grammar wraps in alternation nodes)
	str := firstDirectTokenValue(node, "STRING")
	if str != "" {
		return MosaicValue{Kind: "string", StrValue: str}, nil
	}
	dim := firstDirectTokenValue(node, "DIMENSION")
	if dim != "" {
		return parseDimension(dim)
	}
	num := firstDirectTokenValue(node, "NUMBER")
	if num != "" {
		n, err := strconv.ParseFloat(num, 64)
		if err != nil {
			return MosaicValue{}, err
		}
		return MosaicValue{Kind: "number", NumValue: n}, nil
	}
	color := firstDirectTokenValue(node, "COLOR_HEX")
	if color != "" {
		return MosaicValue{Kind: "color_hex", StrValue: color}, nil
	}
	kw := firstDirectTokenValue(node, "KEYWORD")
	if kw == "true" {
		return MosaicValue{Kind: "bool", BoolValue: true}, nil
	}
	if kw == "false" {
		return MosaicValue{Kind: "bool", BoolValue: false}, nil
	}
	if kw != "" {
		return MosaicValue{Kind: "ident", StrValue: kw}, nil
	}
	ident := firstDirectTokenValue(node, "NAME")
	if ident != "" {
		return MosaicValue{Kind: "ident", StrValue: ident}, nil
	}

	return MosaicValue{}, analysisErr("property_value has no recognizable content")
}

// ============================================================================
// When / Each Block Analysis
// ============================================================================

func analyzeWhenBlock(node *parser.ASTNode) (MosaicChild, error) {
	// when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE
	slotRefNode := findChildByRule(node, "slot_ref")
	if slotRefNode == nil {
		return MosaicChild{}, analysisErr("when_block missing slot_ref")
	}

	slotName := firstDirectTokenValue(slotRefNode, "NAME")
	if slotName == "" {
		for _, c := range slotRefNode.Children {
			if tok, ok := c.(lexer.Token); ok && tok.TypeName == "NAME" {
				slotName = tok.Value
				break
			}
		}
	}
	if slotName == "" {
		return MosaicChild{}, analysisErr("when_block slot_ref missing name")
	}

	body, err := collectNodeContents(node)
	if err != nil {
		return MosaicChild{}, err
	}

	return MosaicChild{Kind: "when", SlotName: slotName, Body: body}, nil
}

func analyzeEachBlock(node *parser.ASTNode) (MosaicChild, error) {
	// each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE
	slotRefNode := findChildByRule(node, "slot_ref")
	if slotRefNode == nil {
		return MosaicChild{}, analysisErr("each_block missing slot_ref")
	}

	slotName := firstDirectTokenValue(slotRefNode, "NAME")
	if slotName == "" {
		for _, c := range slotRefNode.Children {
			if tok, ok := c.(lexer.Token); ok && tok.TypeName == "NAME" {
				slotName = tok.Value
				break
			}
		}
	}
	if slotName == "" {
		return MosaicChild{}, analysisErr("each_block slot_ref missing name")
	}

	// Find the loop variable: the NAME token after "as" that is not inside slot_ref.
	itemName := findLoopVariable(node, slotRefNode)
	if itemName == "" {
		return MosaicChild{}, analysisErr("each_block missing loop variable")
	}

	body, err := collectNodeContents(node)
	if err != nil {
		return MosaicChild{}, err
	}

	return MosaicChild{Kind: "each", SlotName: slotName, ItemName: itemName, Body: body}, nil
}

// findLoopVariable finds the NAME token after "as" in an each_block, skipping the slot_ref.
func findLoopVariable(eachBlock *parser.ASTNode, slotRef *parser.ASTNode) string {
	afterAs := false
	for _, child := range eachBlock.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			if childNode == slotRef || childNode.RuleName == "slot_ref" {
				continue
			}
		}
		if tok, ok := child.(lexer.Token); ok {
			if tok.TypeName == "KEYWORD" && tok.Value == "as" {
				afterAs = true
				continue
			}
			if afterAs && tok.TypeName == "NAME" {
				return tok.Value
			}
		}
	}
	return ""
}

// collectNodeContents gathers all child MosaicNodes from node_content children.
// Used for when/each body collection.
func collectNodeContents(node *parser.ASTNode) ([]*MosaicNode, error) {
	var result []*MosaicNode
	for _, child := range node.Children {
		childNode, ok := child.(*parser.ASTNode)
		if !ok {
			continue
		}
		if childNode.RuleName == "node_content" {
			_, childItem, err := analyzeNodeContent(childNode)
			if err != nil {
				return nil, err
			}
			if childItem != nil && childItem.Kind == "node" {
				result = append(result, childItem.Node)
			}
		}
	}
	return result, nil
}

// ============================================================================
// Value Parsing Helpers
// ============================================================================

// dimensionRegex splits a DIMENSION token like "16dp" into number + unit.
var dimensionRegex = regexp.MustCompile(`^(-?[0-9]*\.?[0-9]+)([a-zA-Z%]+)$`)

// parseDimension parses a DIMENSION token value like "16dp" or "100%" into a
// structured MosaicValue with Kind="dimension", NumValue set, and Unit set.
func parseDimension(raw string) (MosaicValue, error) {
	m := dimensionRegex.FindStringSubmatch(raw)
	if m == nil {
		return MosaicValue{}, analysisErr(fmt.Sprintf("invalid DIMENSION token: %q", raw))
	}
	n, err := strconv.ParseFloat(m[1], 64)
	if err != nil {
		return MosaicValue{}, err
	}
	return MosaicValue{Kind: "dimension", NumValue: n, Unit: m[2]}, nil
}

// ============================================================================
// AST Traversal Helpers
// ============================================================================

// findChildByRule finds the first direct child ASTNode with the given RuleName.
func findChildByRule(node *parser.ASTNode, ruleName string) *parser.ASTNode {
	for _, child := range node.Children {
		if childNode, ok := child.(*parser.ASTNode); ok {
			if childNode.RuleName == ruleName {
				return childNode
			}
		}
	}
	return nil
}

// directTokenValues collects the values of all direct-child tokens with the given TypeName.
// "Direct" means children of this node only, not recursive.
func directTokenValues(node *parser.ASTNode, typeName string) []string {
	var result []string
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			if tok.TypeName == typeName {
				result = append(result, tok.Value)
			}
		}
	}
	return result
}

// firstDirectTokenValue returns the value of the first direct-child token with
// the given TypeName, or "" if none found.
func firstDirectTokenValue(node *parser.ASTNode, typeName string) string {
	for _, child := range node.Children {
		if tok, ok := child.(lexer.Token); ok {
			if tok.TypeName == typeName {
				return tok.Value
			}
		}
	}
	return ""
}

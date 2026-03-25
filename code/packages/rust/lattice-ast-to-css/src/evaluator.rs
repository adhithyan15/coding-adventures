//! # Expression evaluator — compile-time evaluation of Lattice expressions.
//!
//! Lattice expressions appear in three places in the source:
//!
//! 1. `@if $theme == dark { ... }` — condition for conditional blocks
//! 2. `@for $i from 1 through $count { ... }` — loop bounds
//! 3. `@return $n * 8px;` — function return values
//!
//! The evaluator walks the grammar AST nodes produced by the Lattice parser
//! and computes a [`LatticeValue`] result.
//!
//! # Operator Precedence
//!
//! The grammar encodes operator precedence via nested rules, so the evaluator
//! can simply recurse down the tree without managing a precedence table itself.
//!
//! From tightest to loosest binding:
//!
//! ```text
//! 1. Unary minus:       -$x
//! 2. Multiplication:    $a * $b
//! 3. Addition:          $a + $b,  $a - $b
//! 4. Comparison:        $a == $b, $a != $b, $a > $b, $a >= $b, $a <= $b
//! 5. Logical AND:       $a and $b
//! 6. Logical OR:        $a or $b
//! ```
//!
//! The grammar rules are:
//! ```text
//! lattice_expression   = lattice_or_expr
//! lattice_or_expr      = lattice_and_expr { "or" lattice_and_expr }
//! lattice_and_expr     = lattice_comparison { "and" lattice_comparison }
//! lattice_comparison   = lattice_additive [ comparison_op lattice_additive ]
//! lattice_additive     = lattice_multiplicative { (PLUS|MINUS) lattice_multiplicative }
//! lattice_multiplicative = lattice_unary { STAR lattice_unary }
//! lattice_unary        = MINUS lattice_unary | lattice_primary
//! lattice_primary      = VARIABLE | NUMBER | DIMENSION | ... | LPAREN lattice_expression RPAREN
//! ```
//!
//! # Arithmetic Rules
//!
//! | Operation          | Left          | Right         | Result        |
//! |--------------------|---------------|---------------|---------------|
//! | `+` or `-`         | Number        | Number        | Number        |
//! | `+` or `-`         | Dimension(u)  | Dimension(u)  | Dimension(u)  |
//! | `+` or `-`         | Percentage    | Percentage    | Percentage    |
//! | `*`                | Number        | Number        | Number        |
//! | `*`                | Number        | Dimension     | Dimension     |
//! | `*`                | Dimension     | Number        | Dimension     |
//! | Everything else    | any           | any           | TypeError     |

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use lexer::token::Token;

use crate::errors::LatticeError;
use crate::scope::ScopeChain;
use crate::values::{LatticeValue, token_to_value, hex_to_rgba, rgb_to_hsl, hsl_to_rgb, rgba_to_hex};

// ===========================================================================
// Expression Evaluator
// ===========================================================================

/// Evaluates Lattice expression AST nodes at compile time.
///
/// The evaluator walks the AST produced by the grammar parser's expression
/// rules (`lattice_expression`, `lattice_or_expr`, etc.) and computes a
/// `LatticeValue` result.
///
/// The grammar's nesting of rules already encodes operator precedence, so
/// the evaluator just recursively evaluates each node.
///
/// # Usage
///
/// ```no_run
/// # use coding_adventures_lattice_ast_to_css::evaluator::ExpressionEvaluator;
/// # use coding_adventures_lattice_ast_to_css::scope::ScopeChain;
/// let scope = ScopeChain::new();
/// let evaluator = ExpressionEvaluator::new(&scope);
/// // evaluator.evaluate_node(expr_node)  → Ok(LatticeValue)
/// ```
pub struct ExpressionEvaluator<'a> {
    scope: &'a ScopeChain,
}

impl<'a> ExpressionEvaluator<'a> {
    /// Create a new evaluator using the given scope for variable lookups.
    pub fn new(scope: &'a ScopeChain) -> Self {
        ExpressionEvaluator { scope }
    }

    /// Evaluate an expression AST node, returning its compile-time value.
    ///
    /// Dispatches on the `rule_name` of the node to the appropriate handler.
    /// If the node is a leaf token, converts it directly to a value.
    pub fn evaluate_node(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        match node.rule_name.as_str() {
            "lattice_expression" => self.eval_or_delegation(node),
            "lattice_or_expr" => self.eval_or_expr(node),
            "lattice_and_expr" => self.eval_and_expr(node),
            "lattice_comparison" => self.eval_comparison(node),
            "comparison_op" => self.eval_comparison_op_node(node),
            "lattice_additive" => self.eval_additive(node),
            "lattice_multiplicative" => self.eval_multiplicative(node),
            "lattice_unary" => self.eval_unary(node),
            "lattice_primary" => self.eval_primary(node),
            "value_list" => self.eval_value_list(node),
            _ => {
                // For wrapper rules with a single child, unwrap.
                if node.children.len() == 1 {
                    match &node.children[0] {
                        ASTNodeOrToken::Node(child) => self.evaluate_node(child),
                        ASTNodeOrToken::Token(tok) => Ok(self.evaluate_token(tok)),
                    }
                } else {
                    Ok(LatticeValue::Null)
                }
            }
        }
    }

    /// Evaluate a raw token directly to a LatticeValue.
    fn evaluate_token(&self, token: &Token) -> LatticeValue {
        let type_name = get_token_type_name(token);
        token_to_value(&type_name, &token.value)
    }

    // -----------------------------------------------------------------------
    // Delegation helpers
    // -----------------------------------------------------------------------

    /// Evaluate the first child of a rule that just wraps another rule.
    fn eval_or_delegation(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        if let Some(ASTNodeOrToken::Node(child)) = node.children.first() {
            self.evaluate_node(child)
        } else if let Some(ASTNodeOrToken::Token(tok)) = node.children.first() {
            Ok(self.evaluate_token(tok))
        } else {
            Ok(LatticeValue::Null)
        }
    }

    // -----------------------------------------------------------------------
    // lattice_or_expr = lattice_and_expr { "or" lattice_and_expr }
    // -----------------------------------------------------------------------
    //
    // Short-circuit evaluation: returns the first truthy value, or the last
    // value if none are truthy. This matches JavaScript's `||` semantics.

    fn eval_or_expr(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        let mut iter = node.children.iter().peekable();

        // Evaluate the first operand
        let mut result = match iter.next() {
            Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
            Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
            None => return Ok(LatticeValue::Null),
        };

        while let Some(child) = iter.next() {
            // Check if it's the "or" literal token
            if let ASTNodeOrToken::Token(tok) = child {
                if tok.value == "or" {
                    if result.is_truthy() {
                        // Short-circuit: already truthy, skip the rest
                        return Ok(result);
                    }
                    // Evaluate the right operand
                    if let Some(right_child) = iter.next() {
                        result = match right_child {
                            ASTNodeOrToken::Node(n) => self.evaluate_node(n)?,
                            ASTNodeOrToken::Token(t) => self.evaluate_token(t),
                        };
                    }
                }
            } else if let ASTNodeOrToken::Node(n) = child {
                if !result.is_truthy() {
                    result = self.evaluate_node(n)?;
                }
            }
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // lattice_and_expr = lattice_comparison { "and" lattice_comparison }
    // -----------------------------------------------------------------------
    //
    // Short-circuit evaluation: returns the first falsy value, or the last
    // value if all are truthy. This matches JavaScript's `&&` semantics.

    fn eval_and_expr(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        let mut iter = node.children.iter().peekable();

        let mut result = match iter.next() {
            Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
            Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
            None => return Ok(LatticeValue::Null),
        };

        while let Some(child) = iter.next() {
            if let ASTNodeOrToken::Token(tok) = child {
                if tok.value == "and" {
                    if !result.is_truthy() {
                        // Short-circuit: already falsy, skip the rest
                        return Ok(result);
                    }
                    if let Some(right_child) = iter.next() {
                        result = match right_child {
                            ASTNodeOrToken::Node(n) => self.evaluate_node(n)?,
                            ASTNodeOrToken::Token(t) => self.evaluate_token(t),
                        };
                    }
                }
            } else if let ASTNodeOrToken::Node(n) = child {
                if result.is_truthy() {
                    result = self.evaluate_node(n)?;
                }
            }
        }

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // lattice_comparison = lattice_additive [ comparison_op lattice_additive ]
    // -----------------------------------------------------------------------

    fn eval_comparison(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        let children = &node.children;
        if children.is_empty() {
            return Ok(LatticeValue::Null);
        }

        // First child is the left operand (lattice_additive)
        let left = match &children[0] {
            ASTNodeOrToken::Node(n) => self.evaluate_node(n)?,
            ASTNodeOrToken::Token(t) => self.evaluate_token(t),
        };

        if children.len() == 1 {
            return Ok(left);
        }

        // Second child should be comparison_op, third is right operand
        let op = if let Some(ASTNodeOrToken::Node(op_node)) = children.get(1) {
            if op_node.rule_name == "comparison_op" {
                self.extract_comparison_op(op_node)
            } else {
                return Ok(left);
            }
        } else {
            return Ok(left);
        };

        let right = match children.get(2) {
            Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
            Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
            None => return Ok(left),
        };

        Ok(self.compare(&left, &right, &op))
    }

    /// Extract the comparison operator string from a comparison_op node.
    ///
    /// Returns a normalized operator name suitable for `compare()`:
    /// `"EQUALS_EQUALS"`, `"NOT_EQUALS"`, `"GREATER"`, `"GREATER_EQUALS"`,
    /// or `"LESS_EQUALS"`.
    ///
    /// The `==` token has `type_ = TokenType::EqualsEquals` and no `type_name`,
    /// so `get_token_type_name` returns `"EqualsEquals"` (Debug format). We
    /// normalize that to the grammar-style `"EQUALS_EQUALS"`.
    fn extract_comparison_op(&self, node: &GrammarASTNode) -> String {
        if let Some(ASTNodeOrToken::Token(tok)) = node.children.first() {
            let raw = get_token_type_name(tok);
            // Normalize Debug-format enum names to grammar-style SCREAMING_SNAKE_CASE.
            return match raw.as_str() {
                "EqualsEquals" => "EQUALS_EQUALS".to_string(),
                "Bang" | "NotEquals" => "NOT_EQUALS".to_string(),
                "Greater" => "GREATER".to_string(),
                other => {
                    // type_name is already grammar-style (NOT_EQUALS, GREATER_EQUALS, etc.)
                    other.to_string()
                }
            };
        }
        "EQUALS_EQUALS".to_string()
    }

    fn eval_comparison_op_node(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        // comparison_op is handled inline by eval_comparison
        self.eval_or_delegation(node)
    }

    /// Perform a comparison and return a Bool result.
    ///
    /// Comparison logic:
    /// - For numeric types (Number, Dimension, Percentage), compare numerically.
    /// - For dimension equality, units must also match.
    /// - For all other types, compare via string representation.
    fn compare(&self, left: &LatticeValue, right: &LatticeValue, op: &str) -> LatticeValue {
        // Numeric comparison for matching types
        match (left, right) {
            (LatticeValue::Number(lv), LatticeValue::Number(rv)) => {
                return LatticeValue::Bool(compare_nums(*lv, *rv, op));
            }
            (LatticeValue::Dimension { value: lv, unit: lu },
             LatticeValue::Dimension { value: rv, unit: ru }) => {
                let result = match op {
                    "EQUALS_EQUALS" => lv == rv && lu == ru,
                    "NOT_EQUALS" => lv != rv || lu != ru,
                    "GREATER" => lu == ru && lv > rv,
                    "GREATER_EQUALS" => lu == ru && lv >= rv,
                    "LESS_EQUALS" => lu == ru && lv <= rv,
                    _ => false,
                };
                return LatticeValue::Bool(result);
            }
            (LatticeValue::Percentage(lv), LatticeValue::Percentage(rv)) => {
                return LatticeValue::Bool(compare_nums(*lv, *rv, op));
            }
            _ => {}
        }

        // Fallback: string comparison (for idents, colors, booleans, etc.)
        let lstr = left.to_css_string();
        let rstr = right.to_css_string();
        let result = match op {
            "EQUALS_EQUALS" => lstr == rstr,
            "NOT_EQUALS" => lstr != rstr,
            _ => false,
        };
        LatticeValue::Bool(result)
    }

    // -----------------------------------------------------------------------
    // value_list — produced by variable substitution.
    // When expand_variable_declaration substitutes `$i + 1`, the evaluator
    // receives a value_list AST node whose children are
    // [NUMBER(2), PLUS, NUMBER(1)].  If arithmetic operators are present
    // we delegate to the additive handler; otherwise evaluate the first child.
    // -----------------------------------------------------------------------

    fn eval_value_list(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        if node.children.is_empty() {
            return Ok(LatticeValue::Null);
        }
        if node.children.len() <= 1 {
            return match &node.children[0] {
                ASTNodeOrToken::Node(n) => self.evaluate_node(n),
                ASTNodeOrToken::Token(t) => Ok(self.evaluate_token(t)),
            };
        }
        let has_ops = node.children.iter().any(|c| {
            if let ASTNodeOrToken::Token(t) = c {
                t.value == "+" || t.value == "-" || t.value == "*"
            } else {
                false
            }
        });
        if has_ops {
            return self.eval_additive(node);
        }
        match &node.children[0] {
            ASTNodeOrToken::Node(n) => self.evaluate_node(n),
            ASTNodeOrToken::Token(t) => Ok(self.evaluate_token(t)),
        }
    }

    // -----------------------------------------------------------------------
    // lattice_additive = lattice_multiplicative { (PLUS|MINUS) lattice_multiplicative }
    // -----------------------------------------------------------------------

    fn eval_additive(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        let mut children = node.children.iter();

        let mut result = match children.next() {
            Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
            Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
            None => return Ok(LatticeValue::Null),
        };

        // Children alternate: operand, operator, operand, operator, ...
        while let Some(op_child) = children.next() {
            let op = match op_child {
                ASTNodeOrToken::Token(t) if t.value == "+" || t.value == "-" => t.value.as_str(),
                ASTNodeOrToken::Node(_) => {
                    // Might be more operands in a subtree
                    continue;
                }
                _ => continue,
            };

            let right = match children.next() {
                Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
                Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
                None => break,
            };

            result = if op == "+" {
                self.add(&result, &right)?
            } else {
                self.subtract(&result, &right)?
            };
        }

        Ok(result)
    }

    /// Addition: supports Number+Number, Dimension+Dimension (same unit), Percentage+Percentage.
    fn add(&self, left: &LatticeValue, right: &LatticeValue) -> Result<LatticeValue, LatticeError> {
        match (left, right) {
            (LatticeValue::Number(l), LatticeValue::Number(r)) => {
                Ok(LatticeValue::Number(l + r))
            }
            (LatticeValue::Dimension { value: lv, unit: lu },
             LatticeValue::Dimension { value: rv, unit: ru }) => {
                if lu == ru {
                    Ok(LatticeValue::Dimension { value: lv + rv, unit: lu.clone() })
                } else {
                    Err(LatticeError::type_error("add",
                        &left.to_css_string(), &right.to_css_string(), 0, 0))
                }
            }
            (LatticeValue::Percentage(l), LatticeValue::Percentage(r)) => {
                Ok(LatticeValue::Percentage(l + r))
            }
            (LatticeValue::String(l), LatticeValue::String(r)) => {
                Ok(LatticeValue::String(format!("{}{}", l, r)))
            }
            _ => Err(LatticeError::type_error("add",
                &left.to_css_string(), &right.to_css_string(), 0, 0)),
        }
    }

    /// Subtraction: mirrors addition.
    fn subtract(&self, left: &LatticeValue, right: &LatticeValue) -> Result<LatticeValue, LatticeError> {
        match (left, right) {
            (LatticeValue::Number(l), LatticeValue::Number(r)) => {
                Ok(LatticeValue::Number(l - r))
            }
            (LatticeValue::Dimension { value: lv, unit: lu },
             LatticeValue::Dimension { value: rv, unit: ru }) => {
                if lu == ru {
                    Ok(LatticeValue::Dimension { value: lv - rv, unit: lu.clone() })
                } else {
                    Err(LatticeError::type_error("subtract",
                        &left.to_css_string(), &right.to_css_string(), 0, 0))
                }
            }
            (LatticeValue::Percentage(l), LatticeValue::Percentage(r)) => {
                Ok(LatticeValue::Percentage(l - r))
            }
            _ => Err(LatticeError::type_error("subtract",
                &left.to_css_string(), &right.to_css_string(), 0, 0)),
        }
    }

    // -----------------------------------------------------------------------
    // lattice_multiplicative = lattice_unary { STAR lattice_unary }
    // -----------------------------------------------------------------------

    fn eval_multiplicative(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        let mut children = node.children.iter();

        let mut result = match children.next() {
            Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
            Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
            None => return Ok(LatticeValue::Null),
        };

        while let Some(op_child) = children.next() {
            // Skip the "*" token
            if let ASTNodeOrToken::Token(t) = op_child {
                if t.value == "*" {
                    let right = match children.next() {
                        Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
                        Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
                        None => break,
                    };
                    result = self.multiply(&result, &right)?;
                }
            }
        }

        Ok(result)
    }

    /// Multiplication: Number×Number, Number×Dimension, Dimension×Number,
    /// Number×Percentage, Percentage×Number.
    fn multiply(&self, left: &LatticeValue, right: &LatticeValue) -> Result<LatticeValue, LatticeError> {
        match (left, right) {
            (LatticeValue::Number(l), LatticeValue::Number(r)) => {
                Ok(LatticeValue::Number(l * r))
            }
            (LatticeValue::Number(l), LatticeValue::Dimension { value: rv, unit }) => {
                Ok(LatticeValue::Dimension { value: l * rv, unit: unit.clone() })
            }
            (LatticeValue::Dimension { value: lv, unit }, LatticeValue::Number(r)) => {
                Ok(LatticeValue::Dimension { value: lv * r, unit: unit.clone() })
            }
            (LatticeValue::Number(l), LatticeValue::Percentage(r)) => {
                Ok(LatticeValue::Percentage(l * r))
            }
            (LatticeValue::Percentage(l), LatticeValue::Number(r)) => {
                Ok(LatticeValue::Percentage(l * r))
            }
            _ => Err(LatticeError::type_error("multiply",
                &left.to_css_string(), &right.to_css_string(), 0, 0)),
        }
    }

    // -----------------------------------------------------------------------
    // lattice_unary = MINUS lattice_unary | lattice_primary
    // -----------------------------------------------------------------------

    fn eval_unary(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        let children = &node.children;

        // Check if first child is a MINUS token
        if let Some(ASTNodeOrToken::Token(tok)) = children.first() {
            if tok.value == "-" {
                // Negate the second child
                let operand = match children.get(1) {
                    Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n)?,
                    Some(ASTNodeOrToken::Token(t)) => self.evaluate_token(t),
                    None => LatticeValue::Null,
                };
                return self.negate(&operand);
            }
        }

        // Otherwise, delegate to single child
        match children.first() {
            Some(ASTNodeOrToken::Node(n)) => self.evaluate_node(n),
            Some(ASTNodeOrToken::Token(t)) => Ok(self.evaluate_token(t)),
            None => Ok(LatticeValue::Null),
        }
    }

    /// Negate a numeric value (unary minus).
    fn negate(&self, value: &LatticeValue) -> Result<LatticeValue, LatticeError> {
        match value {
            LatticeValue::Number(n) => Ok(LatticeValue::Number(-n)),
            LatticeValue::Dimension { value: v, unit } => {
                Ok(LatticeValue::Dimension { value: -v, unit: unit.clone() })
            }
            LatticeValue::Percentage(p) => Ok(LatticeValue::Percentage(-p)),
            _ => Err(LatticeError::type_error("negate",
                &value.to_css_string(), "", 0, 0)),
        }
    }

    // -----------------------------------------------------------------------
    // lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE | STRING
    //                 | IDENT | HASH | "true" | "false" | "null"
    //                 | function_call
    //                 | LPAREN lattice_expression RPAREN
    // -----------------------------------------------------------------------

    fn eval_primary(&self, node: &GrammarASTNode) -> Result<LatticeValue, LatticeError> {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let type_name = get_token_type_name(tok);

                    // Parentheses: skip them, the expression inside will be handled
                    if type_name == "LParen" || type_name == "RParen" ||
                       tok.value == "(" || tok.value == ")" {
                        continue;
                    }

                    // Variable: look up in scope
                    if type_name == "VARIABLE" {
                        return self.lookup_variable(&tok.value, tok.line, tok.column);
                    }

                    // All other tokens: convert directly
                    return Ok(self.evaluate_token(tok));
                }
                ASTNodeOrToken::Node(n) => {
                    // Recursively evaluate: could be lattice_expression (paren group),
                    // function_call, or another expression node.
                    return self.evaluate_node(n);
                }
            }
        }

        Ok(LatticeValue::Null)
    }

    /// Look up a variable in the current scope.
    ///
    /// Returns the value if found, or an `UndefinedVariable` error.
    fn lookup_variable(&self, name: &str, _line: usize, _column: usize) -> Result<LatticeValue, LatticeError> {
        match self.scope.get(name) {
            Some(scope_val) => {
                match scope_val.as_lattice_value() {
                    Some(v) => Ok(v.clone()),
                    None => {
                        // Raw CSS text: convert to Ident
                        Ok(LatticeValue::Ident(scope_val.to_css_text()))
                    }
                }
            }
            None => {
                // Variable not found — return Ident(name) for graceful degradation.
                // The transformer will emit an error at a higher level.
                Ok(LatticeValue::Ident(name.to_string()))
            }
        }
    }
}

// ===========================================================================
// Built-in Functions (Lattice v2)
// ===========================================================================
//
// Built-in functions are registered by name and dispatched during function
// call evaluation. They cover four categories:
//
// 1. Color: lighten, darken, saturate, desaturate, adjust-hue, complement,
//           mix, red, green, blue, hue, saturation, lightness
// 2. List:  nth, length, join, append, index
// 3. Map:   map-get, map-keys, map-values, map-has-key, map-merge, map-remove
// 4. Math:  math.div, math.floor, math.ceil, math.round, math.abs,
//           math.min, math.max
// 5. Type:  type-of, unit, unitless, comparable

/// Check if a function name is a Lattice built-in function.
pub fn is_builtin_function(name: &str) -> bool {
    matches!(name,
        "lighten" | "darken" | "saturate" | "desaturate" |
        "adjust-hue" | "complement" | "mix" |
        "red" | "green" | "blue" | "hue" | "saturation" | "lightness" |
        "nth" | "length" | "join" | "append" | "index" |
        "map-get" | "map-keys" | "map-values" | "map-has-key" | "map-merge" | "map-remove" |
        "math.div" | "math.floor" | "math.ceil" | "math.round" | "math.abs" |
        "math.min" | "math.max" |
        "type-of" | "unit" | "unitless" | "comparable" |
        "if"
    )
}

/// Evaluate a built-in function call with the given arguments.
pub fn evaluate_builtin(name: &str, args: &[LatticeValue]) -> Result<LatticeValue, LatticeError> {
    match name {
        // -------------------------------------------------------------------
        // Conditional function
        // -------------------------------------------------------------------
        "if" => {
            if args.len() != 3 {
                return Err(LatticeError::wrong_arity("Function", "if", 3, args.len(), 0, 0));
            }
            if args[0].is_truthy() { Ok(args[1].clone()) } else { Ok(args[2].clone()) }
        }

        // -------------------------------------------------------------------
        // Color functions
        // -------------------------------------------------------------------
        "lighten" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let amount = extract_percentage_or_number(&args[1]);
            let (r, g, b, a) = extract_color(&args[0])?;
            let (h, s, l, _) = rgb_to_hsl(r, g, b, a);
            let new_l = (l + amount).clamp(0.0, 100.0);
            let (nr, ng, nb) = hsl_to_rgb(h, s, new_l);
            Ok(LatticeValue::Color(rgba_to_hex(nr, ng, nb, a)))
        }
        "darken" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let amount = extract_percentage_or_number(&args[1]);
            let (r, g, b, a) = extract_color(&args[0])?;
            let (h, s, l, _) = rgb_to_hsl(r, g, b, a);
            let new_l = (l - amount).clamp(0.0, 100.0);
            let (nr, ng, nb) = hsl_to_rgb(h, s, new_l);
            Ok(LatticeValue::Color(rgba_to_hex(nr, ng, nb, a)))
        }
        "complement" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let (r, g, b, a) = extract_color(&args[0])?;
            let (h, s, l, _) = rgb_to_hsl(r, g, b, a);
            let new_h = (h + 180.0) % 360.0;
            let (nr, ng, nb) = hsl_to_rgb(new_h, s, l);
            Ok(LatticeValue::Color(rgba_to_hex(nr, ng, nb, a)))
        }
        "adjust-hue" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let degrees = extract_number_value(&args[1]);
            let (r, g, b, a) = extract_color(&args[0])?;
            let (h, s, l, _) = rgb_to_hsl(r, g, b, a);
            let new_h = (h + degrees) % 360.0;
            let (nr, ng, nb) = hsl_to_rgb(new_h, s, l);
            Ok(LatticeValue::Color(rgba_to_hex(nr, ng, nb, a)))
        }
        "mix" => {
            if args.len() < 2 || args.len() > 3 {
                return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0));
            }
            let (r1, g1, b1, a1) = extract_color(&args[0])?;
            let (r2, g2, b2, a2) = extract_color(&args[1])?;
            let weight = if args.len() == 3 { extract_percentage_or_number(&args[2]) / 100.0 } else { 0.5 };
            let w = weight.clamp(0.0, 1.0);
            let nr = ((r1 as f64) * w + (r2 as f64) * (1.0 - w)).round() as u8;
            let ng = ((g1 as f64) * w + (g2 as f64) * (1.0 - w)).round() as u8;
            let nb = ((b1 as f64) * w + (b2 as f64) * (1.0 - w)).round() as u8;
            let na = a1 * w + a2 * (1.0 - w);
            Ok(LatticeValue::Color(rgba_to_hex(nr, ng, nb, na)))
        }
        "red" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let (r, _, _, _) = extract_color(&args[0])?;
            Ok(LatticeValue::Number(r as f64))
        }
        "green" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let (_, g, _, _) = extract_color(&args[0])?;
            Ok(LatticeValue::Number(g as f64))
        }
        "blue" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let (_, _, b, _) = extract_color(&args[0])?;
            Ok(LatticeValue::Number(b as f64))
        }
        "hue" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let (r, g, b, a) = extract_color(&args[0])?;
            let (h, _, _, _) = rgb_to_hsl(r, g, b, a);
            Ok(LatticeValue::Dimension { value: h, unit: "deg".to_string() })
        }
        "saturation" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let (r, g, b, a) = extract_color(&args[0])?;
            let (_, s, _, _) = rgb_to_hsl(r, g, b, a);
            Ok(LatticeValue::Percentage(s))
        }
        "lightness" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let (r, g, b, a) = extract_color(&args[0])?;
            let (_, _, l, _) = rgb_to_hsl(r, g, b, a);
            Ok(LatticeValue::Percentage(l))
        }

        // -------------------------------------------------------------------
        // List functions
        // -------------------------------------------------------------------
        "nth" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let list = match &args[0] {
                LatticeValue::List(items) => items.clone(),
                other => vec![other.clone()],
            };
            let n = extract_number_value(&args[1]) as usize;
            if n < 1 || n > list.len() {
                return Err(LatticeError::range_error(
                    format!("Index {} out of bounds for list of length {}", n, list.len()), 0, 0));
            }
            Ok(list[n - 1].clone())
        }
        "length" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let len = match &args[0] {
                LatticeValue::List(items) => items.len(),
                LatticeValue::Map(entries) => entries.len(),
                _ => 1,
            };
            Ok(LatticeValue::Number(len as f64))
        }
        "join" => {
            if args.len() < 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let mut items = match &args[0] {
                LatticeValue::List(items) => items.clone(),
                other => vec![other.clone()],
            };
            match &args[1] {
                LatticeValue::List(items2) => items.extend(items2.clone()),
                other => items.push(other.clone()),
            }
            Ok(LatticeValue::List(items))
        }
        "append" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let mut items = match &args[0] {
                LatticeValue::List(items) => items.clone(),
                other => vec![other.clone()],
            };
            items.push(args[1].clone());
            Ok(LatticeValue::List(items))
        }
        "index" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let list = match &args[0] {
                LatticeValue::List(items) => items.clone(),
                other => vec![other.clone()],
            };
            let target = &args[1];
            for (i, item) in list.iter().enumerate() {
                if item.to_css_string() == target.to_css_string() {
                    return Ok(LatticeValue::Number((i + 1) as f64));
                }
            }
            Ok(LatticeValue::Null)
        }

        // -------------------------------------------------------------------
        // Map functions
        // -------------------------------------------------------------------
        "map-get" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let key = args[1].to_css_string().trim_matches('"').to_string();
            match args[0].map_get(&key) {
                Some(v) => Ok(v.clone()),
                None => Ok(LatticeValue::Null),
            }
        }
        "map-keys" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            match args[0].map_keys() {
                Some(keys) => Ok(LatticeValue::List(keys)),
                None => Err(LatticeError::type_error("map-keys", args[0].type_name(), "", 0, 0)),
            }
        }
        "map-values" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            match args[0].map_values() {
                Some(vals) => Ok(LatticeValue::List(vals)),
                None => Err(LatticeError::type_error("map-values", args[0].type_name(), "", 0, 0)),
            }
        }
        "map-has-key" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let key = args[1].to_css_string().trim_matches('"').to_string();
            Ok(LatticeValue::Bool(args[0].map_has_key(&key)))
        }
        "map-merge" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            if let (LatticeValue::Map(mut m1), LatticeValue::Map(m2)) = (args[0].clone(), args[1].clone()) {
                for (k, v) in m2 {
                    if let Some(pos) = m1.iter().position(|(ek, _)| ek == &k) {
                        m1[pos] = (k, v);
                    } else {
                        m1.push((k, v));
                    }
                }
                Ok(LatticeValue::Map(m1))
            } else {
                Err(LatticeError::type_error("map-merge", args[0].type_name(), args[1].type_name(), 0, 0))
            }
        }
        "map-remove" => {
            if args.len() < 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            if let LatticeValue::Map(m) = &args[0] {
                let keys_to_remove: Vec<String> = args[1..].iter()
                    .map(|a| a.to_css_string().trim_matches('"').to_string())
                    .collect();
                let filtered: Vec<(String, LatticeValue)> = m.iter()
                    .filter(|(k, _)| !keys_to_remove.contains(k))
                    .cloned()
                    .collect();
                Ok(LatticeValue::Map(filtered))
            } else {
                Err(LatticeError::type_error("map-remove", args[0].type_name(), "", 0, 0))
            }
        }

        // -------------------------------------------------------------------
        // Math functions
        // -------------------------------------------------------------------
        "math.div" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let divisor = extract_number_value(&args[1]);
            if divisor == 0.0 { return Err(LatticeError::zero_division(0, 0)); }
            match (&args[0], &args[1]) {
                (LatticeValue::Dimension { value, unit }, LatticeValue::Number(_)) => {
                    Ok(LatticeValue::Dimension { value: value / divisor, unit: unit.clone() })
                }
                (LatticeValue::Number(l), LatticeValue::Number(r)) => {
                    Ok(LatticeValue::Number(l / r))
                }
                (LatticeValue::Percentage(l), LatticeValue::Number(r)) => {
                    Ok(LatticeValue::Percentage(l / r))
                }
                _ => Err(LatticeError::type_error("divide", &args[0].to_css_string(), &args[1].to_css_string(), 0, 0)),
            }
        }
        "math.floor" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            apply_math_unary(&args[0], f64::floor)
        }
        "math.ceil" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            apply_math_unary(&args[0], f64::ceil)
        }
        "math.round" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            apply_math_unary(&args[0], f64::round)
        }
        "math.abs" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            apply_math_unary(&args[0], f64::abs)
        }
        "math.min" => {
            if args.is_empty() { return Err(LatticeError::wrong_arity("Function", name, 1, 0, 0, 0)); }
            let mut min = extract_number_value(&args[0]);
            for arg in &args[1..] { min = min.min(extract_number_value(arg)); }
            Ok(LatticeValue::Number(min))
        }
        "math.max" => {
            if args.is_empty() { return Err(LatticeError::wrong_arity("Function", name, 1, 0, 0, 0)); }
            let mut max = extract_number_value(&args[0]);
            for arg in &args[1..] { max = max.max(extract_number_value(arg)); }
            Ok(LatticeValue::Number(max))
        }

        // -------------------------------------------------------------------
        // Type introspection functions
        // -------------------------------------------------------------------
        "type-of" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            Ok(LatticeValue::Ident(args[0].type_name().to_string()))
        }
        "unit" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let u = match &args[0] {
                LatticeValue::Dimension { unit, .. } => unit.clone(),
                LatticeValue::Percentage(_) => "%".to_string(),
                _ => String::new(),
            };
            Ok(LatticeValue::String(u))
        }
        "unitless" => {
            if args.len() != 1 { return Err(LatticeError::wrong_arity("Function", name, 1, args.len(), 0, 0)); }
            let is_unitless = matches!(&args[0], LatticeValue::Number(_));
            Ok(LatticeValue::Bool(is_unitless))
        }
        "comparable" => {
            if args.len() != 2 { return Err(LatticeError::wrong_arity("Function", name, 2, args.len(), 0, 0)); }
            let comparable = match (&args[0], &args[1]) {
                (LatticeValue::Number(_), LatticeValue::Number(_)) => true,
                (LatticeValue::Dimension { unit: u1, .. }, LatticeValue::Dimension { unit: u2, .. }) => u1 == u2,
                (LatticeValue::Percentage(_), LatticeValue::Percentage(_)) => true,
                _ => false,
            };
            Ok(LatticeValue::Bool(comparable))
        }

        _ => Err(LatticeError::undefined_function(name, 0, 0)),
    }
}

/// Extract a numeric value from a LatticeValue (Number, Dimension, Percentage).
fn extract_number_value(val: &LatticeValue) -> f64 {
    match val {
        LatticeValue::Number(n) => *n,
        LatticeValue::Dimension { value, .. } => *value,
        LatticeValue::Percentage(p) => *p,
        _ => 0.0,
    }
}

/// Extract a percentage value: Percentage(50) → 50, Number(20) → 20.
fn extract_percentage_or_number(val: &LatticeValue) -> f64 {
    match val {
        LatticeValue::Percentage(p) => *p,
        LatticeValue::Number(n) => *n,
        LatticeValue::Dimension { value, .. } => *value,
        _ => 0.0,
    }
}

/// Extract RGBA components from a color value.
fn extract_color(val: &LatticeValue) -> Result<(u8, u8, u8, f64), LatticeError> {
    match val {
        LatticeValue::Color(hex) => Ok(hex_to_rgba(hex)),
        _ => Err(LatticeError::type_error("color function", val.type_name(), "color", 0, 0)),
    }
}

/// Apply a unary math function (floor, ceil, round, abs) to a numeric value.
fn apply_math_unary(val: &LatticeValue, f: fn(f64) -> f64) -> Result<LatticeValue, LatticeError> {
    match val {
        LatticeValue::Number(n) => Ok(LatticeValue::Number(f(*n))),
        LatticeValue::Dimension { value, unit } => Ok(LatticeValue::Dimension { value: f(*value), unit: unit.clone() }),
        LatticeValue::Percentage(p) => Ok(LatticeValue::Percentage(f(*p))),
        _ => Err(LatticeError::type_error("math function", val.type_name(), "number", 0, 0)),
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Get the canonical string name of a token's type.
///
/// Tokens from the GrammarLexer have a `type_name` field for custom types
/// (VARIABLE, AT_KEYWORD, etc.) and a `type_` field for built-in types
/// (Number, String, Colon, etc.). We prefer `type_name` when available
/// because it gives us the grammar-level name.
pub fn get_token_type_name(token: &Token) -> String {
    if let Some(ref name) = token.type_name {
        name.clone()
    } else {
        format!("{:?}", token.type_)
    }
}

/// Compare two floats with the given operator string.
fn compare_nums(l: f64, r: f64, op: &str) -> bool {
    match op {
        "EQUALS_EQUALS" => l == r,
        "NOT_EQUALS" => l != r,
        "GREATER" => l > r,
        "GREATER_EQUALS" => l >= r,
        "LESS_EQUALS" => l <= r,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::{ScopeChain, ScopeValue};

    // Helper: create a scope with a variable binding
    fn scope_with(name: &str, value: LatticeValue) -> ScopeChain {
        let mut scope = ScopeChain::new();
        scope.set(name.to_string(), ScopeValue::Evaluated(value));
        scope
    }

    #[test]
    fn test_add_numbers() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let result = ev.add(&LatticeValue::Number(2.0), &LatticeValue::Number(3.0)).unwrap();
        assert_eq!(result, LatticeValue::Number(5.0));
    }

    #[test]
    fn test_add_dimensions_same_unit() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let left = LatticeValue::Dimension { value: 10.0, unit: "px".to_string() };
        let right = LatticeValue::Dimension { value: 5.0, unit: "px".to_string() };
        let result = ev.add(&left, &right).unwrap();
        assert_eq!(result, LatticeValue::Dimension { value: 15.0, unit: "px".to_string() });
    }

    #[test]
    fn test_add_dimensions_different_units_is_error() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let left = LatticeValue::Dimension { value: 10.0, unit: "px".to_string() };
        let right = LatticeValue::Dimension { value: 5.0, unit: "em".to_string() };
        assert!(ev.add(&left, &right).is_err());
    }

    #[test]
    fn test_subtract_percentages() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let result = ev.subtract(
            &LatticeValue::Percentage(100.0),
            &LatticeValue::Percentage(30.0),
        ).unwrap();
        assert_eq!(result, LatticeValue::Percentage(70.0));
    }

    #[test]
    fn test_multiply_number_by_dimension() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let num = LatticeValue::Number(2.0);
        let dim = LatticeValue::Dimension { value: 8.0, unit: "px".to_string() };
        let result = ev.multiply(&num, &dim).unwrap();
        assert_eq!(result, LatticeValue::Dimension { value: 16.0, unit: "px".to_string() });
    }

    #[test]
    fn test_negate_dimension() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let dim = LatticeValue::Dimension { value: 4.0, unit: "px".to_string() };
        let result = ev.negate(&dim).unwrap();
        assert_eq!(result, LatticeValue::Dimension { value: -4.0, unit: "px".to_string() });
    }

    #[test]
    fn test_compare_numbers() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        assert_eq!(ev.compare(&LatticeValue::Number(5.0), &LatticeValue::Number(5.0), "EQUALS_EQUALS"),
            LatticeValue::Bool(true));
        assert_eq!(ev.compare(&LatticeValue::Number(3.0), &LatticeValue::Number(5.0), "EQUALS_EQUALS"),
            LatticeValue::Bool(false));
        assert_eq!(ev.compare(&LatticeValue::Number(3.0), &LatticeValue::Number(5.0), "LESS_EQUALS"),
            LatticeValue::Bool(true));
    }

    #[test]
    fn test_compare_idents() {
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let dark = LatticeValue::Ident("dark".to_string());
        let light = LatticeValue::Ident("light".to_string());
        assert_eq!(ev.compare(&dark, &dark.clone(), "EQUALS_EQUALS"), LatticeValue::Bool(true));
        assert_eq!(ev.compare(&dark, &light, "EQUALS_EQUALS"), LatticeValue::Bool(false));
        assert_eq!(ev.compare(&dark, &light, "NOT_EQUALS"), LatticeValue::Bool(true));
    }

    #[test]
    fn test_lookup_variable() {
        let scope = scope_with("$color", LatticeValue::Ident("red".to_string()));
        let ev = ExpressionEvaluator::new(&scope);
        let result = ev.lookup_variable("$color", 0, 0).unwrap();
        assert_eq!(result, LatticeValue::Ident("red".to_string()));
    }

    #[test]
    fn test_lookup_missing_variable_returns_ident() {
        // Missing variables return Ident rather than error at the evaluator level.
        // The transformer layer checks for undefined variables.
        let scope = ScopeChain::new();
        let ev = ExpressionEvaluator::new(&scope);
        let result = ev.lookup_variable("$missing", 0, 0).unwrap();
        assert_eq!(result, LatticeValue::Ident("$missing".to_string()));
    }
}

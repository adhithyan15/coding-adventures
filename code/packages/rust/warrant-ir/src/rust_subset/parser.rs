//! Recursive-descent parser for the Confined Rust Subset (VER25a, VER-P022).

use crate::rust_subset::ast::*;
use crate::rust_subset::lexer::{LexError, Lexer, Token, TokenKind};

const MAX_RECURSION_DEPTH: usize = 32;
const MAX_SOURCE_BYTES: usize = 1_048_576; // 1 MiB

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self {
            message: e.message,
            span: e.span,
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}

impl Parser {
    pub fn new(source: &str) -> Result<Self, ParseError> {
        if source.len() > MAX_SOURCE_BYTES {
            return Err(ParseError {
                message: format!(
                    "Source length {} exceeds maximum limit of {}",
                    source.len(),
                    MAX_SOURCE_BYTES
                ),
                span: Span::default(),
            });
        }
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize_all()?;
        Ok(Self {
            tokens,
            pos: 0,
            depth: 0,
        })
    }

    fn enter_scope(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            return Err(ParseError {
                message: format!(
                    "Maximum recursion depth of {} exceeded",
                    MAX_RECURSION_DEPTH
                ),
                span: self.peek_span(),
            });
        }
        Ok(())
    }

    fn leave_scope(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    fn peek(&self) -> &TokenKind {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos].kind
        } else {
            &TokenKind::Eof
        }
    }

    fn peek_ahead(&self, n: usize) -> &TokenKind {
        if self.pos + n < self.tokens.len() {
            &self.tokens[self.pos + n].kind
        } else {
            &TokenKind::Eof
        }
    }

    fn peek_span(&self) -> Span {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].span
        } else if let Some(last) = self.tokens.last() {
            last.span
        } else {
            Span::default()
        }
    }

    fn advance(&mut self) -> Token {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            tok
        } else {
            Token {
                kind: TokenKind::Eof,
                span: Span::default(),
            }
        }
    }

    fn expect(&mut self, expected: TokenKind) -> Result<Token, ParseError> {
        let tok = self.advance();
        if tok.kind == expected {
            Ok(tok)
        } else {
            Err(ParseError {
                message: format!("Expected {:?}, found {:?}", expected, tok.kind),
                span: tok.span,
            })
        }
    }

    fn match_token(&mut self, expected: &TokenKind) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn parse_file(&mut self) -> Result<SourceFile, ParseError> {
        let mut inner_attrs = Vec::new();
        while self.peek() == &TokenKind::Hash && self.peek_ahead(1) == &TokenKind::Bang {
            inner_attrs.push(self.parse_attribute(true)?);
        }

        let mut items = Vec::new();
        while self.peek() != &TokenKind::Eof {
            items.push(self.parse_item()?);
        }

        Ok(SourceFile {
            inner_attrs,
            items,
        })
    }

    pub fn parse_attribute(&mut self, is_inner: bool) -> Result<Attribute, ParseError> {
        let start = self.peek_span();
        self.expect(TokenKind::Hash)?;
        if is_inner {
            self.expect(TokenKind::Bang)?;
        }
        self.expect(TokenKind::LBracket)?;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => {
                return Err(ParseError {
                    message: format!("Expected attribute name, found {:?}", other),
                    span: start,
                });
            }
        };

        let mut args = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            while self.peek() != &TokenKind::RParen && self.peek() != &TokenKind::Eof {
                match self.advance().kind {
                    TokenKind::Ident(s) | TokenKind::StringLit(s) => args.push(s),
                    TokenKind::Comma | TokenKind::Eq => {}
                    other => {
                        return Err(ParseError {
                            message: format!("Unexpected attribute argument: {:?}", other),
                            span: self.peek_span(),
                        });
                    }
                }
            }
            self.expect(TokenKind::RParen)?;
        }
        self.expect(TokenKind::RBracket)?;
        let end = self.peek_span();

        Ok(Attribute {
            is_inner,
            name,
            args,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    pub fn parse_item(&mut self) -> Result<Item, ParseError> {
        let mut attrs = Vec::new();
        while self.peek() == &TokenKind::Hash && self.peek_ahead(1) != &TokenKind::Bang {
            attrs.push(self.parse_attribute(false)?);
        }

        let mut vis = Visibility::Private;
        if self.match_token(&TokenKind::Pub) {
            if self.match_token(&TokenKind::LParen) {
                if self.match_token(&TokenKind::Super) {
                    self.expect(TokenKind::RParen)?;
                    vis = Visibility::PublicSuper;
                } else {
                    return Err(ParseError {
                        message: "Unsupported visibility modifier".to_string(),
                        span: self.peek_span(),
                    });
                }
            } else {
                vis = Visibility::Public;
            }
        }

        match self.peek() {
            TokenKind::Fn | TokenKind::Unsafe => {
                let is_unsafe = self.match_token(&TokenKind::Unsafe);
                self.expect(TokenKind::Fn)?;
                self.parse_fn(attrs, vis, is_unsafe)
            }
            TokenKind::Mod => {
                self.expect(TokenKind::Mod)?;
                self.parse_mod(attrs, vis)
            }
            TokenKind::Use => {
                self.expect(TokenKind::Use)?;
                self.parse_use()
            }
            other => Err(ParseError {
                message: format!("Expected item, found {:?}", other),
                span: self.peek_span(),
            }),
        }
    }

    fn parse_use(&mut self) -> Result<Item, ParseError> {
        let start = self.peek_span();
        let mut path = Vec::new();
        loop {
            match self.advance().kind {
                TokenKind::Ident(_) | TokenKind::Super => path.push(s_val(&self.tokens[self.pos - 1])),
                TokenKind::Star => {
                    self.expect(TokenKind::Semi)?;
                    let end = self.peek_span();
                    return Ok(Item::Use(UseDecl {
                        path,
                        tree: UseTree::Wildcard,
                        span: Span::new(start.start, end.end, start.line, start.col),
                    }));
                }
                TokenKind::LBrace => {
                    let mut group = Vec::new();
                    while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
                        match self.advance().kind {
                            TokenKind::Ident(s) => group.push(UseTree::Simple(s)),
                            TokenKind::Comma => {}
                            other => {
                                return Err(ParseError {
                                    message: format!("Unexpected token in use group: {:?}", other),
                                    span: self.peek_span(),
                                });
                            }
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                    self.expect(TokenKind::Semi)?;
                    let end = self.peek_span();
                    return Ok(Item::Use(UseDecl {
                        path,
                        tree: UseTree::Group(group),
                        span: Span::new(start.start, end.end, start.line, start.col),
                    }));
                }
                other => {
                    return Err(ParseError {
                        message: format!("Unexpected token in use path: {:?}", other),
                        span: self.peek_span(),
                    });
                }
            }

            if self.match_token(&TokenKind::ColonColon) {
                continue;
            } else if self.match_token(&TokenKind::Semi) {
                let last = path.pop().unwrap_or_default();
                let end = self.peek_span();
                return Ok(Item::Use(UseDecl {
                    path,
                    tree: UseTree::Simple(last),
                    span: Span::new(start.start, end.end, start.line, start.col),
                }));
            } else {
                return Err(ParseError {
                    message: "Expected '::' or ';' in use declaration".to_string(),
                    span: self.peek_span(),
                });
            }
        }
    }

    fn parse_mod(&mut self, attrs: Vec<Attribute>, vis: Visibility) -> Result<Item, ParseError> {
        let start = self.peek_span();
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => {
                return Err(ParseError {
                    message: format!("Expected module name, found {:?}", other),
                    span: start,
                });
            }
        };

        self.expect(TokenKind::LBrace)?;
        let mut items = Vec::new();
        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            items.push(self.parse_item()?);
        }
        self.expect(TokenKind::RBrace)?;
        let end = self.peek_span();

        Ok(Item::Module(ModuleDecl {
            attrs,
            vis,
            name,
            items,
            span: Span::new(start.start, end.end, start.line, start.col),
        }))
    }

    fn parse_fn(
        &mut self,
        attrs: Vec<Attribute>,
        vis: Visibility,
        is_unsafe: bool,
    ) -> Result<Item, ParseError> {
        let start = self.peek_span();
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => {
                return Err(ParseError {
                    message: format!("Expected function name, found {:?}", other),
                    span: start,
                });
            }
        };

        // Const generics: <const N: usize>
        let mut const_generics = Vec::new();
        if self.match_token(&TokenKind::Lt) {
            while self.peek() != &TokenKind::Gt && self.peek() != &TokenKind::Eof {
                self.expect(TokenKind::Const)?;
                let c_start = self.peek_span();
                let c_name = match self.advance().kind {
                    TokenKind::Ident(s) => s,
                    other => {
                        return Err(ParseError {
                            message: format!("Expected const generic name, found {:?}", other),
                            span: c_start,
                        });
                    }
                };
                self.expect(TokenKind::Colon)?;
                let c_ty = self.parse_type()?;
                const_generics.push(ConstGenericParam {
                    name: c_name,
                    ty: c_ty,
                    span: c_start,
                });
                self.match_token(&TokenKind::Comma);
            }
            self.expect(TokenKind::Gt)?;
        }

        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &TokenKind::RParen && self.peek() != &TokenKind::Eof {
            let p_start = self.peek_span();
            let is_mut = self.match_token(&TokenKind::Mut);
            let p_name = match self.advance().kind {
                TokenKind::Ident(s) => s,
                other => {
                    return Err(ParseError {
                        message: format!("Expected parameter name, found {:?}", other),
                        span: p_start,
                    });
                }
            };
            self.expect(TokenKind::Colon)?;
            let p_ty = self.parse_type()?;
            params.push(Param {
                is_mut,
                name: p_name,
                ty: p_ty,
                span: p_start,
            });
            self.match_token(&TokenKind::Comma);
        }
        self.expect(TokenKind::RParen)?;

        let mut return_type = None;
        if self.match_token(&TokenKind::Arrow) {
            return_type = Some(self.parse_type()?);
        }

        let is_inline_always = attrs
            .iter()
            .any(|a| a.name == "inline" && a.args.contains(&"always".to_string()));
        let is_inline_never = attrs
            .iter()
            .any(|a| a.name == "inline" && a.args.contains(&"never".to_string()));

        let body = self.parse_block()?;
        let end = body.span;

        Ok(Item::Function(FnDecl {
            attrs,
            vis,
            is_unsafe,
            is_inline_always,
            is_inline_never,
            name,
            const_generics,
            params,
            return_type,
            body,
            span: Span::new(start.start, end.end, start.line, start.col),
        }))
    }

    pub fn parse_type(&mut self) -> Result<RustType, ParseError> {
        self.enter_scope()?;
        let ty = self.parse_type_inner()?;
        self.leave_scope();
        Ok(ty)
    }

    fn parse_type_inner(&mut self) -> Result<RustType, ParseError> {
        if self.match_token(&TokenKind::And) {
            let is_mut = self.match_token(&TokenKind::Mut);
            let inner = self.parse_type()?;
            return if is_mut {
                Ok(RustType::RefMut(Box::new(inner)))
            } else {
                Ok(RustType::Ref(Box::new(inner)))
            };
        }

        if self.match_token(&TokenKind::LBracket) {
            let inner = self.parse_type()?;
            if self.match_token(&TokenKind::Semi) {
                // Array repeat: [T; N]
                match self.advance().kind {
                    TokenKind::Int(n, _) => {
                        self.expect(TokenKind::RBracket)?;
                        return Ok(RustType::Array(Box::new(inner), n as usize));
                    }
                    TokenKind::Ident(name) => {
                        self.expect(TokenKind::RBracket)?;
                        return Ok(RustType::ConstArray(Box::new(inner), name));
                    }
                    other => {
                        return Err(ParseError {
                            message: format!("Expected array size, found {:?}", other),
                            span: self.peek_span(),
                        });
                    }
                }
            } else {
                self.expect(TokenKind::RBracket)?;
                return Ok(RustType::Slice(Box::new(inner)));
            }
        }

        if self.match_token(&TokenKind::LParen) {
            self.expect(TokenKind::RParen)?;
            return Ok(RustType::Unit);
        }

        match self.advance().kind {
            TokenKind::Ident(s) => match s.as_str() {
                "u8" => Ok(RustType::U8),
                "u16" => Ok(RustType::U16),
                "u32" => Ok(RustType::U32),
                "u64" => Ok(RustType::U64),
                "usize" => Ok(RustType::Usize),
                "i8" => Ok(RustType::I8),
                "i16" => Ok(RustType::I16),
                "i32" => Ok(RustType::I32),
                "i64" => Ok(RustType::I64),
                "isize" => Ok(RustType::Isize),
                "bool" => Ok(RustType::Bool),
                "Vec" => {
                    self.expect(TokenKind::Lt)?;
                    let inner = self.parse_type()?;
                    self.expect(TokenKind::Gt)?;
                    Ok(RustType::Vec(Box::new(inner)))
                }
                _ => Ok(RustType::Custom(s)),
            },
            other => Err(ParseError {
                message: format!("Expected type, found {:?}", other),
                span: self.peek_span(),
            }),
        }
    }

    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.enter_scope()?;
        let start = self.peek_span();
        self.expect(TokenKind::LBrace)?;

        let mut stmts = Vec::new();
        let mut expr = None;

        while self.peek() != &TokenKind::RBrace && self.peek() != &TokenKind::Eof {
            match self.peek() {
                TokenKind::Let => {
                    stmts.push(self.parse_let_stmt()?);
                }
                TokenKind::Return => {
                    stmts.push(self.parse_return_stmt()?);
                }
                TokenKind::For => {
                    stmts.push(self.parse_for_stmt()?);
                }
                TokenKind::Unsafe => {
                    stmts.push(self.parse_unsafe_stmt()?);
                }
                TokenKind::Fn | TokenKind::Mod | TokenKind::Use => {
                    let item = self.parse_item()?;
                    stmts.push(Stmt::Item(Box::new(item)));
                }
                _ => {
                    // Could be an expression statement, assignment, or trailing expression
                    let e = self.parse_expr()?;
                    if self.is_assign_op(self.peek()) {
                        let op = self.parse_assign_op()?;
                        let val = self.parse_expr()?;
                        self.expect(TokenKind::Semi)?;
                        stmts.push(Stmt::Assign {
                            target: e,
                            op,
                            value: val,
                            span: start,
                        });
                    } else if self.match_token(&TokenKind::Semi) {
                        let s_span = e.span();
                        stmts.push(Stmt::Expr(e, s_span));
                    } else if self.peek() == &TokenKind::RBrace {
                        expr = Some(Box::new(e));
                        break;
                    } else {
                        // In Rust, blocks like `if` or `for` don't require semicolons
                        match &e {
                            Expr::If(..) | Expr::Block(..) => {
                                let s_span = e.span();
                                stmts.push(Stmt::Expr(e, s_span));
                            }
                            _ => {
                                return Err(ParseError {
                                    message: format!(
                                        "Expected ';' after expression, found {:?}",
                                        self.peek()
                                    ),
                                    span: self.peek_span(),
                                });
                            }
                        }
                    }
                }
            }
        }

        self.expect(TokenKind::RBrace)?;
        let end = self.peek_span();
        self.leave_scope();

        Ok(Block {
            stmts,
            expr,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.expect(TokenKind::Let)?;
        let is_mut = self.match_token(&TokenKind::Mut);
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => {
                return Err(ParseError {
                    message: format!("Expected variable name, found {:?}", other),
                    span: start,
                });
            }
        };

        let mut ty = None;
        if self.match_token(&TokenKind::Colon) {
            ty = Some(self.parse_type()?);
        }

        let mut init = None;
        if self.match_token(&TokenKind::Eq) {
            init = Some(self.parse_expr()?);
        }

        self.expect(TokenKind::Semi)?;
        let end = self.peek_span();

        Ok(Stmt::Let {
            name,
            is_mut,
            ty,
            init,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.expect(TokenKind::Return)?;
        let mut expr = None;
        if self.peek() != &TokenKind::Semi {
            expr = Some(self.parse_expr()?);
        }
        self.expect(TokenKind::Semi)?;
        let end = self.peek_span();

        Ok(Stmt::Return(
            expr,
            Span::new(start.start, end.end, start.line, start.col),
        ))
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.expect(TokenKind::For)?;
        let var = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => {
                return Err(ParseError {
                    message: format!("Expected loop variable, found {:?}", other),
                    span: start,
                });
            }
        };

        self.expect(TokenKind::In)?;
        let iter = self.parse_expr()?;
        let body = self.parse_block()?;
        let end = body.span;

        Ok(Stmt::For {
            var,
            iter,
            body,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_unsafe_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.expect(TokenKind::Unsafe)?;
        let block = self.parse_block()?;
        let end = block.span;

        Ok(Stmt::Unsafe(
            block,
            Span::new(start.start, end.end, start.line, start.col),
        ))
    }

    fn is_assign_op(&self, tok: &TokenKind) -> bool {
        matches!(
            tok,
            TokenKind::Eq
                | TokenKind::OrEq
                | TokenKind::AndEq
                | TokenKind::CaretEq
                | TokenKind::PlusEq
                | TokenKind::MinusEq
                | TokenKind::ShlEq
                | TokenKind::ShrEq
        )
    }

    fn parse_assign_op(&mut self) -> Result<AssignOp, ParseError> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::Eq => Ok(AssignOp::Assign),
            TokenKind::OrEq => Ok(AssignOp::BitOrAssign),
            TokenKind::AndEq => Ok(AssignOp::BitAndAssign),
            TokenKind::CaretEq => Ok(AssignOp::BitXorAssign),
            TokenKind::PlusEq => Ok(AssignOp::AddAssign),
            TokenKind::MinusEq => Ok(AssignOp::SubAssign),
            TokenKind::ShlEq => Ok(AssignOp::ShlAssign),
            TokenKind::ShrEq => Ok(AssignOp::ShrAssign),
            other => Err(ParseError {
                message: format!("Expected assignment operator, found {:?}", other),
                span: tok.span,
            }),
        }
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.enter_scope()?;
        let expr = self.parse_expr_precedence(0)?;
        self.leave_scope();
        Ok(expr)
    }

    fn parse_expr_precedence(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary_expr()?;

        loop {
            // Check for range `..` or `..=`
            if self.match_token(&TokenKind::DotDot) || self.match_token(&TokenKind::DotDotEq) {
                let is_inclusive = self.tokens[self.pos - 1].kind == TokenKind::DotDotEq;
                let start_span = left.span();
                let right = if self.can_start_expr(self.peek()) {
                    Some(Box::new(self.parse_expr_precedence(0)?))
                } else {
                    None
                };
                let end_span = right.as_ref().map(|r| r.span()).unwrap_or(start_span);
                left = Expr::Range(
                    Some(Box::new(left)),
                    right,
                    is_inclusive,
                    Span::new(start_span.start, end_span.end, start_span.line, start_span.col),
                );
                continue;
            }

            // Check for cast `as Type`
            if self.match_token(&TokenKind::As) {
                let target_ty = self.parse_type()?;
                let l_span = left.span();
                left = Expr::Cast(Box::new(left), target_ty, l_span);
                continue;
            }

            // Binary operations
            let (op, prec) = match self.get_bin_op(self.peek()) {
                Some((op, p)) if p >= min_prec => (op, p),
                _ => break,
            };

            self.advance(); // consume operator
            let right = self.parse_expr_precedence(prec + 1)?;
            let l_span = left.span();
            let r_span = right.span();
            left = Expr::Binary(
                op,
                Box::new(left),
                Box::new(right),
                Span::new(l_span.start, r_span.end, l_span.line, l_span.col),
            );
        }

        Ok(left)
    }

    fn can_start_expr(&self, tok: &TokenKind) -> bool {
        matches!(
            tok,
            TokenKind::Ident(_)
                | TokenKind::Int(_, _)
                | TokenKind::ByteString(_)
                | TokenKind::StringLit(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::Not
                | TokenKind::Minus
                | TokenKind::And
                | TokenKind::Star
                | TokenKind::If
        )
    }

    fn get_bin_op(&self, tok: &TokenKind) -> Option<(BinOp, u8)> {
        match tok {
            TokenKind::OrOr => Some((BinOp::Or, 1)),
            TokenKind::AndAnd => Some((BinOp::And, 2)),
            TokenKind::Or => Some((BinOp::BitOr, 3)),
            TokenKind::Caret => Some((BinOp::BitXor, 4)),
            TokenKind::And => Some((BinOp::BitAnd, 5)),
            TokenKind::EqEq => Some((BinOp::Eq, 6)),
            TokenKind::Ne => Some((BinOp::Ne, 6)),
            TokenKind::Lt => Some((BinOp::Lt, 7)),
            TokenKind::LtEq => Some((BinOp::Le, 7)),
            TokenKind::Gt => Some((BinOp::Gt, 7)),
            TokenKind::GtEq => Some((BinOp::Ge, 7)),
            TokenKind::Shl => Some((BinOp::Shl, 8)),
            TokenKind::Shr => Some((BinOp::Shr, 8)),
            TokenKind::Plus => Some((BinOp::Add, 9)),
            TokenKind::Minus => Some((BinOp::Sub, 9)),
            TokenKind::Star => Some((BinOp::Mul, 10)),
            TokenKind::Slash => Some((BinOp::Div, 10)),
            TokenKind::Percent => Some((BinOp::Rem, 10)),
            _ => None,
        }
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_base_expr()?;

        loop {
            match self.peek() {
                TokenKind::LParen => {
                    // Function or path call
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek() != &TokenKind::RParen && self.peek() != &TokenKind::Eof {
                        args.push(self.parse_expr()?);
                        self.match_token(&TokenKind::Comma);
                    }
                    self.expect(TokenKind::RParen)?;
                    let span = expr.span();
                    expr = Expr::Call(Box::new(expr), args, span);
                }
                TokenKind::LBracket => {
                    // Indexing `a[i]`
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    let span = expr.span();
                    expr = Expr::Index(Box::new(expr), Box::new(idx), span);
                }
                TokenKind::Dot => {
                    self.advance();
                    let m_start = self.peek_span();
                    let method = match self.advance().kind {
                        TokenKind::Ident(s) => s,
                        other => {
                            return Err(ParseError {
                                message: format!("Expected method name after '.', found {:?}", other),
                                span: m_start,
                            });
                        }
                    };

                    if self.match_token(&TokenKind::LParen) {
                        let mut args = Vec::new();
                        while self.peek() != &TokenKind::RParen && self.peek() != &TokenKind::Eof {
                            args.push(self.parse_expr()?);
                            self.match_token(&TokenKind::Comma);
                        }
                        self.expect(TokenKind::RParen)?;
                        let span = expr.span();
                        expr = Expr::MethodCall(Box::new(expr), method, args, span);
                    } else {
                        // Field or zero-arg without parens, or chained
                        let span = expr.span();
                        expr = Expr::MethodCall(Box::new(expr), method, Vec::new(), span);
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_base_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span();

        // Unary operations
        if self.match_token(&TokenKind::Bang) {
            self.enter_scope()?;
            let inner = self.parse_primary_expr()?;
            self.leave_scope();
            let span = Span::new(start.start, inner.span().end, start.line, start.col);
            return Ok(Expr::Unary(UnOp::Not, Box::new(inner), span));
        }
        if self.match_token(&TokenKind::Minus) {
            self.enter_scope()?;
            let inner = self.parse_primary_expr()?;
            self.leave_scope();
            let span = Span::new(start.start, inner.span().end, start.line, start.col);
            return Ok(Expr::Unary(UnOp::Neg, Box::new(inner), span));
        }
        if self.match_token(&TokenKind::Star) {
            self.enter_scope()?;
            let inner = self.parse_primary_expr()?;
            self.leave_scope();
            let span = Span::new(start.start, inner.span().end, start.line, start.col);
            return Ok(Expr::Unary(UnOp::Deref, Box::new(inner), span));
        }
        if self.match_token(&TokenKind::And) {
            self.enter_scope()?;
            let is_mut = self.match_token(&TokenKind::Mut);
            let inner = self.parse_primary_expr()?;
            self.leave_scope();
            let span = Span::new(start.start, inner.span().end, start.line, start.col);
            return if is_mut {
                Ok(Expr::Unary(UnOp::RefMut, Box::new(inner), span))
            } else {
                Ok(Expr::Unary(UnOp::Ref, Box::new(inner), span))
            };
        }

        // Literals
        match self.peek() {
            TokenKind::True => {
                self.advance();
                return Ok(Expr::Lit(Lit::Bool(true), start));
            }
            TokenKind::False => {
                self.advance();
                return Ok(Expr::Lit(Lit::Bool(false), start));
            }
            TokenKind::Int(val, suf) => {
                let v = *val;
                let s = suf.clone();
                self.advance();
                return Ok(Expr::Lit(Lit::Int(v, s), start));
            }
            TokenKind::ByteString(bytes) => {
                let b = bytes.clone();
                self.advance();
                return Ok(Expr::Lit(Lit::ByteString(b), start));
            }
            TokenKind::StringLit(s) => {
                let str_val = s.clone();
                self.advance();
                return Ok(Expr::Lit(Lit::String(str_val), start));
            }
            TokenKind::LParen => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect(TokenKind::RParen)?;
                return Ok(inner);
            }
            TokenKind::LBracket => {
                self.advance();
                if self.match_token(&TokenKind::RBracket) {
                    let end = self.peek_span();
                    return Ok(Expr::Array(
                        Vec::new(),
                        Span::new(start.start, end.end, start.line, start.col),
                    ));
                }
                let first = self.parse_expr()?;
                if self.match_token(&TokenKind::Semi) {
                    let count_expr = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    let end = self.peek_span();
                    return Ok(Expr::ArrayRepeat(
                        Box::new(first),
                        Box::new(count_expr),
                        Span::new(start.start, end.end, start.line, start.col),
                    ));
                }
                let mut elements = vec![first];
                while self.match_token(&TokenKind::Comma) {
                    if self.peek() == &TokenKind::RBracket {
                        break;
                    }
                    elements.push(self.parse_expr()?);
                }
                self.expect(TokenKind::RBracket)?;
                let end = self.peek_span();
                return Ok(Expr::Array(
                    elements,
                    Span::new(start.start, end.end, start.line, start.col),
                ));
            }
            TokenKind::If => {
                return self.parse_if_expr();
            }
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                let span = block.span;
                return Ok(Expr::Block(block, span));
            }
            _ => {}
        }

        // Identifiers, Paths, Macro calls
        if let TokenKind::Ident(_) = self.peek() {
            let mut path = Vec::new();
            while let TokenKind::Ident(s) = self.peek() {
                path.push(s.clone());
                self.advance();
                if self.match_token(&TokenKind::ColonColon) {
                    if self.match_token(&TokenKind::Lt) {
                        // Turbofish: ::<type>
                        let _turbofish_ty = self.parse_type()?;
                        self.expect(TokenKind::Gt)?;
                        if self.match_token(&TokenKind::ColonColon) {
                            continue;
                        } else {
                            break;
                        }
                    }
                    continue;
                } else {
                    break;
                }
            }

            // Macro call: path! (...) or path! [...]
            if self.match_token(&TokenKind::Bang) {
                let macro_name = path.join("::");
                let delimiter = self.advance();
                let closing = match delimiter.kind {
                    TokenKind::LParen => TokenKind::RParen,
                    TokenKind::LBracket => TokenKind::RBracket,
                    TokenKind::LBrace => TokenKind::RBrace,
                    other => {
                        return Err(ParseError {
                            message: format!("Expected macro delimiter, found {:?}", other),
                            span: delimiter.span,
                        });
                    }
                };

                let mut args = Vec::new();
                while self.peek() != &closing && self.peek() != &TokenKind::Eof {
                    args.push(self.parse_expr()?);
                    if self.match_token(&TokenKind::Semi) {
                        // e.g. vec![0u8; 32]
                    } else {
                        self.match_token(&TokenKind::Comma);
                    }
                }
                self.expect(closing)?;
                let end = self.peek_span();
                return Ok(Expr::MacroCall(
                    macro_name,
                    args,
                    Span::new(start.start, end.end, start.line, start.col),
                ));
            }

            return Ok(Expr::Path(path, start));
        }

        Err(ParseError {
            message: format!("Unexpected token in expression: {:?}", self.peek()),
            span: start,
        })
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.peek_span();
        self.expect(TokenKind::If)?;
        let cond = self.parse_expr()?;
        let then_block = self.parse_block()?;

        let mut else_block = None;
        if self.match_token(&TokenKind::Else) {
            if self.peek() == &TokenKind::If {
                let nested_if = self.parse_if_expr()?;
                let n_span = nested_if.span();
                else_block = Some(Block {
                    stmts: vec![Stmt::Expr(nested_if, n_span)],
                    expr: None,
                    span: n_span,
                });
            } else {
                else_block = Some(self.parse_block()?);
            }
        }

        let end = else_block
            .as_ref()
            .map(|b| b.span)
            .unwrap_or(then_block.span);

        Ok(Expr::If(
            Box::new(cond),
            then_block,
            else_block,
            Span::new(start.start, end.end, start.line, start.col),
        ))
    }
}

fn s_val(tok: &Token) -> String {
    match &tok.kind {
        TokenKind::Ident(s) => s.clone(),
        TokenKind::Super => "super".to_string(),
        _ => String::new(),
    }
}

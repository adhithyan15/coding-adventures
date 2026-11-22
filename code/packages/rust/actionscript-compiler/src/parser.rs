//! ActionScript bounded recursive descent parser supporting AS1, AS2, and AS3 dialects.

#![deny(unsafe_code)]

use crate::ast::{
    AssignOp, BinaryOp, CatchClause, ClassDef, Expr, FieldDef, FunctionDef, InterfaceDef,
    PackageDef, Param, Program, Stmt, TypeAnnotation, UnaryOp, Visibility,
};
use crate::dialect::AsDialect;
use crate::error::{AsCompilerError, MAX_AST_DEPTH, MAX_AST_NODES};
use crate::token::{Span, Token, TokenKind};

/// Parser state machine.
pub struct AsParser {
    tokens: Vec<Token>,
    cursor: usize,
    dialect: AsDialect,
    depth: usize,
    node_count: usize,
    eof_token: Token,
}

impl AsParser {
    /// Create a new parser.
    pub fn parser_new(tokens: Vec<Token>, dialect: AsDialect) -> Self {
        let eof_token = Token::token_new(TokenKind::Eof, Span::span_empty(), String::new());
        Self {
            tokens,
            cursor: 0,
            dialect,
            depth: 0,
            node_count: 0,
            eof_token,
        }
    }

    /// Parse complete program.
    pub fn parser_parse_program(&mut self) -> Result<Program, AsCompilerError> {
        let mut stmts = Vec::new();
        while !self.parser_is_at_end() {
            if self.parser_match(&TokenKind::Semicolon) {
                continue;
            }
            let s = self.parser_parse_statement()?;
            stmts.push(s);
        }
        Ok(Program::program_new(stmts))
    }

    fn parser_enter_scope(&mut self) -> Result<(), AsCompilerError> {
        self.depth += 1;
        if self.depth > MAX_AST_DEPTH {
            return Err(AsCompilerError::RecursionDepthExceeded);
        }
        Ok(())
    }

    fn parser_exit_scope(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    fn parser_charge_node(&mut self) -> Result<(), AsCompilerError> {
        self.node_count += 1;
        if self.node_count > MAX_AST_NODES {
            return Err(AsCompilerError::InstructionLimitExceeded);
        }
        Ok(())
    }

    fn parser_is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
            || self
                .tokens
                .get(self.cursor)
                .is_none_or(|t| t.kind == TokenKind::Eof)
    }

    fn parser_peek(&self) -> &TokenKind {
        if self.cursor < self.tokens.len() {
            &self.tokens[self.cursor].kind
        } else if let Some(last) = self.tokens.last() {
            &last.kind
        } else {
            &self.eof_token.kind
        }
    }

    fn parser_peek_token(&self) -> &Token {
        if self.cursor < self.tokens.len() {
            &self.tokens[self.cursor]
        } else if let Some(last) = self.tokens.last() {
            last
        } else {
            &self.eof_token
        }
    }

    fn parser_advance(&mut self) -> Token {
        if !self.parser_is_at_end() {
            let tok = self.tokens[self.cursor].clone();
            self.cursor += 1;
            tok
        } else if let Some(last) = self.tokens.last() {
            last.clone()
        } else {
            self.eof_token.clone()
        }
    }

    fn parser_check(&self, kind: &TokenKind) -> bool {
        if self.parser_is_at_end() {
            false
        } else {
            &self.tokens[self.cursor].kind == kind
        }
    }

    fn parser_match(&mut self, kind: &TokenKind) -> bool {
        if self.parser_check(kind) {
            self.parser_advance();
            true
        } else {
            false
        }
    }

    fn parser_expect(&mut self, kind: &TokenKind) -> Result<Token, AsCompilerError> {
        if self.parser_check(kind) {
            Ok(self.parser_advance())
        } else {
            Err(AsCompilerError::SyntaxError(format!(
                "expected {:?}, found {:?} at line {}",
                kind,
                self.parser_peek(),
                self.parser_peek_token().span.line
            )))
        }
    }

    fn parser_match_ident(&mut self) -> Result<String, AsCompilerError> {
        match self.parser_peek() {
            TokenKind::Ident(name) => {
                let s = name.clone();
                self.parser_advance();
                Ok(s)
            }
            other => Err(AsCompilerError::SyntaxError(format!(
                "expected identifier, found {:?} at line {}",
                other,
                self.parser_peek_token().span.line
            ))),
        }
    }

    // ------------------------------------------------------------------------
    // Statements
    // ------------------------------------------------------------------------

    fn parser_parse_statement(&mut self) -> Result<Stmt, AsCompilerError> {
        self.parser_enter_scope()?;
        self.parser_charge_node()?;
        let res = self.parser_parse_statement_inner();
        self.parser_exit_scope();
        res
    }

    fn parser_parse_statement_inner(&mut self) -> Result<Stmt, AsCompilerError> {
        if let Some(stmt) = self.parser_try_parse_decl_stmt()? {
            return Ok(stmt);
        }

        if let Some(stmt) = self.parser_try_parse_timeline_verb()? {
            return Ok(stmt);
        }

        // Var / Const / Function declarations
        if self.parser_match(&TokenKind::Var) {
            return self.parser_parse_var_decl();
        }
        if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Const) {
            return self.parser_parse_const_decl();
        }
        if self.parser_check(&TokenKind::Function) {
            let f = self.parser_parse_function_decl(false, Visibility::Public)?;
            return Ok(Stmt::FunctionDecl(f));
        }

        // Block statement
        if self.parser_check(&TokenKind::LBrace) {
            let stmts = self.parser_parse_block()?;
            return Ok(Stmt::Block(stmts));
        }

        // Control flow statements
        if self.parser_match(&TokenKind::If) {
            return self.parser_parse_if_stmt();
        }
        if self.parser_match(&TokenKind::While) {
            return self.parser_parse_while_stmt();
        }
        if self.parser_match(&TokenKind::Do) {
            return self.parser_parse_do_while_stmt();
        }
        if self.parser_match(&TokenKind::For) {
            return self.parser_parse_for_stmt();
        }
        if self.parser_match(&TokenKind::Return) {
            return self.parser_parse_return_stmt();
        }
        if self.parser_match(&TokenKind::Break) {
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Stmt::Break);
        }
        if self.parser_match(&TokenKind::Continue) {
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Stmt::Continue);
        }

        // Try / Catch / Finally & Throw (AS3)
        if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Try) {
            return self.parser_parse_try_stmt();
        }
        if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Throw) {
            let expr = self.parser_parse_expression()?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Stmt::Throw(expr));
        }

        // Expression statement
        let expr = self.parser_parse_expression()?;
        self.parser_match(&TokenKind::Semicolon);
        Ok(Stmt::Expr(expr))
    }

    #[inline(never)]
    fn parser_try_parse_decl_stmt(&mut self) -> Result<Option<Stmt>, AsCompilerError> {
        // AS3 Package
        if self.dialect == AsDialect::As3 && self.parser_check(&TokenKind::Package) {
            return self.parser_parse_package_decl().map(Some);
        }

        // AS3 Namespace
        if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Namespace) {
            let name = self.parser_match_ident()?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::NamespaceDecl(name)));
        }

        // Import (AS2/AS3)
        if self.dialect != AsDialect::As1 && self.parser_match(&TokenKind::Import) {
            let mut path = self.parser_match_ident()?;
            while self.parser_match(&TokenKind::Dot) {
                if self.parser_match(&TokenKind::Star) {
                    path.push_str(".*");
                    break;
                } else {
                    let seg = self.parser_match_ident()?;
                    path.push('.');
                    path.push_str(&seg);
                }
            }
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::ImportDecl(path)));
        }

        // Interface (AS2/AS3)
        if self.dialect != AsDialect::As1 && self.parser_check(&TokenKind::Interface) {
            return self
                .parser_parse_interface_decl()
                .map(|iface| Some(Stmt::InterfaceDecl(iface)));
        }

        // Modifiers for Class / Function / Var in AS2 / AS3
        if self.dialect != AsDialect::As1 {
            let mut visibility = Visibility::Public;
            let mut member_static = false;
            let mut is_final = false;
            let mut is_dynamic = false;
            let mut saw_modifier = false;

            loop {
                if self.parser_match(&TokenKind::Public) {
                    visibility = Visibility::Public;
                    saw_modifier = true;
                } else if self.parser_match(&TokenKind::Private) {
                    visibility = Visibility::Private;
                    saw_modifier = true;
                } else if self.parser_match(&TokenKind::Static) {
                    member_static = true;
                    saw_modifier = true;
                } else if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Internal)
                {
                    visibility = Visibility::Internal;
                    saw_modifier = true;
                } else if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Final) {
                    is_final = true;
                    saw_modifier = true;
                } else if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Dynamic) {
                    is_dynamic = true;
                    saw_modifier = true;
                } else if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Override)
                {
                    saw_modifier = true;
                } else {
                    break;
                }
            }

            if self.parser_check(&TokenKind::Class) {
                let cls = self.parser_parse_class_decl(visibility, is_final, is_dynamic)?;
                return Ok(Some(Stmt::ClassDecl(cls)));
            }

            if saw_modifier {
                if self.parser_check(&TokenKind::Function) {
                    let func = self.parser_parse_function_decl(member_static, visibility)?;
                    return Ok(Some(Stmt::FunctionDecl(func)));
                }
                if self.parser_match(&TokenKind::Var) {
                    return self.parser_parse_var_decl().map(Some);
                }
                if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Const) {
                    return self.parser_parse_const_decl().map(Some);
                }
            }
        }

        // Bare class declaration without modifier
        if self.dialect != AsDialect::As1 && self.parser_check(&TokenKind::Class) {
            let cls = self.parser_parse_class_decl(Visibility::Public, false, false)?;
            return Ok(Some(Stmt::ClassDecl(cls)));
        }

        Ok(None)
    }

    #[inline(never)]
    fn parser_try_parse_timeline_verb(&mut self) -> Result<Option<Stmt>, AsCompilerError> {
        if self.parser_match(&TokenKind::Stop) {
            self.parser_expect(&TokenKind::LParen)?;
            self.parser_expect(&TokenKind::RParen)?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::Stop));
        }
        if self.parser_match(&TokenKind::Play) {
            self.parser_expect(&TokenKind::LParen)?;
            self.parser_expect(&TokenKind::RParen)?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::Play));
        }
        if self.parser_match(&TokenKind::GotoAndPlay) {
            self.parser_expect(&TokenKind::LParen)?;
            let arg = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::RParen)?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::GotoAndPlay(arg)));
        }
        if self.parser_match(&TokenKind::GotoAndStop) {
            self.parser_expect(&TokenKind::LParen)?;
            let arg = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::RParen)?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::GotoAndStop(arg)));
        }
        if self.parser_match(&TokenKind::TellTarget) {
            self.parser_expect(&TokenKind::LParen)?;
            let target = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::RParen)?;
            let body = self.parser_parse_block()?;
            return Ok(Some(Stmt::TellTarget { target, body }));
        }
        if self.parser_match(&TokenKind::Call) {
            self.parser_expect(&TokenKind::LParen)?;
            let arg = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::RParen)?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::CallAction(arg)));
        }
        if self.parser_match(&TokenKind::OnClipEvent) {
            self.parser_expect(&TokenKind::LParen)?;
            let event = self.parser_match_ident()?;
            self.parser_expect(&TokenKind::RParen)?;
            let body = self.parser_parse_block()?;
            return Ok(Some(Stmt::OnClipEvent { event, body }));
        }
        if self.parser_match(&TokenKind::Trace) {
            self.parser_expect(&TokenKind::LParen)?;
            let arg = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::RParen)?;
            self.parser_match(&TokenKind::Semicolon);
            return Ok(Some(Stmt::Trace(arg)));
        }
        Ok(None)
    }

    #[inline(never)]
    fn parser_parse_var_decl(&mut self) -> Result<Stmt, AsCompilerError> {
        let name = self.parser_match_ident()?;
        let type_ann = if self.parser_match(&TokenKind::Colon) {
            Some(self.parser_parse_type_annotation()?)
        } else {
            None
        };
        let init = if self.parser_match(&TokenKind::Eq) {
            Some(self.parser_parse_expression()?)
        } else {
            None
        };
        self.parser_match(&TokenKind::Semicolon);
        Ok(Stmt::VarDecl {
            name,
            type_ann,
            init,
        })
    }

    #[inline(never)]
    fn parser_parse_const_decl(&mut self) -> Result<Stmt, AsCompilerError> {
        let name = self.parser_match_ident()?;
        let type_ann = if self.parser_match(&TokenKind::Colon) {
            Some(self.parser_parse_type_annotation()?)
        } else {
            None
        };
        self.parser_expect(&TokenKind::Eq)?;
        let init = self.parser_parse_expression()?;
        self.parser_match(&TokenKind::Semicolon);
        Ok(Stmt::ConstDecl {
            name,
            type_ann,
            init,
        })
    }

    #[inline(never)]
    fn parser_parse_if_stmt(&mut self) -> Result<Stmt, AsCompilerError> {
        self.parser_expect(&TokenKind::LParen)?;
        let cond = self.parser_parse_expression()?;
        self.parser_expect(&TokenKind::RParen)?;
        let then_branch = Box::new(self.parser_parse_statement()?);
        let else_branch = if self.parser_match(&TokenKind::Else) {
            Some(Box::new(self.parser_parse_statement()?))
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
        })
    }

    #[inline(never)]
    fn parser_parse_while_stmt(&mut self) -> Result<Stmt, AsCompilerError> {
        self.parser_expect(&TokenKind::LParen)?;
        let cond = self.parser_parse_expression()?;
        self.parser_expect(&TokenKind::RParen)?;
        let body = Box::new(self.parser_parse_statement()?);
        Ok(Stmt::While { cond, body })
    }

    #[inline(never)]
    fn parser_parse_do_while_stmt(&mut self) -> Result<Stmt, AsCompilerError> {
        let body = Box::new(self.parser_parse_statement()?);
        self.parser_expect(&TokenKind::While)?;
        self.parser_expect(&TokenKind::LParen)?;
        let cond = self.parser_parse_expression()?;
        self.parser_expect(&TokenKind::RParen)?;
        self.parser_match(&TokenKind::Semicolon);
        Ok(Stmt::DoWhile { body, cond })
    }

    #[inline(never)]
    fn parser_parse_for_stmt(&mut self) -> Result<Stmt, AsCompilerError> {
        self.parser_expect(&TokenKind::LParen)?;

        // Check if it's for..in: for (var x in obj) or for (x in obj)
        let is_var = self.parser_match(&TokenKind::Var);
        if let TokenKind::Ident(var_name) = self.parser_peek() {
            let var_name = var_name.clone();
            if self.cursor + 1 < self.tokens.len()
                && self.tokens[self.cursor + 1].kind == TokenKind::In
            {
                self.parser_advance(); // consume ident
                self.parser_advance(); // consume in
                let obj = self.parser_parse_expression()?;
                self.parser_expect(&TokenKind::RParen)?;
                let body = Box::new(self.parser_parse_statement()?);
                return Ok(Stmt::ForIn {
                    var_name,
                    obj,
                    body,
                });
            }
        }

        let init = if is_var {
            let name = self.parser_match_ident()?;
            let type_ann = if self.parser_match(&TokenKind::Colon) {
                Some(self.parser_parse_type_annotation()?)
            } else {
                None
            };
            let init_expr = if self.parser_match(&TokenKind::Eq) {
                Some(self.parser_parse_expression()?)
            } else {
                None
            };
            self.parser_expect(&TokenKind::Semicolon)?;
            Some(Box::new(Stmt::VarDecl {
                name,
                type_ann,
                init: init_expr,
            }))
        } else if self.parser_match(&TokenKind::Semicolon) {
            None
        } else {
            let expr = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::Semicolon)?;
            Some(Box::new(Stmt::Expr(expr)))
        };

        let cond = if self.parser_check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parser_parse_expression()?)
        };
        self.parser_expect(&TokenKind::Semicolon)?;

        let step = if self.parser_check(&TokenKind::RParen) {
            None
        } else {
            Some(self.parser_parse_expression()?)
        };
        self.parser_expect(&TokenKind::RParen)?;

        let body = Box::new(self.parser_parse_statement()?);
        Ok(Stmt::For {
            init,
            cond,
            step,
            body,
        })
    }

    #[inline(never)]
    fn parser_parse_return_stmt(&mut self) -> Result<Stmt, AsCompilerError> {
        let expr = if self.parser_check(&TokenKind::Semicolon)
            || self.parser_check(&TokenKind::RBrace)
            || self.parser_is_at_end()
        {
            None
        } else {
            Some(self.parser_parse_expression()?)
        };
        self.parser_match(&TokenKind::Semicolon);
        Ok(Stmt::Return(expr))
    }

    #[inline(never)]
    fn parser_parse_try_stmt(&mut self) -> Result<Stmt, AsCompilerError> {
        let try_block = self.parser_parse_block()?;
        let mut catches = Vec::new();
        while self.parser_match(&TokenKind::Catch) {
            self.parser_expect(&TokenKind::LParen)?;
            let param = self.parser_match_ident()?;
            let param_type = if self.parser_match(&TokenKind::Colon) {
                Some(self.parser_parse_type_annotation()?)
            } else {
                None
            };
            self.parser_expect(&TokenKind::RParen)?;
            let body = self.parser_parse_block()?;
            catches.push(CatchClause::catch_clause_new(param, param_type, body));
        }
        let finally_block = if self.parser_match(&TokenKind::Finally) {
            Some(self.parser_parse_block()?)
        } else {
            None
        };
        Ok(Stmt::TryCatchFinally {
            try_block,
            catches,
            finally_block,
        })
    }

    fn parser_parse_block(&mut self) -> Result<Vec<Stmt>, AsCompilerError> {
        self.parser_expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while !self.parser_check(&TokenKind::RBrace) && !self.parser_is_at_end() {
            if self.parser_match(&TokenKind::Semicolon) {
                continue;
            }
            stmts.push(self.parser_parse_statement()?);
        }
        self.parser_expect(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn parser_parse_type_annotation(&mut self) -> Result<TypeAnnotation, AsCompilerError> {
        self.parser_enter_scope()?;
        self.parser_charge_node()?;
        let res = self.parser_parse_type_annotation_inner();
        self.parser_exit_scope();
        res
    }

    fn parser_parse_type_annotation_inner(&mut self) -> Result<TypeAnnotation, AsCompilerError> {
        if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Vector) {
            self.parser_expect(&TokenKind::DotLt)?;
            let inner = self.parser_parse_type_annotation()?;
            self.parser_expect(&TokenKind::Gt)?;
            return Ok(TypeAnnotation::Vector(Box::new(inner)));
        }
        if self.parser_match(&TokenKind::Star) {
            return Ok(TypeAnnotation::Any);
        }
        let name = self.parser_match_ident()?;
        Ok(TypeAnnotation::Simple(name))
    }

    fn parser_parse_function_decl(
        &mut self,
        is_static: bool,
        visibility: Visibility,
    ) -> Result<FunctionDef, AsCompilerError> {
        self.parser_expect(&TokenKind::Function)?;
        let name = self.parser_match_ident()?;
        self.parser_expect(&TokenKind::LParen)?;

        let mut params = Vec::new();
        if !self.parser_check(&TokenKind::RParen) {
            loop {
                let p_name = self.parser_match_ident()?;
                let p_type = if self.parser_match(&TokenKind::Colon) {
                    Some(self.parser_parse_type_annotation()?)
                } else {
                    None
                };
                params.push(Param::param_new(p_name, p_type));
                if !self.parser_match(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.parser_expect(&TokenKind::RParen)?;

        let ret_type = if self.parser_match(&TokenKind::Colon) {
            Some(self.parser_parse_type_annotation()?)
        } else {
            None
        };

        let body = self.parser_parse_block()?;

        Ok(FunctionDef::function_def_new(
            name, params, ret_type, body, is_static, visibility,
        ))
    }

    fn parser_parse_class_decl(
        &mut self,
        visibility: Visibility,
        is_final: bool,
        is_dynamic: bool,
    ) -> Result<ClassDef, AsCompilerError> {
        self.parser_expect(&TokenKind::Class)?;
        let name = self.parser_match_ident()?;

        let extends = if self.parser_match(&TokenKind::Extends) {
            Some(self.parser_match_ident()?)
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.parser_match(&TokenKind::Implements) {
            loop {
                implements.push(self.parser_match_ident()?);
                if !self.parser_match(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.parser_expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while !self.parser_check(&TokenKind::RBrace) && !self.parser_is_at_end() {
            if self.parser_match(&TokenKind::Semicolon) {
                continue;
            }

            let mut member_vis = Visibility::Public;
            let mut member_static = false;
            let mut member_const = false;

            loop {
                if self.parser_match(&TokenKind::Public) {
                    member_vis = Visibility::Public;
                } else if self.parser_match(&TokenKind::Private) {
                    member_vis = Visibility::Private;
                } else if self.parser_match(&TokenKind::Static) {
                    member_static = true;
                } else if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Internal)
                {
                    member_vis = Visibility::Internal;
                } else if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Override)
                {
                    // Ignore override in IR lowering
                } else {
                    break;
                }
            }

            if self.parser_check(&TokenKind::Function) {
                let func = self.parser_parse_function_decl(member_static, member_vis)?;
                methods.push(func);
            } else if self.parser_match(&TokenKind::Var)
                || (self.dialect == AsDialect::As3 && {
                    if self.parser_match(&TokenKind::Const) {
                        member_const = true;
                        true
                    } else {
                        false
                    }
                })
            {
                let f_name = self.parser_match_ident()?;
                let f_type = if self.parser_match(&TokenKind::Colon) {
                    Some(self.parser_parse_type_annotation()?)
                } else {
                    None
                };
                let f_init = if self.parser_match(&TokenKind::Eq) {
                    Some(self.parser_parse_expression()?)
                } else {
                    None
                };
                self.parser_match(&TokenKind::Semicolon);
                fields.push(FieldDef::field_def_new(
                    f_name,
                    f_type,
                    f_init,
                    member_static,
                    member_vis,
                    member_const,
                ));
            } else {
                return Err(AsCompilerError::SyntaxError(format!(
                    "unexpected token in class body: {:?} at line {}",
                    self.parser_peek(),
                    self.parser_peek_token().span.line
                )));
            }
        }

        self.parser_expect(&TokenKind::RBrace)?;

        Ok(ClassDef::class_def_new(
            name, extends, implements, fields, methods, is_final, is_dynamic, visibility,
        ))
    }

    fn parser_parse_interface_decl(&mut self) -> Result<InterfaceDef, AsCompilerError> {
        self.parser_expect(&TokenKind::Interface)?;
        let name = self.parser_match_ident()?;

        let mut extends = Vec::new();
        if self.parser_match(&TokenKind::Extends) {
            loop {
                extends.push(self.parser_match_ident()?);
                if !self.parser_match(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.parser_expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();

        while !self.parser_check(&TokenKind::RBrace) && !self.parser_is_at_end() {
            if self.parser_match(&TokenKind::Semicolon) {
                continue;
            }
            let _ = self.parser_match(&TokenKind::Public);
            self.parser_expect(&TokenKind::Function)?;
            let m_name = self.parser_match_ident()?;
            self.parser_expect(&TokenKind::LParen)?;

            let mut params = Vec::new();
            if !self.parser_check(&TokenKind::RParen) {
                loop {
                    let p_name = self.parser_match_ident()?;
                    let p_type = if self.parser_match(&TokenKind::Colon) {
                        Some(self.parser_parse_type_annotation()?)
                    } else {
                        None
                    };
                    params.push(Param::param_new(p_name, p_type));
                    if !self.parser_match(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.parser_expect(&TokenKind::RParen)?;
            let ret_type = if self.parser_match(&TokenKind::Colon) {
                Some(self.parser_parse_type_annotation()?)
            } else {
                None
            };
            self.parser_match(&TokenKind::Semicolon);
            methods.push((m_name, params, ret_type));
        }

        self.parser_expect(&TokenKind::RBrace)?;
        Ok(InterfaceDef::interface_def_new(name, extends, methods))
    }

    fn parser_parse_package_decl(&mut self) -> Result<Stmt, AsCompilerError> {
        self.parser_expect(&TokenKind::Package)?;
        let mut name = String::new();
        if let TokenKind::Ident(_) = self.parser_peek() {
            name = self.parser_match_ident()?;
            while self.parser_match(&TokenKind::Dot) {
                let seg = self.parser_match_ident()?;
                name.push('.');
                name.push_str(&seg);
            }
        }

        self.parser_expect(&TokenKind::LBrace)?;
        let mut statements = Vec::new();
        while !self.parser_check(&TokenKind::RBrace) && !self.parser_is_at_end() {
            if self.parser_match(&TokenKind::Semicolon) {
                continue;
            }
            statements.push(self.parser_parse_statement()?);
        }
        self.parser_expect(&TokenKind::RBrace)?;

        Ok(Stmt::PackageDecl(PackageDef::package_def_new(
            name, statements,
        )))
    }

    // ------------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------------

    fn parser_parse_expression(&mut self) -> Result<Expr, AsCompilerError> {
        self.parser_parse_assignment()
    }

    fn parser_parse_assignment(&mut self) -> Result<Expr, AsCompilerError> {
        self.parser_enter_scope()?;
        self.parser_charge_node()?;
        let res = self.parser_parse_assignment_inner();
        self.parser_exit_scope();
        res
    }

    fn parser_parse_assignment_inner(&mut self) -> Result<Expr, AsCompilerError> {
        let left = self.parser_parse_conditional()?;

        let assign_op = if self.parser_match(&TokenKind::Eq) {
            Some(AssignOp::Assign)
        } else if self.parser_match(&TokenKind::PlusEq) {
            Some(AssignOp::AddAssign)
        } else if self.parser_match(&TokenKind::MinusEq) {
            Some(AssignOp::SubAssign)
        } else if self.parser_match(&TokenKind::StarEq) {
            Some(AssignOp::MulAssign)
        } else if self.parser_match(&TokenKind::SlashEq) {
            Some(AssignOp::DivAssign)
        } else if self.parser_match(&TokenKind::PercentEq) {
            Some(AssignOp::ModAssign)
        } else {
            None
        };

        if let Some(op) = assign_op {
            let right = self.parser_parse_assignment()?;
            Ok(Expr::Assign {
                op,
                target: Box::new(left),
                value: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parser_parse_conditional(&mut self) -> Result<Expr, AsCompilerError> {
        self.parser_enter_scope()?;
        self.parser_charge_node()?;
        let res = self.parser_parse_conditional_inner();
        self.parser_exit_scope();
        res
    }

    fn parser_parse_conditional_inner(&mut self) -> Result<Expr, AsCompilerError> {
        let mut expr = self.parser_parse_binary(0)?;

        if self.parser_match(&TokenKind::Question) {
            let then_expr = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::Colon)?;
            let else_expr = self.parser_parse_conditional()?;
            expr = Expr::Conditional {
                cond: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            };
        }

        Ok(expr)
    }

    fn parser_parse_binary(&mut self, min_bp: u8) -> Result<Expr, AsCompilerError> {
        let mut left = self.parser_parse_unary()?;

        loop {
            let (op, l_bp, r_bp) = match self.parser_peek() {
                TokenKind::PipePipe => (BinaryOp::LogOr, 1, 2),
                TokenKind::AmpAmp => (BinaryOp::LogAnd, 3, 4),
                TokenKind::Pipe => (BinaryOp::BitOr, 5, 6),
                TokenKind::Caret => (BinaryOp::BitXor, 7, 8),
                TokenKind::Amp => (BinaryOp::BitAnd, 9, 10),
                TokenKind::EqEq => (BinaryOp::Eq, 11, 12),
                TokenKind::NotEq => (BinaryOp::NotEq, 11, 12),
                TokenKind::StrictEq => (BinaryOp::StrictEq, 11, 12),
                TokenKind::StrictNotEq => (BinaryOp::StrictNotEq, 11, 12),
                TokenKind::Lt => (BinaryOp::Lt, 13, 14),
                TokenKind::LtEq => (BinaryOp::LtEq, 13, 14),
                TokenKind::Gt => (BinaryOp::Gt, 13, 14),
                TokenKind::GtEq => (BinaryOp::GtEq, 13, 14),
                TokenKind::Is if self.dialect == AsDialect::As3 => (BinaryOp::Is, 13, 14),
                TokenKind::As if self.dialect == AsDialect::As3 => (BinaryOp::As, 13, 14),
                TokenKind::Shl => (BinaryOp::Shl, 15, 16),
                TokenKind::Shr => (BinaryOp::Shr, 15, 16),
                TokenKind::Ushr => (BinaryOp::Ushr, 15, 16),
                TokenKind::Plus => (BinaryOp::Add, 17, 18),
                TokenKind::Minus => (BinaryOp::Sub, 17, 18),
                TokenKind::Star => (BinaryOp::Mul, 19, 20),
                TokenKind::Slash => (BinaryOp::Div, 19, 20),
                TokenKind::Percent => (BinaryOp::Mod, 19, 20),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            self.parser_advance();
            self.parser_charge_node()?;
            let right = self.parser_parse_binary(r_bp)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parser_parse_unary(&mut self) -> Result<Expr, AsCompilerError> {
        self.parser_enter_scope()?;
        self.parser_charge_node()?;
        let res = self.parser_parse_unary_inner();
        self.parser_exit_scope();
        res
    }

    fn parser_parse_unary_inner(&mut self) -> Result<Expr, AsCompilerError> {
        if self.parser_match(&TokenKind::Minus) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            });
        }
        if self.parser_match(&TokenKind::Plus) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Pos,
                expr: Box::new(expr),
            });
        }
        if self.parser_match(&TokenKind::Bang) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
            });
        }
        if self.parser_match(&TokenKind::Tilde) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::BitNot,
                expr: Box::new(expr),
            });
        }
        if self.parser_match(&TokenKind::PlusPlus) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreInc,
                expr: Box::new(expr),
            });
        }
        if self.parser_match(&TokenKind::MinusMinus) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::PreDec,
                expr: Box::new(expr),
            });
        }
        if self.parser_match(&TokenKind::Typeof) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Typeof,
                expr: Box::new(expr),
            });
        }
        if self.parser_match(&TokenKind::Delete) {
            let expr = self.parser_parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Delete,
                expr: Box::new(expr),
            });
        }

        self.parser_parse_postfix()
    }

    fn parser_parse_postfix(&mut self) -> Result<Expr, AsCompilerError> {
        let mut expr = self.parser_parse_primary()?;

        loop {
            if self.parser_match(&TokenKind::Dot) {
                let prop = self.parser_match_ident()?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: prop,
                };
            } else if self.parser_match(&TokenKind::LBracket) {
                let index = self.parser_parse_expression()?;
                self.parser_expect(&TokenKind::RBracket)?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.parser_match(&TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.parser_check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parser_parse_expression()?);
                        if !self.parser_match(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.parser_expect(&TokenKind::RParen)?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.parser_match(&TokenKind::PlusPlus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostInc,
                    expr: Box::new(expr),
                };
            } else if self.parser_match(&TokenKind::MinusMinus) {
                expr = Expr::Unary {
                    op: UnaryOp::PostDec,
                    expr: Box::new(expr),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parser_parse_primary(&mut self) -> Result<Expr, AsCompilerError> {
        // Number
        if let TokenKind::Number(val) = self.parser_peek() {
            let val = *val;
            self.parser_advance();
            return Ok(Expr::Number(val));
        }

        // String
        if let TokenKind::StringLiteral(s) = self.parser_peek() {
            let s = s.clone();
            self.parser_advance();
            return Ok(Expr::StringLit(s));
        }

        // Boolean
        if let TokenKind::BoolLiteral(b) = self.parser_peek() {
            let b = *b;
            self.parser_advance();
            return Ok(Expr::BoolLit(b));
        }

        // Null / Undefined
        if self.parser_match(&TokenKind::Null) {
            return Ok(Expr::Null);
        }
        if self.parser_match(&TokenKind::Undefined) {
            return Ok(Expr::Undefined);
        }

        // This
        if self.parser_match(&TokenKind::This) {
            return Ok(Expr::This);
        }

        // Flash AS1 Paths
        if self.parser_match(&TokenKind::Root) {
            return Ok(Expr::Root);
        }
        if self.parser_match(&TokenKind::Parent) {
            return Ok(Expr::Parent);
        }
        if self.parser_match(&TokenKind::Global) {
            return Ok(Expr::Global);
        }

        // Eval
        if self.parser_match(&TokenKind::Eval) {
            self.parser_expect(&TokenKind::LParen)?;
            let arg = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::RParen)?;
            return Ok(Expr::Eval(Box::new(arg)));
        }

        // New
        if self.parser_match(&TokenKind::New) {
            self.parser_enter_scope()?;
            self.parser_charge_node()?;
            let res = self.parser_parse_new_inner();
            self.parser_exit_scope();
            return res;
        }

        // Parenthesized expression
        if self.parser_match(&TokenKind::LParen) {
            self.parser_enter_scope()?;
            self.parser_charge_node()?;
            let expr = self.parser_parse_expression()?;
            self.parser_expect(&TokenKind::RParen)?;
            self.parser_exit_scope();
            return Ok(expr);
        }

        // Array literal: [ ... ]
        if self.parser_match(&TokenKind::LBracket) {
            return self.parser_parse_array_lit();
        }

        // Object literal: { ... }
        if self.parser_match(&TokenKind::LBrace) {
            return self.parser_parse_object_lit();
        }

        // Identifier
        if let TokenKind::Ident(name) = self.parser_peek() {
            let name = name.clone();
            self.parser_advance();
            return Ok(Expr::Ident(name));
        }

        Err(AsCompilerError::SyntaxError(format!(
            "unexpected token {:?} in expression at line {}",
            self.parser_peek(),
            self.parser_peek_token().span.line
        )))
    }

    fn parser_parse_new_inner(&mut self) -> Result<Expr, AsCompilerError> {
        // Check for new Vector.<T>() in AS3
        if self.dialect == AsDialect::As3 && self.parser_match(&TokenKind::Vector) {
            self.parser_expect(&TokenKind::DotLt)?;
            let item_type = self.parser_parse_type_annotation()?;
            self.parser_expect(&TokenKind::Gt)?;
            self.parser_expect(&TokenKind::LParen)?;
            let mut items = Vec::new();
            if !self.parser_check(&TokenKind::RParen) {
                loop {
                    items.push(self.parser_parse_expression()?);
                    if !self.parser_match(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.parser_expect(&TokenKind::RParen)?;
            return Ok(Expr::VectorLit { item_type, items });
        }

        let callee = self.parser_parse_primary()?;
        let mut args = Vec::new();
        if self.parser_match(&TokenKind::LParen) {
            if !self.parser_check(&TokenKind::RParen) {
                loop {
                    args.push(self.parser_parse_expression()?);
                    if !self.parser_match(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.parser_expect(&TokenKind::RParen)?;
        }
        Ok(Expr::New {
            callee: Box::new(callee),
            args,
        })
    }

    #[inline(never)]
    fn parser_parse_array_lit(&mut self) -> Result<Expr, AsCompilerError> {
        let mut items = Vec::new();
        if !self.parser_check(&TokenKind::RBracket) {
            loop {
                items.push(self.parser_parse_expression()?);
                if !self.parser_match(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.parser_expect(&TokenKind::RBracket)?;
        Ok(Expr::ArrayLit(items))
    }

    #[inline(never)]
    fn parser_parse_object_lit(&mut self) -> Result<Expr, AsCompilerError> {
        let mut props = Vec::new();
        if !self.parser_check(&TokenKind::RBrace) {
            loop {
                let key = match self.parser_peek() {
                    TokenKind::Ident(id) => {
                        let s = id.clone();
                        self.parser_advance();
                        s
                    }
                    TokenKind::StringLiteral(s) => {
                        let s = s.clone();
                        self.parser_advance();
                        s
                    }
                    other => {
                        return Err(AsCompilerError::SyntaxError(format!(
                            "expected object property name, found {:?} at line {}",
                            other,
                            self.parser_peek_token().span.line
                        )));
                    }
                };
                self.parser_expect(&TokenKind::Colon)?;
                let val = self.parser_parse_expression()?;
                props.push((key, val));
                if !self.parser_match(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.parser_expect(&TokenKind::RBrace)?;
        Ok(Expr::ObjectLit(props))
    }
}

/// PBL Parser — Parses tokens into an AST.

use crate::ast::*;
use crate::lexer::{Token, TokenKind};
use crate::errors::{PblError, PblResult};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    #[allow(dead_code)]
    file: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: &str) -> Self {
        Self {
            tokens,
            pos: 0,
            file: file.to_string(),
        }
    }

    pub fn parse(&mut self) -> PblResult<Program> {
        let mut items = Vec::new();

        while !self.is_at_end() {
            if self.check(&TokenKind::Eof) {
                break;
            }
            let item = self.parse_item()?;
            items.push(item);
        }

        Ok(Program { items })
    }

    // -----------------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------------

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if !self.is_at_end() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.tokens[self.pos].kind) == std::mem::discriminant(kind)
    }

    fn check_identifier(&self) -> bool {
        matches!(self.tokens[self.pos].kind, TokenKind::Identifier(_))
    }

    fn expect(&mut self, kind: &TokenKind) -> PblResult<Token> {
        if self.check(kind) {
            Ok(self.advance().clone())
        } else {
            Err(PblError::parser(
                format!("expected '{}', found '{}'", kind, self.peek_kind()),
                self.peek().span.clone(),
            ))
        }
    }

    fn expect_identifier(&mut self) -> PblResult<(String, Span)> {
        let tok = self.advance().clone();
        match tok.kind {
            TokenKind::Identifier(name) => Ok((name, tok.span)),
            _ => Err(PblError::parser(
                format!("expected identifier, found '{}'", tok.kind),
                tok.span,
            )),
        }
    }

    fn current_span(&self) -> Span {
        self.peek().span.clone()
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    // -----------------------------------------------------------------------
    // Top-level items
    // -----------------------------------------------------------------------

    fn parse_item(&mut self) -> PblResult<Item> {
        // Handle pub modifier
        let is_pub = self.match_token(&TokenKind::Pub);

        match self.peek_kind().clone() {
            TokenKind::Fn => {
                let func = self.parse_function(is_pub)?;
                Ok(Item::Function(func))
            }
            TokenKind::Struct => {
                let s = self.parse_struct(is_pub)?;
                Ok(Item::Struct(s))
            }
            TokenKind::Enum => {
                let e = self.parse_enum(is_pub)?;
                Ok(Item::Enum(e))
            }
            TokenKind::Crate => {
                if is_pub {
                    return Err(PblError::parser(
                        "crate declarations cannot be pub",
                        self.current_span(),
                    ));
                }
                let c = self.parse_crate_decl()?;
                Ok(Item::CrateDecl(c))
            }
            TokenKind::Use => {
                if is_pub {
                    return Err(PblError::parser(
                        "use declarations cannot be pub in PBL",
                        self.current_span(),
                    ));
                }
                let u = self.parse_use_decl()?;
                Ok(Item::UseDecl(u))
            }
            TokenKind::Rust | TokenKind::RustInline => {
                if is_pub {
                    return Err(PblError::parser(
                        "rust blocks cannot be pub",
                        self.current_span(),
                    ));
                }
                let r = self.parse_rust_block()?;
                Ok(Item::RustBlock(r))
            }
            TokenKind::Project => {
                if is_pub {
                    return Err(PblError::parser(
                        "project config cannot be pub",
                        self.current_span(),
                    ));
                }
                let p = self.parse_project_config()?;
                Ok(Item::ProjectConfig(p))
            }
            _ => {
                if is_pub {
                    return Err(PblError::parser(
                        format!("unexpected token '{}' after 'pub'", self.peek_kind()),
                        self.current_span(),
                    ));
                }
                let stmt = self.parse_statement()?;
                Ok(Item::Statement(stmt))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Functions
    // -----------------------------------------------------------------------

    fn parse_function(&mut self, is_pub: bool) -> PblResult<FunctionDef> {
        let span = self.current_span();
        self.expect(&TokenKind::Fn)?;
        let (name, _) = self.expect_identifier()?;

        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(FunctionDef {
            name,
            params,
            return_type,
            body,
            is_pub,
            span,
        })
    }

    fn parse_params(&mut self) -> PblResult<Vec<Param>> {
        let mut params = Vec::new();
        if self.check(&TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let span = self.current_span();
            let (name, _) = self.expect_identifier()?;
            self.expect(&TokenKind::Colon)?;
            let type_ann = self.parse_type()?;
            params.push(Param {
                name,
                type_ann,
                span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    fn parse_type(&mut self) -> PblResult<TypeAnnotation> {
        // Check for reference types
        if self.match_token(&TokenKind::Ampersand) {
            if self.match_token(&TokenKind::Mut) {
                let inner = self.parse_type()?;
                return Ok(TypeAnnotation::MutReference(Box::new(inner)));
            }
            let inner = self.parse_type()?;
            return Ok(TypeAnnotation::Reference(Box::new(inner)));
        }

        // Check for optional type
        if self.match_token(&TokenKind::Question) {
            let inner = self.parse_type()?;
            return Ok(TypeAnnotation::Optional(Box::new(inner)));
        }

        // Check for array type [T]
        if self.match_token(&TokenKind::LBracket) {
            let inner = self.parse_type()?;
            self.expect(&TokenKind::RBracket)?;
            return Ok(TypeAnnotation::Array(Box::new(inner)));
        }

        // Check for tuple type (T, U, ...)
        if self.check(&TokenKind::LParen) {
            self.advance();
            if self.check(&TokenKind::RParen) {
                self.advance();
                return Ok(TypeAnnotation::Unit);
            }
            let mut types = vec![self.parse_type()?];
            while self.match_token(&TokenKind::Comma) {
                types.push(self.parse_type()?);
            }
            self.expect(&TokenKind::RParen)?;
            if types.len() == 1 {
                return Ok(types.into_iter().next().unwrap());
            }
            return Ok(TypeAnnotation::Tuple(types));
        }

        // Simple type or generic
        let (name, _) = self.expect_identifier()?;

        // Check for generic params: Name<T, U>
        if self.check(&TokenKind::Lt) {
            self.advance();
            let mut type_params = vec![self.parse_type()?];
            while self.match_token(&TokenKind::Comma) {
                type_params.push(self.parse_type()?);
            }
            self.expect(&TokenKind::Gt)?;
            Ok(TypeAnnotation::Generic(name, type_params))
        } else {
            Ok(TypeAnnotation::Simple(name))
        }
    }

    // -----------------------------------------------------------------------
    // Blocks
    // -----------------------------------------------------------------------

    fn parse_block(&mut self) -> PblResult<Block> {
        let span = self.current_span();
        self.expect(&TokenKind::LBrace)?;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }

        self.expect(&TokenKind::RBrace)?;
        Ok(Block { statements, span })
    }

    // -----------------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------------

    fn parse_statement(&mut self) -> PblResult<Statement> {
        match self.peek_kind().clone() {
            TokenKind::Let => self.parse_let(),
            TokenKind::Mut => self.parse_mut(),
            TokenKind::Return => self.parse_return(),
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::While => self.parse_while(),
            TokenKind::Break => {
                let span = self.current_span();
                self.advance();
                Ok(Statement::Break(span))
            }
            TokenKind::Continue => {
                let span = self.current_span();
                self.advance();
                Ok(Statement::Continue(span))
            }
            TokenKind::Rust | TokenKind::RustInline => {
                let r = self.parse_rust_block()?;
                Ok(Statement::Expression(Expression::RustInline(r.code, r.span)))
            }
            _ => {
                let expr = self.parse_expression()?;

                // Check for assignment operators
                let assign_op = match self.peek_kind() {
                    TokenKind::Assign => Some(AssignOp::Assign),
                    TokenKind::PlusAssign => Some(AssignOp::AddAssign),
                    TokenKind::MinusAssign => Some(AssignOp::SubAssign),
                    TokenKind::StarAssign => Some(AssignOp::MulAssign),
                    TokenKind::SlashAssign => Some(AssignOp::DivAssign),
                    _ => None,
                };

                if let Some(op) = assign_op {
                    let span = self.current_span();
                    self.advance(); // consume the operator
                    let value = self.parse_expression()?;
                    Ok(Statement::Assignment(Assignment {
                        target: expr,
                        op,
                        value,
                        span,
                    }))
                } else {
                    Ok(Statement::Expression(expr))
                }
            }
        }
    }

    fn parse_let(&mut self) -> PblResult<Statement> {
        let span = self.current_span();
        self.expect(&TokenKind::Let)?;
        let (name, _) = self.expect_identifier()?;

        let type_ann = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&TokenKind::Assign)?;
        let value = self.parse_expression()?;

        Ok(Statement::Let(LetBinding {
            name,
            type_ann,
            value,
            span,
        }))
    }

    fn parse_mut(&mut self) -> PblResult<Statement> {
        let span = self.current_span();
        self.expect(&TokenKind::Mut)?;
        let (name, _) = self.expect_identifier()?;

        let type_ann = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(&TokenKind::Assign)?;
        let value = self.parse_expression()?;

        Ok(Statement::Mut(MutBinding {
            name,
            type_ann,
            value,
            span,
        }))
    }

    fn parse_return(&mut self) -> PblResult<Statement> {
        let span = self.current_span();
        self.expect(&TokenKind::Return)?;

        let value = if self.check(&TokenKind::RBrace) || self.is_at_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };

        Ok(Statement::Return(ReturnStmt { value, span }))
    }

    fn parse_if(&mut self) -> PblResult<Statement> {
        let span = self.current_span();
        self.expect(&TokenKind::If)?;
        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;

        let mut else_if_blocks = Vec::new();
        let mut else_block = None;

        while self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                self.advance();
                let cond = self.parse_expression()?;
                let block = self.parse_block()?;
                else_if_blocks.push((cond, block));
            } else {
                else_block = Some(self.parse_block()?);
                break;
            }
        }

        Ok(Statement::If(IfStmt {
            condition,
            then_block,
            else_if_blocks,
            else_block,
            span,
        }))
    }

    fn parse_for(&mut self) -> PblResult<Statement> {
        let span = self.current_span();
        self.expect(&TokenKind::For)?;
        let (variable, _) = self.expect_identifier()?;
        self.expect(&TokenKind::In)?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Statement::ForLoop(ForLoop {
            variable,
            iterable,
            body,
            span,
        }))
    }

    fn parse_while(&mut self) -> PblResult<Statement> {
        let span = self.current_span();
        self.expect(&TokenKind::While)?;
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Statement::WhileLoop(WhileLoop {
            condition,
            body,
            span,
        }))
    }

    // -----------------------------------------------------------------------
    // Expressions (Pratt parser)
    // -----------------------------------------------------------------------

    fn parse_expression(&mut self) -> PblResult<Expression> {
        self.parse_binary(0)
    }

    fn parse_binary(&mut self, min_prec: u8) -> PblResult<Expression> {
        let mut left = self.parse_unary()?;

        while let Some(op) = self.peek_binary_op() {
            let prec = op.precedence();
            if prec < min_prec {
                break;
            }
            let span = self.current_span();
            self.advance(); // consume operator
            let right = self.parse_binary(prec + 1)?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn peek_binary_op(&self) -> Option<BinOp> {
        match self.peek_kind() {
            TokenKind::Plus => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            TokenKind::Percent => Some(BinOp::Mod),
            TokenKind::Eq => Some(BinOp::Eq),
            TokenKind::NotEq => Some(BinOp::NotEq),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::LtEq => Some(BinOp::LtEq),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::GtEq => Some(BinOp::GtEq),
            TokenKind::And => Some(BinOp::And),
            TokenKind::Or => Some(BinOp::Or),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> PblResult<Expression> {
        match self.peek_kind().clone() {
            TokenKind::Minus => {
                let span = self.current_span();
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Not => {
                let span = self.current_span();
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> PblResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek_kind().clone() {
                TokenKind::LParen => {
                    // Function call
                    let span = self.current_span();
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&TokenKind::RParen)?;
                    expr = Expression::FunctionCall {
                        name: Box::new(expr),
                        args,
                        span,
                    };
                }
                TokenKind::Dot => {
                    let span = self.current_span();
                    self.advance();
                    let (field, _) = self.expect_identifier()?;

                    if self.check(&TokenKind::LParen) {
                        // Method call
                        self.advance();
                        let args = self.parse_args()?;
                        self.expect(&TokenKind::RParen)?;
                        expr = Expression::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span,
                        };
                    } else {
                        // Field access
                        expr = Expression::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span,
                        };
                    }
                }
                TokenKind::LBracket => {
                    // Index access
                    let span = self.current_span();
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&TokenKind::RBracket)?;
                    expr = Expression::IndexAccess {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> PblResult<Expression> {
        let tok = self.advance().clone();

        match tok.kind {
            TokenKind::IntLiteral(n) => Ok(Expression::IntLiteral(n, tok.span)),
            TokenKind::FloatLiteral(n) => Ok(Expression::FloatLiteral(n, tok.span)),
            TokenKind::StringLiteral(s) => Ok(Expression::StringLiteral(s, tok.span)),
            TokenKind::CharLiteral(c) => Ok(Expression::CharLiteral(c, tok.span)),
            TokenKind::True => Ok(Expression::BoolLiteral(true, tok.span)),
            TokenKind::False => Ok(Expression::BoolLiteral(false, tok.span)),

            TokenKind::Identifier(name) => {
                // Check for path access (Name::Variant)
                if self.check(&TokenKind::ColonColon) {
                    let mut segments = vec![name];
                    while self.match_token(&TokenKind::ColonColon) {
                        let (seg, _) = self.expect_identifier()?;
                        segments.push(seg);
                    }
                    Ok(Expression::PathAccess {
                        segments,
                        span: tok.span,
                    })
                }
                // Check for struct literal: Name { field: value }
                else if self.check(&TokenKind::LBrace) && self.is_struct_literal_context(&name) {
                    self.advance(); // {
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                        let (field_name, _) = self.expect_identifier()?;
                        self.expect(&TokenKind::Colon)?;
                        let value = self.parse_expression()?;
                        fields.push((field_name, value));
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RBrace)?;
                    Ok(Expression::StructLiteral {
                        name,
                        fields,
                        span: tok.span,
                    })
                } else {
                    Ok(Expression::Identifier(name, tok.span))
                }
            }

            TokenKind::LParen => {
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }

            TokenKind::LBracket => {
                // Array literal
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expression::ArrayLiteral(elements, tok.span))
            }

            TokenKind::Pipe => {
                // Closure: |params| expr
                let mut params = Vec::new();
                if !self.check(&TokenKind::Pipe) {
                    loop {
                        let (name, _) = self.expect_identifier()?;
                        let type_ann = if self.match_token(&TokenKind::Colon) {
                            Some(self.parse_type()?)
                        } else {
                            None
                        };
                        params.push(ClosureParam { name, type_ann });
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::Pipe)?;
                let body = self.parse_expression()?;
                Ok(Expression::Closure {
                    params,
                    body: Box::new(body),
                    span: tok.span,
                })
            }

            _ => Err(PblError::parser(
                format!("unexpected token '{}'", tok.kind),
                tok.span,
            )),
        }
    }

    fn parse_args(&mut self) -> PblResult<Vec<Expression>> {
        let mut args = Vec::new();
        if self.check(&TokenKind::RParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }

        Ok(args)
    }

    /// Heuristic: identifiers starting with uppercase are likely struct constructors.
    fn is_struct_literal_context(&self, name: &str) -> bool {
        name.chars().next().map_or(false, |c| c.is_uppercase())
    }

    // -----------------------------------------------------------------------
    // Structs
    // -----------------------------------------------------------------------

    fn parse_struct(&mut self, is_pub: bool) -> PblResult<StructDef> {
        let span = self.current_span();
        self.expect(&TokenKind::Struct)?;
        let (name, _) = self.expect_identifier()?;
        self.expect(&TokenKind::LBrace)?;

        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let field_pub = self.match_token(&TokenKind::Pub);
            let field_span = self.current_span();
            let (field_name, _) = self.expect_identifier()?;
            self.expect(&TokenKind::Colon)?;
            let type_ann = self.parse_type()?;
            fields.push(StructField {
                name: field_name,
                type_ann,
                is_pub: field_pub,
                span: field_span,
            });
            // Allow trailing comma
            self.match_token(&TokenKind::Comma);
        }

        self.expect(&TokenKind::RBrace)?;
        Ok(StructDef {
            name,
            fields,
            is_pub,
            span,
        })
    }

    // -----------------------------------------------------------------------
    // Enums
    // -----------------------------------------------------------------------

    fn parse_enum(&mut self, is_pub: bool) -> PblResult<EnumDef> {
        let span = self.current_span();
        self.expect(&TokenKind::Enum)?;
        let (name, _) = self.expect_identifier()?;
        self.expect(&TokenKind::LBrace)?;

        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let variant_span = self.current_span();
            let (variant_name, _) = self.expect_identifier()?;

            let fields = if self.match_token(&TokenKind::LParen) {
                let mut types = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        types.push(self.parse_type()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RParen)?;
                Some(types)
            } else {
                None
            };

            variants.push(EnumVariant {
                name: variant_name,
                fields,
                span: variant_span,
            });
            self.match_token(&TokenKind::Comma);
        }

        self.expect(&TokenKind::RBrace)?;
        Ok(EnumDef {
            name,
            variants,
            is_pub,
            span,
        })
    }

    // -----------------------------------------------------------------------
    // Crate declarations
    // -----------------------------------------------------------------------

    fn parse_crate_decl(&mut self) -> PblResult<CrateDecl> {
        let span = self.current_span();
        self.expect(&TokenKind::Crate)?;
        let (name, _) = self.expect_identifier()?;
        self.expect(&TokenKind::Assign)?;

        let version_tok = self.advance().clone();
        let version = match version_tok.kind {
            TokenKind::StringLiteral(v) => v,
            _ => {
                return Err(PblError::parser(
                    "expected version string for crate declaration",
                    version_tok.span,
                ));
            }
        };

        // Parse optional features
        let mut features = Vec::new();
        if self.check_identifier() {
            if let TokenKind::Identifier(ref id) = self.peek_kind() {
                if id == "features" {
                    self.advance();
                    self.expect(&TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) && !self.is_at_end() {
                        let feat_tok = self.advance().clone();
                        match feat_tok.kind {
                            TokenKind::StringLiteral(f) => features.push(f),
                            _ => {
                                return Err(PblError::parser(
                                    "expected feature name string",
                                    feat_tok.span,
                                ));
                            }
                        }
                        self.match_token(&TokenKind::Comma);
                    }
                    self.expect(&TokenKind::RBracket)?;
                }
            }
        }

        // For now, default to CratesIo. Path and Git sources would need additional syntax.
        let source = CrateSource::CratesIo;

        Ok(CrateDecl {
            name,
            version,
            features,
            source,
            span,
        })
    }

    // -----------------------------------------------------------------------
    // Use declarations
    // -----------------------------------------------------------------------

    fn parse_use_decl(&mut self) -> PblResult<UseDecl> {
        let span = self.current_span();
        self.expect(&TokenKind::Use)?;

        let mut path = Vec::new();
        let (first, _) = self.expect_identifier()?;
        path.push(first);

        while self.match_token(&TokenKind::ColonColon) {
            let (seg, _) = self.expect_identifier()?;
            path.push(seg);
        }

        let alias = if self.match_token(&TokenKind::As) {
            let (alias_name, _) = self.expect_identifier()?;
            Some(alias_name)
        } else {
            None
        };

        Ok(UseDecl { path, alias, span })
    }

    // -----------------------------------------------------------------------
    // Rust blocks
    // -----------------------------------------------------------------------

    fn parse_rust_block(&mut self) -> PblResult<RustBlock> {
        let span = self.current_span();
        // Consume 'rust' or 'rust_inline'
        self.advance();

        self.expect(&TokenKind::LBrace)?;

        // Collect everything until matching closing brace
        let mut code = String::new();
        let mut depth = 1;

        while !self.is_at_end() && depth > 0 {
            let tok = self.advance().clone();
            match tok.kind {
                TokenKind::LBrace => {
                    depth += 1;
                    code.push('{');
                }
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth > 0 {
                        code.push('}');
                    }
                }
                _ => {
                    code.push_str(&format!("{} ", tok.kind));
                }
            }
        }

        Ok(RustBlock {
            code: code.trim().to_string(),
            span,
        })
    }

    // -----------------------------------------------------------------------
    // Project configuration
    // -----------------------------------------------------------------------

    fn parse_project_config(&mut self) -> PblResult<ProjectConfig> {
        let span = self.current_span();
        self.expect(&TokenKind::Project)?;
        self.expect(&TokenKind::LBrace)?;

        let mut settings = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let (key, _) = self.expect_identifier()?;
            self.expect(&TokenKind::Colon)?;
            let (value, _) = self.expect_identifier()?;

            match key.as_str() {
                "mode" => settings.push(ProjectSetting::Mode(value)),
                "entry" => settings.push(ProjectSetting::Entry(value)),
                _ => {
                    return Err(PblError::parser(
                        format!("unknown project setting '{}'", key),
                        self.current_span(),
                    ));
                }
            }
            self.match_token(&TokenKind::Comma);
        }

        self.expect(&TokenKind::RBrace)?;
        Ok(ProjectConfig { settings, span })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> PblResult<Program> {
        let mut lexer = Lexer::new(input, "test.pbl");
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens, "test.pbl");
        parser.parse()
    }

    #[test]
    fn test_parse_hello_world() {
        let program = parse(r#"fn main() { print("Hello, Polar Bear!") }"#).unwrap();
        assert_eq!(program.items.len(), 1);
        match &program.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "main");
                assert!(f.params.is_empty());
                assert!(f.return_type.is_none());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_variables() {
        let program = parse(r#"
            let name = "Polar Bear"
            mut score = 0
        "#).unwrap();
        assert_eq!(program.items.len(), 2);
    }

    #[test]
    fn test_parse_function_with_return() {
        let program = parse("fn add(a: i32, b: i32) -> i32 { return a + b }").unwrap();
        match &program.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert!(f.return_type.is_some());
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_struct() {
        let program = parse("struct PolarBear { name: str, age: i32 }").unwrap();
        match &program.items[0] {
            Item::Struct(s) => {
                assert_eq!(s.name, "PolarBear");
                assert_eq!(s.fields.len(), 2);
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_parse_enum() {
        let program = parse("enum Animal { PolarBear, Penguin, Fox }").unwrap();
        match &program.items[0] {
            Item::Enum(e) => {
                assert_eq!(e.name, "Animal");
                assert_eq!(e.variants.len(), 3);
            }
            _ => panic!("expected enum"),
        }
    }

    #[test]
    fn test_parse_crate_decl() {
        let program = parse(r#"crate serde = "1" features ["derive"]"#).unwrap();
        match &program.items[0] {
            Item::CrateDecl(c) => {
                assert_eq!(c.name, "serde");
                assert_eq!(c.version, "1");
                assert_eq!(c.features, vec!["derive"]);
            }
            _ => panic!("expected crate decl"),
        }
    }

    #[test]
    fn test_parse_use_decl() {
        let program = parse("use serde::Deserialize").unwrap();
        match &program.items[0] {
            Item::UseDecl(u) => {
                assert_eq!(u.path, vec!["serde", "Deserialize"]);
            }
            _ => panic!("expected use decl"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let program = parse(r#"
            if x > 10 {
                print("big")
            } else {
                print("small")
            }
        "#).unwrap();
        match &program.items[0] {
            Item::Statement(Statement::If(if_stmt)) => {
                assert!(if_stmt.else_block.is_some());
            }
            _ => panic!("expected if statement"),
        }
    }

    #[test]
    fn test_parse_for_loop() {
        let program = parse(r#"
            for item in items {
                print(item)
            }
        "#).unwrap();
        match &program.items[0] {
            Item::Statement(Statement::ForLoop(f)) => {
                assert_eq!(f.variable, "item");
            }
            _ => panic!("expected for loop"),
        }
    }

    #[test]
    fn test_parse_while_loop() {
        let program = parse(r#"
            while x < 10 {
                x += 1
            }
        "#).unwrap();
        match &program.items[0] {
            Item::Statement(Statement::WhileLoop(_)) => {}
            _ => panic!("expected while loop"),
        }
    }

    #[test]
    fn test_parse_project_config() {
        let program = parse("project { mode: no_std }").unwrap();
        match &program.items[0] {
            Item::ProjectConfig(p) => {
                assert_eq!(p.settings.len(), 1);
            }
            _ => panic!("expected project config"),
        }
    }
}

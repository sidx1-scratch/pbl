/// PBL Semantic Analysis — Validates the AST before code generation.

use crate::ast::*;
use crate::errors::PblError;
use std::collections::{HashMap, HashSet};

/// Semantic context tracking known symbols.
struct SemanticContext {
    /// Stack of scopes. Each scope maps names to their kind.
    scopes: Vec<HashMap<String, SymbolKind>>,
    /// Known struct names.
    structs: HashSet<String>,
    /// Known enum names.
    enums: HashSet<String>,
    /// Known function names.
    functions: HashSet<String>,
    /// Collected errors (we try to report as many as possible).
    errors: Vec<PblError>,
}

#[derive(Debug, Clone, PartialEq)]
enum SymbolKind {
    Immutable,
    Mutable,
    Parameter,
    Function,
    LoopVariable,
}

impl SemanticContext {
    fn new() -> Self {
        let mut ctx = Self {
            scopes: Vec::new(),
            structs: HashSet::new(),
            enums: HashSet::new(),
            functions: HashSet::new(),
            errors: Vec::new(),
        };
        // Push a global scope with built-in functions
        let mut builtins = HashMap::new();
        builtins.insert("print".to_string(), SymbolKind::Function);
        builtins.insert("println".to_string(), SymbolKind::Function);
        builtins.insert("format".to_string(), SymbolKind::Function);
        builtins.insert("len".to_string(), SymbolKind::Function);
        builtins.insert("push".to_string(), SymbolKind::Function);
        builtins.insert("pop".to_string(), SymbolKind::Function);
        builtins.insert("to_string".to_string(), SymbolKind::Function);
        ctx.scopes.push(builtins);
        ctx
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: &str, kind: SymbolKind) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), kind);
        }
    }

    fn lookup(&self, name: &str) -> Option<&SymbolKind> {
        for scope in self.scopes.iter().rev() {
            if let Some(kind) = scope.get(name) {
                return Some(kind);
            }
        }
        None
    }

    fn error(&mut self, msg: impl Into<String>, span: Span) {
        self.errors.push(PblError::semantic(msg, span));
    }

    fn error_with_hint(&mut self, msg: impl Into<String>, span: Span, hint: impl Into<String>) {
        self.errors.push(PblError::semantic(msg, span).with_hint(hint));
    }
}

/// Analyze a program, returning collected errors.
pub fn analyze(program: &Program) -> Vec<PblError> {
    let mut ctx = SemanticContext::new();

    // First pass: register all top-level names
    for item in &program.items {
        match item {
            Item::Function(f) => {
                if !ctx.functions.insert(f.name.clone()) {
                    ctx.error(
                        format!("duplicate function '{}'", f.name),
                        f.span.clone(),
                    );
                }
                ctx.define(&f.name, SymbolKind::Function);
            }
            Item::Struct(s) => {
                if !ctx.structs.insert(s.name.clone()) {
                    ctx.error(
                        format!("duplicate struct '{}'", s.name),
                        s.span.clone(),
                    );
                }
            }
            Item::Enum(e) => {
                if !ctx.enums.insert(e.name.clone()) {
                    ctx.error(
                        format!("duplicate enum '{}'", e.name),
                        e.span.clone(),
                    );
                }
            }
            _ => {}
        }
    }

    // Second pass: analyze each item
    for item in &program.items {
        analyze_item(&mut ctx, item);
    }

    ctx.errors
}

fn analyze_item(ctx: &mut SemanticContext, item: &Item) {
    match item {
        Item::Function(f) => analyze_function(ctx, f),
        Item::Struct(s) => analyze_struct(ctx, s),
        Item::Enum(e) => analyze_enum(ctx, e),
        Item::CrateDecl(c) => analyze_crate_decl(ctx, c),
        Item::UseDecl(u) => analyze_use_decl(ctx, u),
        Item::RustBlock(_) => {} // Rust blocks are pass-through
        Item::ProjectConfig(p) => analyze_project_config(ctx, p),
        Item::Statement(s) => analyze_statement(ctx, s),
    }
}

fn analyze_function(ctx: &mut SemanticContext, func: &FunctionDef) {
    ctx.push_scope();

    // Register parameters
    for param in &func.params {
        ctx.define(&param.name, SymbolKind::Parameter);
        validate_type(ctx, &param.type_ann, &param.span);
    }

    // Validate return type
    if let Some(ref ret_type) = func.return_type {
        validate_type(ctx, ret_type, &func.span);
    }

    // Check function body
    analyze_block(ctx, &func.body);

    ctx.pop_scope();
}

fn analyze_struct(ctx: &mut SemanticContext, s: &StructDef) {
    let mut field_names = HashSet::new();
    for field in &s.fields {
        if !field_names.insert(&field.name) {
            ctx.error(
                format!("duplicate field '{}' in struct '{}'", field.name, s.name),
                field.span.clone(),
            );
        }
        validate_type(ctx, &field.type_ann, &field.span);
    }
}

fn analyze_enum(ctx: &mut SemanticContext, e: &EnumDef) {
    let mut variant_names = HashSet::new();
    for variant in &e.variants {
        if !variant_names.insert(&variant.name) {
            ctx.error(
                format!("duplicate variant '{}' in enum '{}'", variant.name, e.name),
                variant.span.clone(),
            );
        }
        if let Some(ref fields) = variant.fields {
            for field_type in fields {
                validate_type(ctx, field_type, &variant.span);
            }
        }
    }
}

fn analyze_crate_decl(ctx: &mut SemanticContext, c: &CrateDecl) {
    if c.name.is_empty() {
        ctx.error("crate name cannot be empty", c.span.clone());
    }
    if c.version.is_empty() {
        ctx.error("crate version cannot be empty", c.span.clone());
    }
}

fn analyze_use_decl(ctx: &mut SemanticContext, u: &UseDecl) {
    if u.path.is_empty() {
        ctx.error("use path cannot be empty", u.span.clone());
    }
    // Register the imported name in the current scope
    let imported_name = u.alias.as_ref().unwrap_or_else(|| u.path.last().unwrap());
    ctx.define(imported_name, SymbolKind::Immutable);
}

fn analyze_project_config(ctx: &mut SemanticContext, p: &ProjectConfig) {
    for setting in &p.settings {
        match setting {
            ProjectSetting::Mode(mode) => {
                if mode != "no_std" && mode != "std" {
                    ctx.error_with_hint(
                        format!("unknown project mode '{}'", mode),
                        p.span.clone(),
                        "valid modes are 'no_std' and 'std'",
                    );
                }
            }
            ProjectSetting::Entry(_) => {}
        }
    }
}

fn analyze_block(ctx: &mut SemanticContext, block: &Block) {
    for stmt in &block.statements {
        analyze_statement(ctx, stmt);
    }
}

fn analyze_statement(ctx: &mut SemanticContext, stmt: &Statement) {
    match stmt {
        Statement::Let(binding) => {
            analyze_expression(ctx, &binding.value);
            if let Some(ref type_ann) = binding.type_ann {
                validate_type(ctx, type_ann, &binding.span);
            }
            ctx.define(&binding.name, SymbolKind::Immutable);
        }
        Statement::Mut(binding) => {
            analyze_expression(ctx, &binding.value);
            if let Some(ref type_ann) = binding.type_ann {
                validate_type(ctx, type_ann, &binding.span);
            }
            ctx.define(&binding.name, SymbolKind::Mutable);
        }
        Statement::Assignment(assign) => {
            // Check that target is a mutable variable (if it's a simple identifier)
            if let Expression::Identifier(ref name, ref span) = assign.target {
                match ctx.lookup(name) {
                    Some(SymbolKind::Immutable) => {
                        ctx.error_with_hint(
                            format!("cannot assign to immutable variable '{}'", name),
                            span.clone(),
                            "declare with 'mut' to make it mutable",
                        );
                    }
                    None => {
                        ctx.error(
                            format!("unknown variable '{}'", name),
                            span.clone(),
                        );
                    }
                    _ => {}
                }
            }
            analyze_expression(ctx, &assign.target);
            analyze_expression(ctx, &assign.value);
        }
        Statement::Expression(expr) => {
            analyze_expression(ctx, expr);
        }
        Statement::Return(ret) => {
            if let Some(ref value) = ret.value {
                analyze_expression(ctx, value);
            }
        }
        Statement::If(if_stmt) => {
            analyze_expression(ctx, &if_stmt.condition);
            ctx.push_scope();
            analyze_block(ctx, &if_stmt.then_block);
            ctx.pop_scope();
            for (cond, block) in &if_stmt.else_if_blocks {
                analyze_expression(ctx, cond);
                ctx.push_scope();
                analyze_block(ctx, block);
                ctx.pop_scope();
            }
            if let Some(ref else_block) = if_stmt.else_block {
                ctx.push_scope();
                analyze_block(ctx, else_block);
                ctx.pop_scope();
            }
        }
        Statement::ForLoop(for_loop) => {
            analyze_expression(ctx, &for_loop.iterable);
            ctx.push_scope();
            ctx.define(&for_loop.variable, SymbolKind::LoopVariable);
            analyze_block(ctx, &for_loop.body);
            ctx.pop_scope();
        }
        Statement::WhileLoop(while_loop) => {
            analyze_expression(ctx, &while_loop.condition);
            ctx.push_scope();
            analyze_block(ctx, &while_loop.body);
            ctx.pop_scope();
        }
        Statement::Break(_) | Statement::Continue(_) => {}
    }
}

fn analyze_expression(ctx: &mut SemanticContext, expr: &Expression) {
    match expr {
        Expression::Identifier(name, _span) => {
            if ctx.lookup(name).is_none() && !ctx.structs.contains(name) && !ctx.enums.contains(name) {
                // Don't report errors for things that might come from imported crates
                // We only warn for clearly local identifiers
            }
        }
        Expression::BinaryOp { left, right, .. } => {
            analyze_expression(ctx, left);
            analyze_expression(ctx, right);
        }
        Expression::UnaryOp { operand, .. } => {
            analyze_expression(ctx, operand);
        }
        Expression::FunctionCall { name, args, .. } => {
            analyze_expression(ctx, name);
            for arg in args {
                analyze_expression(ctx, arg);
            }
        }
        Expression::MethodCall { object, args, .. } => {
            analyze_expression(ctx, object);
            for arg in args {
                analyze_expression(ctx, arg);
            }
        }
        Expression::FieldAccess { object, .. } => {
            analyze_expression(ctx, object);
        }
        Expression::ArrayLiteral(elements, _) => {
            for elem in elements {
                analyze_expression(ctx, elem);
            }
        }
        Expression::IndexAccess { object, index, .. } => {
            analyze_expression(ctx, object);
            analyze_expression(ctx, index);
        }
        Expression::StructLiteral { name, fields, span: _ } => {
            if !ctx.structs.contains(name) {
                // Might be from an imported crate — don't error
            }
            for (_, value) in fields {
                analyze_expression(ctx, value);
            }
        }
        Expression::Closure { body, .. } => {
            analyze_expression(ctx, body);
        }
        Expression::Block(block) => {
            ctx.push_scope();
            analyze_block(ctx, block);
            ctx.pop_scope();
        }
        // Literals and paths need no further analysis
        _ => {}
    }
}

fn validate_type(ctx: &mut SemanticContext, type_ann: &TypeAnnotation, span: &Span) {
    match type_ann {
        TypeAnnotation::Simple(name) => {
            let valid_builtins = [
                "str", "String", "i8", "i16", "i32", "i64", "i128",
                "u8", "u16", "u32", "u64", "u128", "f32", "f64",
                "bool", "char", "usize", "isize",
            ];
            if !valid_builtins.contains(&name.as_str())
                && !ctx.structs.contains(name)
                && !ctx.enums.contains(name)
            {
                // Could be from an imported crate, so don't error.
                // The Rust compiler will catch truly unknown types.
            }
        }
        TypeAnnotation::Array(inner) => validate_type(ctx, inner, span),
        TypeAnnotation::Optional(inner) => validate_type(ctx, inner, span),
        TypeAnnotation::Tuple(types) => {
            for t in types {
                validate_type(ctx, t, span);
            }
        }
        TypeAnnotation::Reference(inner) | TypeAnnotation::MutReference(inner) => {
            validate_type(ctx, inner, span);
        }
        TypeAnnotation::Generic(_, params) => {
            for t in params {
                validate_type(ctx, t, span);
            }
        }
        TypeAnnotation::Unit => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn analyze_source(input: &str) -> Vec<PblError> {
        let mut lexer = Lexer::new(input, "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens, "test.pbl");
        let program = parser.parse().unwrap();
        analyze(&program)
    }

    #[test]
    fn test_valid_program() {
        let errors = analyze_source(r#"
            fn main() {
                let name = "Polar Bear"
                mut score = 0
                score = score + 1
                print(name)
            }
        "#);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_immutable_assignment() {
        let errors = analyze_source(r#"
            fn main() {
                let x = 5
                x = 10
            }
        "#);
        assert!(!errors.is_empty(), "Expected error for assigning to immutable variable");
    }

    #[test]
    fn test_duplicate_function() {
        let errors = analyze_source(r#"
            fn foo() { }
            fn foo() { }
        "#);
        assert!(!errors.is_empty(), "Expected error for duplicate function");
    }

    #[test]
    fn test_duplicate_struct_field() {
        let errors = analyze_source(r#"
            struct Foo { x: i32, x: i32 }
        "#);
        assert!(!errors.is_empty(), "Expected error for duplicate field");
    }

    #[test]
    fn test_duplicate_enum_variant() {
        let errors = analyze_source(r#"
            enum Foo { A, A }
        "#);
        assert!(!errors.is_empty(), "Expected error for duplicate variant");
    }
}

/// PBL Code Generator — Emits Rust source code from a PBL AST.

use crate::ast::*;
use crate::errors::PblResult;

/// Configuration for code generation.
pub struct CodeGenConfig {
    pub no_std: bool,
}

impl Default for CodeGenConfig {
    fn default() -> Self {
        Self { no_std: false }
    }
}

/// Generate Rust source code from a PBL program.
pub fn generate(program: &Program, config: &CodeGenConfig) -> PblResult<String> {
    let mut gen = RustGenerator::new(config);
    gen.generate_program(program)?;
    Ok(gen.output)
}

struct RustGenerator<'a> {
    output: String,
    indent: usize,
    config: &'a CodeGenConfig,
}

impl<'a> RustGenerator<'a> {
    fn new(config: &'a CodeGenConfig) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            config,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.write_indent();
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    fn generate_program(&mut self, program: &Program) -> PblResult<()> {
        // Emit preamble
        if self.config.no_std {
            self.writeln("#![no_std]");
            self.newline();
        }

        // Emit use declarations
        for item in &program.items {
            if let Item::UseDecl(u) = item {
                self.generate_use(u)?;
            }
        }

        // Emit rust blocks
        for item in &program.items {
            if let Item::RustBlock(r) = item {
                self.generate_rust_block(r)?;
            }
        }

        // Emit structs
        for item in &program.items {
            if let Item::Struct(s) = item {
                self.generate_struct(s)?;
                self.newline();
            }
        }

        // Emit enums
        for item in &program.items {
            if let Item::Enum(e) = item {
                self.generate_enum(e)?;
                self.newline();
            }
        }

        // Emit functions
        for item in &program.items {
            if let Item::Function(f) = item {
                self.generate_function(f)?;
                self.newline();
            }
        }

        // Emit top-level statements wrapped in main if no main function exists
        let has_main = program.items.iter().any(|item| {
            matches!(item, Item::Function(f) if f.name == "main")
        });

        let top_level_stmts: Vec<&Statement> = program
            .items
            .iter()
            .filter_map(|item| {
                if let Item::Statement(s) = item {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();

        if !top_level_stmts.is_empty() && !has_main {
            self.writeln("fn main() {");
            self.indent += 1;
            for stmt in &top_level_stmts {
                self.generate_statement(stmt)?;
            }
            self.indent -= 1;
            self.writeln("}");
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn block_uses_print(&self, block: &Block) -> bool {
        for stmt in &block.statements {
            if self.stmt_uses_print(stmt) {
                return true;
            }
        }
        false
    }

    fn stmt_uses_print(&self, stmt: &Statement) -> bool {
        match stmt {
            Statement::Expression(expr) => self.expr_uses_print(expr),
            Statement::If(if_stmt) => {
                self.block_uses_print(&if_stmt.then_block)
                    || if_stmt.else_if_blocks.iter().any(|(_, b)| self.block_uses_print(b))
                    || if_stmt.else_block.as_ref().map_or(false, |b| self.block_uses_print(b))
            }
            Statement::ForLoop(f) => self.block_uses_print(&f.body),
            Statement::WhileLoop(w) => self.block_uses_print(&w.body),
            _ => false,
        }
    }

    fn expr_uses_print(&self, expr: &Expression) -> bool {
        match expr {
            Expression::FunctionCall { name, .. } => {
                if let Expression::Identifier(ref n, _) = **name {
                    n == "print" || n == "println"
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    // -----------------------------------------------------------------------
    // Functions
    // -----------------------------------------------------------------------

    fn generate_function(&mut self, func: &FunctionDef) -> PblResult<()> {
        self.write_indent();
        if func.is_pub {
            self.write("pub ");
        }
        self.write(&format!("fn {}(", func.name));

        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.write(&format!("{}: {}", param.name, self.type_to_rust(&param.type_ann)));
        }

        self.write(")");

        if let Some(ref ret_type) = func.return_type {
            self.write(&format!(" -> {}", self.type_to_rust(ret_type)));
        }

        self.write(" {\n");
        self.indent += 1;

        for stmt in &func.body.statements {
            self.generate_statement(stmt)?;
        }

        self.indent -= 1;
        self.writeln("}");

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Statements
    // -----------------------------------------------------------------------

    fn generate_statement(&mut self, stmt: &Statement) -> PblResult<()> {
        match stmt {
            Statement::Let(binding) => {
                self.write_indent();
                self.write(&format!("let {}", binding.name));
                if let Some(ref type_ann) = binding.type_ann {
                    self.write(&format!(": {}", self.type_to_rust(type_ann)));
                }
                self.write(" = ");
                self.generate_expression(&binding.value)?;
                self.write(";\n");
            }
            Statement::Mut(binding) => {
                self.write_indent();
                self.write(&format!("let mut {}", binding.name));
                if let Some(ref type_ann) = binding.type_ann {
                    self.write(&format!(": {}", self.type_to_rust(type_ann)));
                }
                self.write(" = ");
                self.generate_expression(&binding.value)?;
                self.write(";\n");
            }
            Statement::Assignment(assign) => {
                self.write_indent();
                self.generate_expression(&assign.target)?;
                let op_str = match assign.op {
                    AssignOp::Assign => " = ",
                    AssignOp::AddAssign => " += ",
                    AssignOp::SubAssign => " -= ",
                    AssignOp::MulAssign => " *= ",
                    AssignOp::DivAssign => " /= ",
                };
                self.write(op_str);
                self.generate_expression_unwrapped(&assign.value)?;
                self.write(";\n");
            }
            Statement::Expression(expr) => {
                self.write_indent();
                self.generate_expression(expr)?;
                self.write(";\n");
            }
            Statement::Return(ret) => {
                self.write_indent();
                if let Some(ref value) = ret.value {
                    self.write("return ");
                    self.generate_expression_unwrapped(value)?;
                    self.write(";\n");
                } else {
                    self.write("return;\n");
                }
            }
            Statement::If(if_stmt) => {
                self.write_indent();
                self.write("if ");
                self.generate_expression_unwrapped(&if_stmt.condition)?;
                self.write(" {\n");
                self.indent += 1;
                for s in &if_stmt.then_block.statements {
                    self.generate_statement(s)?;
                }
                self.indent -= 1;
                self.write_indent();
                self.write("}");

                for (cond, block) in &if_stmt.else_if_blocks {
                    self.write(" else if ");
                    self.generate_expression_unwrapped(cond)?;
                    self.write(" {\n");
                    self.indent += 1;
                    for s in &block.statements {
                        self.generate_statement(s)?;
                    }
                    self.indent -= 1;
                    self.write_indent();
                    self.write("}");
                }

                if let Some(ref else_block) = if_stmt.else_block {
                    self.write(" else {\n");
                    self.indent += 1;
                    for s in &else_block.statements {
                        self.generate_statement(s)?;
                    }
                    self.indent -= 1;
                    self.write_indent();
                    self.write("}");
                }
                self.write("\n");
            }
            Statement::ForLoop(for_loop) => {
                self.write_indent();
                self.write(&format!("for {} in ", for_loop.variable));
                self.generate_expression_unwrapped(&for_loop.iterable)?;
                self.write(" {\n");
                self.indent += 1;
                for s in &for_loop.body.statements {
                    self.generate_statement(s)?;
                }
                self.indent -= 1;
                self.writeln("}");
            }
            Statement::WhileLoop(while_loop) => {
                self.write_indent();
                self.write("while ");
                self.generate_expression_unwrapped(&while_loop.condition)?;
                self.write(" {\n");
                self.indent += 1;
                for s in &while_loop.body.statements {
                    self.generate_statement(s)?;
                }
                self.indent -= 1;
                self.writeln("}");
            }
            Statement::Break(_) => {
                self.writeln("break;");
            }
            Statement::Continue(_) => {
                self.writeln("continue;");
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Expressions
    // -----------------------------------------------------------------------

    fn generate_expression(&mut self, expr: &Expression) -> PblResult<()> {
        match expr {
            Expression::IntLiteral(n, _) => self.write(&n.to_string()),
            Expression::FloatLiteral(n, _) => {
                let s = n.to_string();
                if s.contains('.') {
                    self.write(&s);
                } else {
                    self.write(&format!("{}.0", s));
                }
            }
            Expression::StringLiteral(s, _) => {
                self.write(&format!("\"{}\".to_string()", Self::escape_string(s)));
            }
            Expression::BoolLiteral(b, _) => self.write(&b.to_string()),
            Expression::CharLiteral(c, _) => {
                self.write(&format!("'{}'", Self::escape_char(*c)));
            }
            Expression::Identifier(name, _) => self.write(name),

            Expression::BinaryOp { left, op, right, .. } => {
                // Special case: string concatenation with +
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%",
                    BinOp::Eq => "==",
                    BinOp::NotEq => "!=",
                    BinOp::Lt => "<",
                    BinOp::LtEq => "<=",
                    BinOp::Gt => ">",
                    BinOp::GtEq => ">=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                };

                // Check if this is string concatenation
                if *op == BinOp::Add && self.is_string_expr(left) {
                    self.write("format!(\"{}{}\", ");
                    self.generate_expression(left)?;
                    self.write(", ");
                    self.generate_expression(right)?;
                    self.write(")");
                } else {
                    self.write("(");
                    self.generate_expression(left)?;
                    self.write(&format!(" {} ", op_str));
                    self.generate_expression(right)?;
                    self.write(")");
                }
            }

            Expression::UnaryOp { op, operand, .. } => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                };
                self.write(op_str);
                self.generate_expression(operand)?;
            }

            Expression::FunctionCall { name, args, .. } => {
                // Handle built-in print/println
                if let Expression::Identifier(ref n, _) = **name {
                    match n.as_str() {
                        "print" => {
                            if args.len() == 1 {
                                if let Expression::StringLiteral(ref s, _) = args[0] {
                                    self.write(&format!("print!(\"{}\", )", Self::escape_string(s)));
                                } else {
                                    self.write("print!(\"{}\", ");
                                    self.generate_expression(&args[0])?;
                                    self.write(")");
                                }
                            } else {
                                self.write("print!(");
                                for (i, arg) in args.iter().enumerate() {
                                    if i > 0 {
                                        self.write(", ");
                                    }
                                    self.generate_expression(arg)?;
                                }
                                self.write(")");
                            }
                            return Ok(());
                        }
                        "println" => {
                            if args.is_empty() {
                                self.write("println!()");
                            } else if args.len() == 1 {
                                if let Expression::StringLiteral(ref s, _) = args[0] {
                                    self.write(&format!("println!(\"{}\")", Self::escape_string(s)));
                                } else {
                                    self.write("println!(\"{}\", ");
                                    self.generate_expression(&args[0])?;
                                    self.write(")");
                                }
                            } else {
                                self.write("println!(");
                                for (i, arg) in args.iter().enumerate() {
                                    if i > 0 {
                                        self.write(", ");
                                    }
                                    self.generate_expression(arg)?;
                                }
                                self.write(")");
                            }
                            return Ok(());
                        }
                        "format" => {
                            self.write("format!(");
                            for (i, arg) in args.iter().enumerate() {
                                if i > 0 {
                                    self.write(", ");
                                }
                                self.generate_expression(arg)?;
                            }
                            self.write(")");
                            return Ok(());
                        }
                        _ => {}
                    }
                }

                self.generate_expression(name)?;
                self.write("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.generate_expression(arg)?;
                }
                self.write(")");
            }

            Expression::MethodCall { object, method, args, .. } => {
                self.generate_expression(object)?;
                self.write(&format!(".{}(", method));
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.generate_expression(arg)?;
                }
                self.write(")");
            }

            Expression::FieldAccess { object, field, .. } => {
                self.generate_expression(object)?;
                self.write(&format!(".{}", field));
            }

            Expression::ArrayLiteral(elements, _) => {
                self.write("vec![");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.generate_expression(elem)?;
                }
                self.write("]");
            }

            Expression::IndexAccess { object, index, .. } => {
                self.generate_expression(object)?;
                self.write("[");
                self.generate_expression(index)?;
                self.write("]");
            }

            Expression::StructLiteral { name, fields, .. } => {
                self.write(&format!("{} {{ ", name));
                for (i, (field_name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&format!("{}: ", field_name));
                    self.generate_expression(value)?;
                }
                self.write(" }");
            }

            Expression::PathAccess { segments, .. } => {
                self.write(&segments.join("::"));
            }

            Expression::Closure { params, body, .. } => {
                self.write("|");
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&param.name);
                    if let Some(ref type_ann) = param.type_ann {
                        self.write(&format!(": {}", self.type_to_rust(type_ann)));
                    }
                }
                self.write("| ");
                self.generate_expression(body)?;
            }

            Expression::Block(block) => {
                self.write("{\n");
                self.indent += 1;
                for s in &block.statements {
                    self.generate_statement(s)?;
                }
                self.indent -= 1;
                self.write_indent();
                self.write("}");
            }

            Expression::RustInline(code, _) => {
                self.write(code);
            }
        }

        Ok(())
    }

    fn generate_expression_unwrapped(&mut self, expr: &Expression) -> PblResult<()> {
        match expr {
            Expression::BinaryOp { left, op, right, .. } => {
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%",
                    BinOp::Eq => "==",
                    BinOp::NotEq => "!=",
                    BinOp::Lt => "<",
                    BinOp::LtEq => "<=",
                    BinOp::Gt => ">",
                    BinOp::GtEq => ">=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                };

                if *op == BinOp::Add && self.is_string_expr(left) {
                    self.write("format!(\"{}{}\", ");
                    self.generate_expression(left)?;
                    self.write(", ");
                    self.generate_expression(right)?;
                    self.write(")");
                } else {
                    self.generate_expression(left)?;
                    self.write(&format!(" {} ", op_str));
                    self.generate_expression(right)?;
                }
                Ok(())
            }
            _ => self.generate_expression(expr),
        }
    }

    // -----------------------------------------------------------------------
    // Types
    // -----------------------------------------------------------------------

    fn type_to_rust(&self, type_ann: &TypeAnnotation) -> String {
        match type_ann {
            TypeAnnotation::Simple(name) => match name.as_str() {
                "str" => "String".to_string(),
                other => other.to_string(),
            },
            TypeAnnotation::Array(inner) => format!("Vec<{}>", self.type_to_rust(inner)),
            TypeAnnotation::Optional(inner) => format!("Option<{}>", self.type_to_rust(inner)),
            TypeAnnotation::Tuple(types) => {
                let inner: Vec<String> = types.iter().map(|t| self.type_to_rust(t)).collect();
                format!("({})", inner.join(", "))
            }
            TypeAnnotation::Reference(inner) => format!("&{}", self.type_to_rust(inner)),
            TypeAnnotation::MutReference(inner) => format!("&mut {}", self.type_to_rust(inner)),
            TypeAnnotation::Generic(name, params) => {
                let inner: Vec<String> = params.iter().map(|t| self.type_to_rust(t)).collect();
                format!("{}<{}>", name, inner.join(", "))
            }
            TypeAnnotation::Unit => "()".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Structs & Enums
    // -----------------------------------------------------------------------

    fn generate_struct(&mut self, s: &StructDef) -> PblResult<()> {
        self.write_indent();
        if s.is_pub {
            self.write("pub ");
        }
        self.write(&format!("struct {} {{\n", s.name));
        self.indent += 1;

        for field in &s.fields {
            self.write_indent();
            if field.is_pub {
                self.write("pub ");
            }
            self.write(&format!("{}: {},\n", field.name, self.type_to_rust(&field.type_ann)));
        }

        self.indent -= 1;
        self.writeln("}");
        Ok(())
    }

    fn generate_enum(&mut self, e: &EnumDef) -> PblResult<()> {
        self.write_indent();
        if e.is_pub {
            self.write("pub ");
        }
        self.write(&format!("enum {} {{\n", e.name));
        self.indent += 1;

        for variant in &e.variants {
            self.write_indent();
            self.write(&variant.name);
            if let Some(ref fields) = variant.fields {
                let types: Vec<String> = fields.iter().map(|t| self.type_to_rust(t)).collect();
                self.write(&format!("({})", types.join(", ")));
            }
            self.write(",\n");
        }

        self.indent -= 1;
        self.writeln("}");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Use declarations & Rust blocks
    // -----------------------------------------------------------------------

    fn generate_use(&mut self, u: &UseDecl) -> PblResult<()> {
        self.write_indent();
        self.write(&format!("use {};\n", u.path.join("::")));
        Ok(())
    }

    fn generate_rust_block(&mut self, r: &RustBlock) -> PblResult<()> {
        self.writeln(&r.code);
        self.newline();
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------------

    fn is_string_expr(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::StringLiteral(_, _))
    }

    fn escape_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\r', "\\r")
    }

    fn escape_char(c: char) -> String {
        match c {
            '\n' => "\\n".to_string(),
            '\t' => "\\t".to_string(),
            '\r' => "\\r".to_string(),
            '\\' => "\\\\".to_string(),
            '\'' => "\\'".to_string(),
            '\0' => "\\0".to_string(),
            other => other.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn gen(input: &str) -> String {
        let mut lexer = Lexer::new(input, "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens, "test.pbl");
        let program = parser.parse().unwrap();
        generate(&program, &CodeGenConfig::default()).unwrap()
    }

    #[test]
    fn test_hello_world() {
        let rust = gen(r#"fn main() { println("Hello, Polar Bear!") }"#);
        assert!(rust.contains("fn main()"));
        assert!(rust.contains("println!"));
        assert!(rust.contains("Hello, Polar Bear!"));
    }

    #[test]
    fn test_variables() {
        let rust = gen(r#"
            fn main() {
                let name: str = "Polar Bear"
                mut score: i32 = 0
                score = score + 1
            }
        "#);
        assert!(rust.contains("let name: String"));
        assert!(rust.contains("let mut score: i32"));
        assert!(rust.contains("score = score + 1"));
    }

    #[test]
    fn test_function_with_return() {
        let rust = gen("fn add(a: i32, b: i32) -> i32 { return a + b }");
        assert!(rust.contains("fn add(a: i32, b: i32) -> i32"));
        assert!(rust.contains("return a + b"));
    }

    #[test]
    fn test_struct() {
        let rust = gen("struct PolarBear { name: str, age: i32 }");
        assert!(rust.contains("struct PolarBear"));
        assert!(rust.contains("name: String"));
        assert!(rust.contains("age: i32"));
    }

    #[test]
    fn test_enum() {
        let rust = gen("enum Animal { PolarBear, Penguin, Fox }");
        assert!(rust.contains("enum Animal"));
        assert!(rust.contains("PolarBear,"));
        assert!(rust.contains("Penguin,"));
        assert!(rust.contains("Fox,"));
    }

    #[test]
    fn test_no_std() {
        let mut lexer = Lexer::new("fn start() { }", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens, "test.pbl");
        let program = parser.parse().unwrap();
        let config = CodeGenConfig { no_std: true };
        let rust = generate(&program, &config).unwrap();
        assert!(rust.contains("#![no_std]"));
    }

    #[test]
    fn test_if_else() {
        let rust = gen(r#"
            fn main() {
                let x = 10
                if x > 5 {
                    println("big")
                } else {
                    println("small")
                }
            }
        "#);
        assert!(rust.contains("if x > 5"));
        assert!(rust.contains("} else {"));
    }

    #[test]
    fn test_for_loop() {
        let rust = gen(r#"
            fn main() {
                for item in items {
                    println(item)
                }
            }
        "#);
        assert!(rust.contains("for item in items"));
    }
}

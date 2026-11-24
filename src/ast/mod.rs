/// PBL AST — Abstract Syntax Tree types for the Polar Bear Language.
///
/// Every PBL source construct is represented here. The parser builds this tree,
/// semantic analysis validates it, and the code generator walks it to emit Rust.

/// Source location attached to every node for error reporting.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(file: &str, line: usize, column: usize) -> Self {
        Self {
            file: file.to_string(),
            line,
            column,
        }
    }

    pub fn unknown() -> Self {
        Self {
            file: "<unknown>".to_string(),
            line: 0,
            column: 0,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

// ---------------------------------------------------------------------------
// Top-level program
// ---------------------------------------------------------------------------

/// A complete PBL source file.
#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

/// Top-level items that can appear in a PBL file.
#[derive(Debug, Clone)]
pub enum Item {
    Function(FunctionDef),
    Struct(StructDef),
    Enum(EnumDef),
    CrateDecl(CrateDecl),
    UseDecl(UseDecl),
    RustBlock(RustBlock),
    ProjectConfig(ProjectConfig),
    Statement(Statement),
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    Simple(String),                           // str, i32, bool, etc.
    Array(Box<TypeAnnotation>),               // [i32]
    Optional(Box<TypeAnnotation>),            // ?str
    Tuple(Vec<TypeAnnotation>),               // (i32, str)
    Reference(Box<TypeAnnotation>),           // &str
    MutReference(Box<TypeAnnotation>),        // &mut str
    Generic(String, Vec<TypeAnnotation>),     // Vec<i32>, HashMap<str, i32>
    Unit,                                     // ()
}

impl std::fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeAnnotation::Simple(name) => write!(f, "{}", name),
            TypeAnnotation::Array(inner) => write!(f, "[{}]", inner),
            TypeAnnotation::Optional(inner) => write!(f, "?{}", inner),
            TypeAnnotation::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            TypeAnnotation::Reference(inner) => write!(f, "&{}", inner),
            TypeAnnotation::MutReference(inner) => write!(f, "&mut {}", inner),
            TypeAnnotation::Generic(name, params) => {
                write!(f, "{}<", name)?;
                for (i, t) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ">")
            }
            TypeAnnotation::Unit => write!(f, "()"),
        }
    }
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetBinding),
    Mut(MutBinding),
    Assignment(Assignment),
    Expression(Expression),
    Return(ReturnStmt),
    If(IfStmt),
    ForLoop(ForLoop),
    WhileLoop(WhileLoop),
    Break(Span),
    Continue(Span),
}

#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MutBinding {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub target: Expression,
    pub op: AssignOp,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignOp {
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expression,
    pub then_block: Block,
    pub else_if_blocks: Vec<(Expression, Block)>,
    pub else_block: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForLoop {
    pub variable: String,
    pub iterable: Expression,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileLoop {
    pub condition: Expression,
    pub body: Block,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Expression {
    IntLiteral(i64, Span),
    FloatLiteral(f64, Span),
    StringLiteral(String, Span),
    BoolLiteral(bool, Span),
    CharLiteral(char, Span),
    Identifier(String, Span),
    BinaryOp {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expression>,
        span: Span,
    },
    FunctionCall {
        name: Box<Expression>,
        args: Vec<Expression>,
        span: Span,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
        span: Span,
    },
    FieldAccess {
        object: Box<Expression>,
        field: String,
        span: Span,
    },
    ArrayLiteral(Vec<Expression>, Span),
    IndexAccess {
        object: Box<Expression>,
        index: Box<Expression>,
        span: Span,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
        span: Span,
    },
    PathAccess {
        segments: Vec<String>,
        span: Span,
    },
    Closure {
        params: Vec<ClosureParam>,
        body: Box<Expression>,
        span: Span,
    },
    Block(Block),
    RustInline(String, Span),
}

impl Expression {
    pub fn span(&self) -> &Span {
        match self {
            Expression::IntLiteral(_, s)
            | Expression::FloatLiteral(_, s)
            | Expression::StringLiteral(_, s)
            | Expression::BoolLiteral(_, s)
            | Expression::CharLiteral(_, s)
            | Expression::Identifier(_, s)
            | Expression::ArrayLiteral(_, s)
            | Expression::RustInline(_, s) => s,
            Expression::BinaryOp { span, .. }
            | Expression::UnaryOp { span, .. }
            | Expression::FunctionCall { span, .. }
            | Expression::MethodCall { span, .. }
            | Expression::FieldAccess { span, .. }
            | Expression::IndexAccess { span, .. }
            | Expression::StructLiteral { span, .. }
            | Expression::PathAccess { span, .. }
            | Expression::Closure { span, .. } => span,
            Expression::Block(block) => &block.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClosureParam {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}

impl BinOp {
    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::Eq | BinOp::NotEq => 3,
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => 4,
            BinOp::Add | BinOp::Sub => 5,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,
    Not,
}

// ---------------------------------------------------------------------------
// Structs & Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Option<Vec<TypeAnnotation>>, // None = unit, Some = tuple variant
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Crate declarations & imports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CrateDecl {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub source: CrateSource,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CrateSource {
    CratesIo,
    Path(String),
    Git(String),
}

#[derive(Debug, Clone)]
pub struct UseDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Rust interop
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RustBlock {
    pub code: String,
    pub span: Span,
}

// ---------------------------------------------------------------------------
// Project configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub settings: Vec<ProjectSetting>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ProjectSetting {
    Mode(String),    // e.g., "no_std"
    Entry(String),   // custom entry point
}

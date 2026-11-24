/// PBL Error types — unified error handling for the compiler.

use crate::ast::Span;
use std::fmt;

#[derive(Debug, Clone)]
pub struct PblError {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    Lexer,
    Parser,
    Semantic,
    CodeGen,
    Project,
    Io,
}

impl PblError {
    pub fn new(kind: ErrorKind, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn lexer(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Lexer, message, Some(span))
    }

    pub fn parser(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Parser, message, Some(span))
    }

    pub fn semantic(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Semantic, message, Some(span))
    }

    pub fn codegen(message: impl Into<String>, span: Option<Span>) -> Self {
        Self::new(ErrorKind::CodeGen, message, span)
    }

    pub fn project(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Project, message, None)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Io, message, None)
    }
}

impl fmt::Display for PblError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.kind {
            ErrorKind::Lexer => "lexer error",
            ErrorKind::Parser => "parse error",
            ErrorKind::Semantic => "semantic error",
            ErrorKind::CodeGen => "codegen error",
            ErrorKind::Project => "project error",
            ErrorKind::Io => "i/o error",
        };

        if let Some(ref span) = self.span {
            write!(f, "{}: {}: {}", span, prefix, self.message)?;
        } else {
            write!(f, "{}: {}", prefix, self.message)?;
        }

        if let Some(ref hint) = self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }

        Ok(())
    }
}

impl std::error::Error for PblError {}

pub type PblResult<T> = Result<T, PblError>;

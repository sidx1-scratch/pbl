/// PBL Lexer — Tokenizer implementation.

use crate::ast::Span;
use crate::errors::{PblError, PblResult};
use super::{Token, TokenKind};

pub struct Lexer {
    source: Vec<char>,
    file: String,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str, file: &str) -> Self {
        Self {
            source: source.chars().collect(),
            file: file.to_string(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> PblResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            if self.is_at_end() {
                tokens.push(Token::new(
                    TokenKind::Eof,
                    self.current_span(),
                ));
                break;
            }

            let token = self.next_token()?;
            tokens.push(token);
        }

        Ok(tokens)
    }

    fn current_span(&self) -> Span {
        Span::new(&self.file, self.line, self.column)
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    self.advance();
                }
                Some('/') if self.peek_next() == Some('/') => {
                    // Line comment
                    while !self.is_at_end() && self.peek() != Some('\n') {
                        self.advance();
                    }
                }
                Some('/') if self.peek_next() == Some('*') => {
                    // Block comment
                    self.advance(); // /
                    self.advance(); // *
                    let mut depth = 1;
                    while !self.is_at_end() && depth > 0 {
                        if self.peek() == Some('/') && self.peek_next() == Some('*') {
                            self.advance();
                            self.advance();
                            depth += 1;
                        } else if self.peek() == Some('*') && self.peek_next() == Some('/') {
                            self.advance();
                            self.advance();
                            depth -= 1;
                        } else {
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> PblResult<Token> {
        let span = self.current_span();
        let ch = self.advance();

        match ch {
            // Single-character tokens
            '(' => Ok(Token::new(TokenKind::LParen, span)),
            ')' => Ok(Token::new(TokenKind::RParen, span)),
            '{' => Ok(Token::new(TokenKind::LBrace, span)),
            '}' => Ok(Token::new(TokenKind::RBrace, span)),
            '[' => Ok(Token::new(TokenKind::LBracket, span)),
            ']' => Ok(Token::new(TokenKind::RBracket, span)),
            ',' => Ok(Token::new(TokenKind::Comma, span)),
            ';' => Ok(Token::new(TokenKind::Semicolon, span)),
            '%' => Ok(Token::new(TokenKind::Percent, span)),
            '#' => Ok(Token::new(TokenKind::Hash, span)),
            '?' => Ok(Token::new(TokenKind::Question, span)),

            // Dot
            '.' => Ok(Token::new(TokenKind::Dot, span)),

            // Colon or ColonColon
            ':' => {
                if self.peek() == Some(':') {
                    self.advance();
                    Ok(Token::new(TokenKind::ColonColon, span))
                } else {
                    Ok(Token::new(TokenKind::Colon, span))
                }
            }

            // Operators that might be two characters
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::PlusAssign, span))
                } else {
                    Ok(Token::new(TokenKind::Plus, span))
                }
            }
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Ok(Token::new(TokenKind::Arrow, span))
                } else if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::MinusAssign, span))
                } else {
                    Ok(Token::new(TokenKind::Minus, span))
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::StarAssign, span))
                } else {
                    Ok(Token::new(TokenKind::Star, span))
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::SlashAssign, span))
                } else {
                    Ok(Token::new(TokenKind::Slash, span))
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::Eq, span))
                } else {
                    Ok(Token::new(TokenKind::Assign, span))
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::NotEq, span))
                } else {
                    Ok(Token::new(TokenKind::Not, span))
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::LtEq, span))
                } else {
                    Ok(Token::new(TokenKind::Lt, span))
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(Token::new(TokenKind::GtEq, span))
                } else {
                    Ok(Token::new(TokenKind::Gt, span))
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    Ok(Token::new(TokenKind::And, span))
                } else {
                    Ok(Token::new(TokenKind::Ampersand, span))
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    Ok(Token::new(TokenKind::Or, span))
                } else {
                    Ok(Token::new(TokenKind::Pipe, span))
                }
            }

            // String literal
            '"' => self.lex_string(span),

            // Char literal
            '\'' => self.lex_char(span),

            // Number
            c if c.is_ascii_digit() => self.lex_number(c, span),

            // Identifier or keyword
            c if c.is_ascii_alphabetic() || c == '_' => self.lex_identifier(c, span),

            other => Err(PblError::lexer(
                format!("unexpected character '{}'", other),
                span,
            )),
        }
    }

    fn lex_string(&mut self, span: Span) -> PblResult<Token> {
        let mut s = String::new();
        loop {
            if self.is_at_end() {
                return Err(PblError::lexer("unterminated string literal", span));
            }
            let ch = self.advance();
            match ch {
                '"' => break,
                '\\' => {
                    if self.is_at_end() {
                        return Err(PblError::lexer("unterminated escape sequence", span));
                    }
                    let escaped = self.advance();
                    match escaped {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        '0' => s.push('\0'),
                        other => {
                            return Err(PblError::lexer(
                                format!("unknown escape sequence '\\{}'", other),
                                self.current_span(),
                            ));
                        }
                    }
                }
                _ => s.push(ch),
            }
        }
        Ok(Token::new(TokenKind::StringLiteral(s), span))
    }

    fn lex_char(&mut self, span: Span) -> PblResult<Token> {
        if self.is_at_end() {
            return Err(PblError::lexer("unterminated character literal", span));
        }
        let ch = if self.peek() == Some('\\') {
            self.advance(); // consume backslash
            if self.is_at_end() {
                return Err(PblError::lexer("unterminated escape in character literal", span));
            }
            let escaped = self.advance();
            match escaped {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '0' => '\0',
                other => {
                    return Err(PblError::lexer(
                        format!("unknown escape sequence '\\{}'", other),
                        span,
                    ));
                }
            }
        } else {
            self.advance()
        };

        if self.peek() != Some('\'') {
            return Err(PblError::lexer(
                "expected closing quote for character literal",
                self.current_span(),
            ));
        }
        self.advance(); // consume closing quote
        Ok(Token::new(TokenKind::CharLiteral(ch), span))
    }

    fn lex_number(&mut self, first: char, span: Span) -> PblResult<Token> {
        let mut num_str = String::new();
        num_str.push(first);
        let mut is_float = false;

        while !self.is_at_end() {
            match self.peek() {
                Some(c) if c.is_ascii_digit() => {
                    num_str.push(self.advance());
                }
                Some('_') if self.peek_next().map_or(false, |c| c.is_ascii_digit()) => {
                    self.advance(); // skip underscore separator
                }
                Some('.') if !is_float && self.peek_next().map_or(false, |c| c.is_ascii_digit()) => {
                    is_float = true;
                    num_str.push(self.advance()); // consume '.'
                }
                _ => break,
            }
        }

        if is_float {
            let val: f64 = num_str.parse().map_err(|_| {
                PblError::lexer(format!("invalid float literal '{}'", num_str), span.clone())
            })?;
            Ok(Token::new(TokenKind::FloatLiteral(val), span))
        } else {
            let val: i64 = num_str.parse().map_err(|_| {
                PblError::lexer(format!("invalid integer literal '{}'", num_str), span.clone())
            })?;
            Ok(Token::new(TokenKind::IntLiteral(val), span))
        }
    }

    fn lex_identifier(&mut self, first: char, span: Span) -> PblResult<Token> {
        let mut ident = String::new();
        ident.push(first);

        while !self.is_at_end() {
            match self.peek() {
                Some(c) if c.is_ascii_alphanumeric() || c == '_' => {
                    ident.push(self.advance());
                }
                _ => break,
            }
        }

        let kind = match ident.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "pub" => TokenKind::Pub,
            "crate" => TokenKind::Crate,
            "use" => TokenKind::Use,
            "rust" => TokenKind::Rust,
            "rust_inline" => TokenKind::RustInline,
            "project" => TokenKind::Project,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "as" => TokenKind::As,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "features" => TokenKind::Identifier(ident),
            _ => TokenKind::Identifier(ident),
        };

        Ok(Token::new(kind, span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("fn main() { }", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("main".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::LParen);
        assert_eq!(tokens[3].kind, TokenKind::RParen);
        assert_eq!(tokens[4].kind, TokenKind::LBrace);
        assert_eq!(tokens[5].kind, TokenKind::RBrace);
        assert_eq!(tokens[6].kind, TokenKind::Eof);
    }

    #[test]
    fn test_string_literal() {
        let mut lexer = Lexer::new(r#""hello world""#, "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral("hello world".to_string()));
    }

    #[test]
    fn test_escape_sequences() {
        let mut lexer = Lexer::new(r#""hello\nworld""#, "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::StringLiteral("hello\nworld".to_string()));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 1_000", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
        assert_eq!(tokens[1].kind, TokenKind::FloatLiteral(3.14));
        assert_eq!(tokens[2].kind, TokenKind::IntLiteral(1000));
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / == != <= >= && || -> += -=", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
        assert_eq!(tokens[4].kind, TokenKind::Eq);
        assert_eq!(tokens[5].kind, TokenKind::NotEq);
        assert_eq!(tokens[6].kind, TokenKind::LtEq);
        assert_eq!(tokens[7].kind, TokenKind::GtEq);
        assert_eq!(tokens[8].kind, TokenKind::And);
        assert_eq!(tokens[9].kind, TokenKind::Or);
        assert_eq!(tokens[10].kind, TokenKind::Arrow);
        assert_eq!(tokens[11].kind, TokenKind::PlusAssign);
        assert_eq!(tokens[12].kind, TokenKind::MinusAssign);
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("// comment\nfn /* block */ main", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("main".to_string()));
    }

    #[test]
    fn test_char_literal() {
        let mut lexer = Lexer::new("'a' '\\n'", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::CharLiteral('a'));
        assert_eq!(tokens[1].kind, TokenKind::CharLiteral('\n'));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("let mut if else for while in return struct enum pub crate use fn", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Mut);
        assert_eq!(tokens[2].kind, TokenKind::If);
        assert_eq!(tokens[3].kind, TokenKind::Else);
        assert_eq!(tokens[4].kind, TokenKind::For);
        assert_eq!(tokens[5].kind, TokenKind::While);
        assert_eq!(tokens[6].kind, TokenKind::In);
        assert_eq!(tokens[7].kind, TokenKind::Return);
        assert_eq!(tokens[8].kind, TokenKind::Struct);
        assert_eq!(tokens[9].kind, TokenKind::Enum);
        assert_eq!(tokens[10].kind, TokenKind::Pub);
        assert_eq!(tokens[11].kind, TokenKind::Crate);
        assert_eq!(tokens[12].kind, TokenKind::Use);
        assert_eq!(tokens[13].kind, TokenKind::Fn);
    }

    #[test]
    fn test_span_tracking() {
        let mut lexer = Lexer::new("fn\nmain", "test.pbl");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.column, 1);
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[1].span.column, 1);
    }

    #[test]
    fn test_unterminated_string() {
        let mut lexer = Lexer::new(r#""hello"#, "test.pbl");
        assert!(lexer.tokenize().is_err());
    }
}

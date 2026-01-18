//! GS2 Lexer using Logos
//!
//! Tokenizes GS2 source code into a stream of tokens for parsing.

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
    // Whitespace (skipped)
    #[regex(r"[ \t\r\n]+", logos::skip)]
    #[regex(r"//.*", logos::skip)]
    Whitespace,

    // Punctuation
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token("?")]
    Question,
    #[token(";")]
    Semicolon,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("!")]
    Bang,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
    #[token("=")]
    Equals,

    // Multi-character operators
    #[token("==")]
    EqualsEquals,
    #[token("!=")]
    BangEquals,
    #[token("<=")]
    LessEquals,
    #[token(">=")]
    GreaterEquals,
    #[token("&&")]
    AmpAmp,
    #[token("||")]
    PipePipe,
    #[token("<<")]
    LShift,
    #[token(">>")]
    RShift,
    #[token("++")]
    PlusPlus,
    #[token("--")]
    MinusMinus,

    // Assignment operators
    #[token("+=")]
    PlusEquals,
    #[token("-=")]
    MinusEquals,
    #[token("*=")]
    StarEquals,
    #[token("/=")]
    SlashEquals,
    #[token("%=")]
    PercentEquals,
    #[token("^=")]
    CaretEquals,
    #[token("&=")]
    AmpEquals,
    #[token("|=")]
    PipeEquals,
    #[token("<<=")]
    LShiftEquals,
    #[token(">>=")]
    RShiftEquals,

    // Keywords
    #[token("true")]
    KeywordTrue,
    #[token("false")]
    KeywordFalse,
    #[token("null")]
    KeywordNull,
    #[token("if")]
    KeywordIf,
    #[token("else")]
    KeywordElse,
    #[token("for")]
    KeywordFor,
    #[token("while")]
    KeywordWhile,
    #[token("break")]
    KeywordBreak,
    #[token("continue")]
    KeywordContinue,
    #[token("return")]
    KeywordReturn,
    #[token("function")]
    KeywordFunction,
    #[token("public")]
    KeywordPublic,
    #[token("with")]
    KeywordWith,
    #[token("switch")]
    KeywordSwitch,
    #[token("case")]
    KeywordCase,
    #[token("default")]
    KeywordDefault,
    #[token("new")]
    KeywordNew,
    #[token("in")]
    KeywordIn,
    #[token("elseif")]
    KeywordElseIf,
    #[token("const")]
    KeywordConst,
    #[token("enum")]
    KeywordEnum,

    // Literals
    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().to_string())]
    Number(String),

    #[regex(r#""[^"]*""#, |lex| lex.slice()[1..lex.slice().len()-1].to_string())]
    String(String),

    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
}

/// A simple lexer that produces tokens
pub struct Lexer<'a> {
    lexer: logos::Lexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            lexer: Token::lexer(source),
        }
    }

    pub fn next(&mut self) -> Option<Token> {
        loop {
            match self.lexer.next()? {
                Ok(Token::Whitespace) => continue,
                Ok(token) => return Some(token),
                Err(_) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let source = "function test() { return 42; }";
        let mut lexer = Lexer::new(source);

        assert_eq!(lexer.next(), Some(Token::KeywordFunction));
        assert_eq!(lexer.next(), Some(Token::Identifier("test".to_string())));
        assert_eq!(lexer.next(), Some(Token::LParen));
        assert_eq!(lexer.next(), Some(Token::RParen));
        assert_eq!(lexer.next(), Some(Token::LBrace));
        assert_eq!(lexer.next(), Some(Token::KeywordReturn));
        assert_eq!(lexer.next(), Some(Token::Number("42".to_string())));
        assert_eq!(lexer.next(), Some(Token::Semicolon));
        assert_eq!(lexer.next(), Some(Token::RBrace));
    }

    #[test]
    fn test_operators() {
        let source = "== != < > <= >=";
        let mut lexer = Lexer::new(source);

        assert_eq!(lexer.next(), Some(Token::EqualsEquals));
        assert_eq!(lexer.next(), Some(Token::BangEquals));
        assert_eq!(lexer.next(), Some(Token::Less));
        assert_eq!(lexer.next(), Some(Token::Greater));
        assert_eq!(lexer.next(), Some(Token::LessEquals));
        assert_eq!(lexer.next(), Some(Token::GreaterEquals));
    }

    #[test]
    fn test_literals() {
        let source = r#"42 3.14 "hello""#;
        let mut lexer = Lexer::new(source);

        assert_eq!(lexer.next(), Some(Token::Number("42".to_string())));
        assert_eq!(lexer.next(), Some(Token::Number("3.14".to_string())));
        assert_eq!(lexer.next(), Some(Token::String("hello".to_string())));
    }
}

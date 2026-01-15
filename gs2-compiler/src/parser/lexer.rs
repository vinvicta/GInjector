//! GS2 Lexer using Logos
//!
//! Tokenizes GS2 source code into a stream of tokens for parsing.

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
    // End of input - Logos 0.13 doesn't need #[error] variant
    #[regex(r"[ \t]+", logos::skip)]
    #[regex(r"//.*", logos::skip)]
    Unknown,

    // Whitespace and newlines (we track newlines for line numbers)
    #[regex(r"\n", priority = 10)]
    Newline,

    // Single-character punctuation
    #[token(".")]
    Dot,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token("|")]
    Pipe,
    #[token("&")]
    Ampersand,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("?")]
    Question,
    #[token("!")]
    Bang,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("^")]
    Caret,
    #[token("%")]
    Percent,
    // Equal for both comparison and assignment
    #[token("=")]
    Equal,
    #[token("~")]
    Tilde,
    #[token("@")]
    At,

    // Multi-character operators
    #[token("&&")]
    OpAnd,
    #[token("||")]
    OpOr,
    #[token("==")]
    OpEquals,
    #[token("!=")]
    OpNotEquals,
    #[token("<>")]
    OpNotEqualsAlt,
    #[token("<=")]
    OpLessThanEqual,
    #[token("=<")]
    OpLessThanEqualAlt,
    #[token(">=")]
    OpGreaterThanEqual,
    #[token("=>")]
    OpGreaterThanEqualAlt,

    // Assignment operators
    #[token("+=")]
    OpAddAssign,
    #[token("-=")]
    OpSubAssign,
    #[token("*=")]
    OpMulAssign,
    #[token("/=")]
    OpDivAssign,
    #[token("^=")]
    OpPowAssign,
    #[token("%=")]
    OpModAssign,
    #[token("@=")]
    OpCatAssign,
    #[token("|=")]
    OpBwOrAssign,
    #[token("&=")]
    OpBwAndAssign,
    #[token("<<=")]
    OpBwLShiftAssign,
    #[token(">>=")]
    OpBwRShiftAssign,

    // Increment/Decrement
    #[token("--")]
    OpDecrement,
    #[token("++")]
    OpIncrement,

    // Bitwise shifts
    #[token("<<")]
    OpBwLShift,
    #[token(">>")]
    OpBwRShift,

    // Keywords
    #[token("public")]
    KwPublic,
    #[token("if")]
    KwIf,
    #[token("else")]
    KwElse,
    #[token("elseif")]
    KwElseif,
    #[token("for")]
    KwFor,
    #[token("while")]
    KwWhile,
    #[token("break")]
    KwBreak,
    #[token("continue")]
    KwContinue,
    #[token("return")]
    KwReturn,
    #[token("function")]
    KwFunction,
    #[token("new")]
    KwNew,
    #[token("with")]
    KwWith,
    #[token("switch")]
    KwSwitch,
    #[token("case")]
    KwCase,
    #[token("default")]
    KwDefault,
    #[token("const")]
    KwConst,
    #[token("enum")]
    KwEnum,
    #[token("int")]
    KwCastInt,
    #[token("float")]
    KwCastFloat,
    #[token("in")]
    KwIn,
    #[token("_")]
    KwTranslate,
    #[token("xor")]
    OpBwXor,

    // Special string concatenation keywords (treated as @)
    #[token("NL")]
    NewlineLiteral,
    #[token("SPC")]
    SpaceLiteral,
    #[token("TAB")]
    TabLiteral,

    // Literals - order matters for priority
    #[regex(r"0x[0-9a-fA-F]+", parse_hex)]
    HexLiteral(i64),

    #[regex(r"[0-9]*\.[0-9]+", parse_float)]
    FloatLiteral(f64),

    #[regex(r"[0-9]+", lex_int)]
    IntLiteral(i64),

    #[regex(r#"'\\?.'"#, parse_char)]
    CharLiteral(char),

    #[regex(r#""[^"]*""#, parse_string)]
    StringLiteral(String),

    // Identifiers (must be last to catch everything else)
    #[regex(r"[a-zA-Z_$][a-zA-Z_0-9]*(::[a-zA-Z_$][a-zA-Z_0-9]*)*")]
    Identifier,
}

fn parse_hex(lex: &mut logos::Lexer<Token>) -> i64 {
    let s = lex.slice();
    i64::from_str_radix(&s[2..], 16).unwrap_or(0)
}

fn parse_float(lex: &mut logos::Lexer<Token>) -> f64 {
    lex.slice().parse().unwrap_or(0.0)
}

fn lex_int(lex: &mut logos::Lexer<Token>) -> i64 {
    lex.slice().parse().unwrap_or(0)
}

fn parse_char(lex: &mut logos::Lexer<Token>) -> char {
    let s = lex.slice();
    // Remove quotes
    let content = &s[1..s.len() - 1];
    // Handle escape sequences
    if content.starts_with('\\') && content.len() > 1 {
        match content.chars().nth(1).unwrap() {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '0' => '\0',
            '\\' => '\\',
            '\'' => '\'',
            _ => content.chars().nth(1).unwrap(),
        }
    } else {
        content.chars().next().unwrap_or('\0')
    }
}

fn parse_string(lex: &mut logos::Lexer<Token>) -> String {
    let s = lex.slice();
    // Remove surrounding quotes
    s[1..s.len() - 1].to_string()
}

impl Token {
    /// Get a human-readable name for this token
    pub fn name(&self) -> &'static str {
        match self {
            Token::Unknown => "unknown",
            Token::Newline => "newline",
            Token::Dot => ".",
            Token::Comma => ",",
            Token::Colon => ":",
            Token::Semicolon => ";",
            Token::Pipe => "|",
            Token::Ampersand => "&",
            Token::LeftParen => "(",
            Token::RightParen => ")",
            Token::LeftBrace => "{",
            Token::RightBrace => "}",
            Token::LeftBracket => "[",
            Token::RightBracket => "]",
            Token::Question => "?",
            Token::Bang => "!",
            Token::Less => "<",
            Token::Greater => ">",
            Token::Plus => "+",
            Token::Minus => "-",
            Token::Star => "*",
            Token::Slash => "/",
            Token::Caret => "^",
            Token::Percent => "%",
            Token::Equal => "=",
            Token::Tilde => "~",
            Token::At => "@",
            Token::OpAnd => "&&",
            Token::OpOr => "||",
            Token::OpEquals => "==",
            Token::OpNotEquals => "!=",
            Token::OpNotEqualsAlt => "<>",
            Token::OpLessThanEqual => "<=",
            Token::OpLessThanEqualAlt => "=<",
            Token::OpGreaterThanEqual => ">=",
            Token::OpGreaterThanEqualAlt => "=>",
            Token::OpAddAssign => "+=",
            Token::OpSubAssign => "-=",
            Token::OpMulAssign => "*=",
            Token::OpDivAssign => "/=",
            Token::OpPowAssign => "^=",
            Token::OpModAssign => "%=",
            Token::OpCatAssign => "@=",
            Token::OpBwOrAssign => "|=",
            Token::OpBwAndAssign => "&=",
            Token::OpBwLShiftAssign => "<<=",
            Token::OpBwRShiftAssign => ">>=",
            Token::OpDecrement => "--",
            Token::OpIncrement => "++",
            Token::OpBwLShift => "<<",
            Token::OpBwRShift => ">>",
            Token::KwPublic => "public",
            Token::KwIf => "if",
            Token::KwElse => "else",
            Token::KwElseif => "elseif",
            Token::KwFor => "for",
            Token::KwWhile => "while",
            Token::KwBreak => "break",
            Token::KwContinue => "continue",
            Token::KwReturn => "return",
            Token::KwFunction => "function",
            Token::KwNew => "new",
            Token::KwWith => "with",
            Token::KwSwitch => "switch",
            Token::KwCase => "case",
            Token::KwDefault => "default",
            Token::KwConst => "const",
            Token::KwEnum => "enum",
            Token::KwCastInt => "int",
            Token::KwCastFloat => "float",
            Token::KwIn => "in",
            Token::KwTranslate => "_",
            Token::OpBwXor => "xor",
            Token::NewlineLiteral => "NL",
            Token::SpaceLiteral => "SPC",
            Token::TabLiteral => "TAB",
            Token::HexLiteral(_) => "hex",
            Token::IntLiteral(_) => "integer",
            Token::FloatLiteral(_) => "float",
            Token::CharLiteral(_) => "char",
            Token::StringLiteral(_) => "string",
            Token::Identifier => "identifier",
        }
    }
}

/// A lexer source location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

/// A token with its location
#[derive(Debug, Clone, PartialEq)]
pub struct LocatedToken {
    pub token: Token,
    pub location: SourceLocation,
}

/// Lexer that produces tokens with location information
pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
            line: 1,
            column: 0,
        }
    }

    pub fn next(&mut self) -> Option<LocatedToken> {
        loop {
            let token_result = self.inner.next()?;

            let token = match token_result {
                Ok(t) => t,
                Err(_) => continue, // Skip errors
            };

            // Track line numbers
            if token == Token::Newline {
                self.line += 1;
                self.column = 0;
                continue;
            }

            // Skip unknown tokens
            if token == Token::Unknown {
                continue;
            }

            let span = self.inner.span();
            let location = SourceLocation {
                line: self.line,
                column: self.column,
                offset: span.start,
            };
            self.column = span.end;

            return Some(LocatedToken { token, location });
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

        assert_eq!(lexer.next().unwrap().token, Token::KwFunction);
        assert_eq!(lexer.next().unwrap().token, Token::Identifier);
        assert_eq!(lexer.next().unwrap().token, Token::LeftParen);
        assert_eq!(lexer.next().unwrap().token, Token::RightParen);
        assert_eq!(lexer.next().unwrap().token, Token::LeftBrace);
        assert_eq!(lexer.next().unwrap().token, Token::KwReturn);
    }

    #[test]
    fn test_operators() {
        let source = "== != < > <= >=";
        let mut lexer = Lexer::new(source);

        assert_eq!(lexer.next().unwrap().token, Token::OpEquals);
        assert_eq!(lexer.next().unwrap().token, Token::OpNotEquals);
        assert_eq!(lexer.next().unwrap().token, Token::Less);
        assert_eq!(lexer.next().unwrap().token, Token::Greater);
        assert_eq!(lexer.next().unwrap().token, Token::OpLessThanEqual);
        assert_eq!(lexer.next().unwrap().token, Token::OpGreaterThanEqual);
    }

    #[test]
    fn test_literals() {
        let source = r#"42 3.14 "hello" 'a'"#;
        let mut lexer = Lexer::new(source);

        assert!(matches!(lexer.next().unwrap().token, Token::IntLiteral(42)));
        assert!(matches!(lexer.next().unwrap().token, Token::FloatLiteral(3.14)));
        let tok = lexer.next().unwrap().token;
        assert!(matches!(tok, Token::StringLiteral(_)));
        if let Token::StringLiteral(s) = tok {
            assert_eq!(s, "hello");
        }
        assert!(matches!(lexer.next().unwrap().token, Token::CharLiteral(_)));
    }
}

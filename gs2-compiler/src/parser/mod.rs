//! Parser module
//!
//! Uses nom parser combinators for GS2 source code.

use crate::ast::expression::{BinaryOp, Expression, UnaryOp};
use crate::ast::identifier::Identifier;
use crate::ast::literal::Literal;
use crate::ast::program::Program;
use crate::ast::statement::{EnumMember, Statement};
use crate::error::{CompileError, SourceLocation};

pub mod lexer;

use lexer::Token;

/// Parse result type
type ParseResult<T> = Result<T, CompileError>;

/// GS2 Parser
pub struct Parser {
    tokens: Vec<(Token, SourceLocation)>,
    pos: usize,
}

impl Parser {
    /// Create a new parser
    pub fn new(source: &str) -> Self {
        let mut lexer = lexer::Lexer::new(source);
        let mut tokens = Vec::new();

        while let Some(token) = lexer.next() {
            if let Token::Whitespace = token {
                continue;
            }
            tokens.push((
                token,
                SourceLocation::default(),
            ));
        }

        Parser {
            tokens,
            pos: 0,
        }
    }

    /// Peek at the current token
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    /// Peek at the next token (token after current)
    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|(t, _)| t)
    }

    /// Peek at the current token with location
    fn peek_with_loc(&self) -> Option<&(Token, SourceLocation)> {
        self.tokens.get(self.pos)
    }

    /// Consume the current token
    fn consume(&mut self) -> Option<Token> {
        self.tokens.get(self.pos).map(|(t, _)| {
            self.pos += 1;
            t.clone()
        })
    }

    /// Check if the current token matches the expected token
    fn check(&self, token: Token) -> bool {
        self.peek() == Some(&token)
    }

    /// Expect a specific token or return an error
    fn expect(&mut self, token: Token) -> ParseResult<()> {
        if self.check(token.clone()) {
            self.consume();
            Ok(())
        } else {
            Err(CompileError::Parse {
                message: format!("expected {:?}, found {:?}", token, self.peek()),
                location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
            })
        }
    }

    /// Parse a complete program
    pub fn parse(&mut self) -> ParseResult<Program> {
        let mut statements = Vec::new();
        while self.peek().is_some() {
            statements.push(*self.parse_statement()?);
        }
        Ok(Program::with_statements(statements))
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> ParseResult<Box<Statement>> {
        match self.peek() {
            Some(Token::KeywordPublic) | Some(Token::KeywordFunction) => {
                self.parse_function_declaration()
            }
            Some(Token::KeywordIf) => self.parse_if_statement(),
            Some(Token::KeywordWhile) => self.parse_while_statement(),
            Some(Token::KeywordFor) => self.parse_for_statement(),
            Some(Token::KeywordWith) => self.parse_with_statement(),
            Some(Token::KeywordSwitch) => self.parse_switch_statement(),
            Some(Token::KeywordReturn) => self.parse_return_statement(),
            Some(Token::KeywordBreak) => self.parse_break_statement(),
            Some(Token::KeywordContinue) => self.parse_continue_statement(),
            Some(Token::KeywordConst) => self.parse_const_declaration(),
            Some(Token::KeywordEnum) => self.parse_enum_declaration(),
            Some(Token::LBrace) => self.parse_block(),
            _ => self.parse_expression_statement(),
        }
    }

    /// Parse a function declaration
    fn parse_function_declaration(&mut self) -> ParseResult<Box<Statement>> {
        let is_public = self.check(Token::KeywordPublic);
        if is_public {
            self.consume();
        }
        self.expect(Token::KeywordFunction)?;

        let name = match self.consume() {
            Some(Token::Identifier(s)) => s,
            _ => return Err(CompileError::Parse {
                message: "expected function name".to_string(),
                location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
            }),
        };

        self.expect(Token::LParen)?;

        let mut params = Vec::new();
        if !self.check(Token::RParen) {
            loop {
                if let Some(Token::Identifier(param)) = self.consume() {
                    params.push(Identifier::with_name(param));
                } else {
                    return Err(CompileError::Parse {
                        message: "expected parameter name".to_string(),
                        location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
                    });
                }

                if !self.check(Token::Comma) {
                    break;
                }
                self.consume();
            }
        }
        self.expect(Token::RParen)?;

        let body = self.parse_block()?;

        Ok(Box::new(Statement::FunctionDeclaration {
            name: Identifier::with_name(name),
            params,
            body,
            is_public,
            location: SourceLocation::new(),
        }))
    }

    /// Parse a const declaration: `const NAME = VALUE;`
    fn parse_const_declaration(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordConst)?;

        let name = match self.consume() {
            Some(Token::Identifier(s)) => Identifier::with_name(s),
            _ => return Err(CompileError::Parse {
                message: "expected const name".to_string(),
                location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
            }),
        };

        self.expect(Token::Equals)?;

        // Parse the value - can be a literal or identifier (reference to another const)
        let value = self.parse_const_value()?;

        self.expect(Token::Semicolon)?;

        Ok(Box::new(Statement::Const {
            name,
            value,
            location: SourceLocation::new(),
        }))
    }

    /// Parse a const value - can be a literal, unary negation of a literal, or identifier
    fn parse_const_value(&mut self) -> ParseResult<Expression> {
        // Check for unary minus (negative numbers)
        if self.check(Token::Minus) {
            self.consume();
            let expr = self.parse_primary()?;
            // Negate the literal value
            match expr {
                Expression::Literal { literal, location } => {
                    match *literal {
                        Literal::Number { value, .. } => {
                            // Negate the number string
                            let negated = if value.starts_with('-') {
                                value[1..].to_string()
                            } else {
                                format!("-{}", value)
                            };
                            return Ok(Expression::Literal {
                                literal: Box::new(Literal::Number { value: negated, location }),
                                location,
                            });
                        }
                        _ => return Err(CompileError::Parse {
                            message: "const value must be a number, string, or identifier".to_string(),
                            location,
                        }),
                    }
                }
                _ => return Err(CompileError::Parse {
                    message: "can only negate literal values in const declarations".to_string(),
                    location: expr.location().clone(),
                }),
            }
        }

        // Parse as a regular expression (literal or identifier)
        let expr = self.parse_primary()?;
        // Validate that it's a valid const value (literal or identifier)
        match &expr {
            Expression::Literal { .. } | Expression::Identifier { .. } => Ok(expr),
            _ => Err(CompileError::Parse {
                message: "const value must be a literal or identifier".to_string(),
                location: expr.location().clone(),
            }),
        }
    }

    /// Parse an enum declaration: `enum Name { MEMBER1, MEMBER2 = value, ... }` or `enum { MEMBER1, ... }`
    fn parse_enum_declaration(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordEnum)?;

        // Check if there's a name (named enum) or just { (anonymous enum)
        let name = match self.peek() {
            Some(Token::Identifier(s)) => {
                let name = Identifier::with_name(s.clone());
                self.consume(); // consume the identifier
                Some(name)
            }
            Some(Token::LBrace) => None,
            _ => return Err(CompileError::Parse {
                message: "expected enum name or {{".to_string(),
                location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
            }),
        };

        self.expect(Token::LBrace)?;

        let mut members = Vec::new();
        let mut next_auto_value = 0i32;

        // Parse enum members
        loop {
            // Check for end of enum
            if self.check(Token::RBrace) {
                self.consume();
                break;
            }

            // Parse member name
            let member_name = match self.consume() {
                Some(Token::Identifier(s)) => Identifier::with_name(s),
                _ => return Err(CompileError::Parse {
                    message: "expected enum member name".to_string(),
                    location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
                }),
            };

            // Check for explicit value: `MEMBER = value`
            let member_value = if self.check(Token::Equals) {
                self.consume();
                // Check for negative numbers
                if self.check(Token::Minus) {
                    self.consume();
                    match self.consume() {
                        Some(Token::Number(n)) => {
                            let negated = if n.starts_with('-') {
                                n[1..].to_string()
                            } else {
                                format!("-{}", n)
                            };
                            // Parse as integer and update auto-increment
                            if let Ok(val) = negated.parse::<i32>() {
                                next_auto_value = val + 1;
                                Some(Expression::Literal {
                                    literal: Box::new(Literal::Number { value: negated, location: SourceLocation::new() }),
                                    location: SourceLocation::new(),
                                })
                            } else {
                                return Err(CompileError::Parse {
                                    message: "enum value must be an integer".to_string(),
                                    location: SourceLocation::new(),
                                });
                            }
                        }
                        _ => return Err(CompileError::Parse {
                            message: "expected number after minus".to_string(),
                            location: SourceLocation::new(),
                        }),
                    }
                } else {
                    match self.consume() {
                        Some(Token::Number(n)) => {
                            // Parse as integer and update auto-increment
                            if let Ok(val) = n.parse::<i32>() {
                                next_auto_value = val + 1;
                                Some(Expression::Literal {
                                    literal: Box::new(Literal::Number { value: n, location: SourceLocation::new() }),
                                    location: SourceLocation::new(),
                                })
                            } else {
                                return Err(CompileError::Parse {
                                    message: "enum value must be an integer".to_string(),
                                    location: SourceLocation::new(),
                                });
                            }
                        }
                        _ => return Err(CompileError::Parse {
                            message: "expected integer value for enum member".to_string(),
                            location: SourceLocation::new(),
                        }),
                    }
                }
            } else {
                // Auto-increment value
                let value = next_auto_value;
                next_auto_value += 1;
                Some(Expression::Literal {
                    literal: Box::new(Literal::Number { value: value.to_string(), location: SourceLocation::new() }),
                    location: SourceLocation::new(),
                })
            };

            members.push(EnumMember {
                name: member_name,
                value: member_value,
            });

            // Check for comma
            if !self.check(Token::Comma) {
                // No comma - might be end of enum
                if !self.check(Token::RBrace) {
                    return Err(CompileError::Parse {
                        message: "expected comma or } after enum member".to_string(),
                        location: SourceLocation::new(),
                    });
                }
            } else {
                self.consume(); // consume comma
            }
        }

        // Consume trailing semicolon if present
        if self.check(Token::Semicolon) {
            self.consume();
        }

        Ok(Box::new(Statement::Enum {
            name,
            members,
            location: SourceLocation::new(),
        }))
    }

    /// Parse an if statement
    fn parse_if_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordIf)?;
        let condition = self.parse_expression()?;

        // Parse body - can be a block or single statement
        let true_block = if self.check(Token::LBrace) {
            self.parse_block()?
        } else {
            self.parse_statement()?
        };

        let false_block = if self.check(Token::KeywordElse) {
            self.consume();
            // Check if next is "if" for "else if", or if we already have "elseif"
            if self.check(Token::KeywordIf) {
                // "else if" pattern
                Some(self.parse_if_statement()?)
            } else {
                // Regular else block
                if self.check(Token::LBrace) {
                    Some(self.parse_block()?)
                } else {
                    Some(self.parse_statement()?)
                }
            }
        } else if self.check(Token::KeywordElseIf) {
            // "elseif" is a single keyword meaning "else if"
            // We need to parse: elseif (condition) statement
            // And then check for more elseif/else clauses after the statement
            self.consume(); // consume "elseif"
            let elseif_condition = self.parse_expression()?;
            let elseif_block = if self.check(Token::LBrace) {
                self.parse_block()?
            } else {
                self.parse_statement()?
            };
            // Create the elseif if statement and check for more elseif/else
            // by recursively calling parse_if_statement for the else clause
            Some(Box::new(Statement::If {
                condition: elseif_condition,
                true_block: elseif_block,
                // Recursively handle any additional elseif/else
                false_block: if self.check(Token::KeywordElse) || self.check(Token::KeywordElseIf) {
                    Some(self.parse_else_chain()?)
                } else {
                    None
                },
                location: SourceLocation::new(),
            }))
        } else {
            None
        };

        Ok(Box::new(Statement::If {
            condition,
            true_block,
            false_block,
            location: SourceLocation::new(),
        }))
    }

    /// Parse an else clause chain after an if/elseif block
    /// Handles: "else { block }", "else statement", "elseif (condition) statement"
    fn parse_else_chain(&mut self) -> ParseResult<Box<Statement>> {
        if self.check(Token::KeywordElse) {
            self.consume(); // consume "else"
            if self.check(Token::KeywordIf) {
                // "else if" - recursively parse the if statement
                self.parse_if_statement()
            } else {
                // Regular else block
                let block = if self.check(Token::LBrace) {
                    self.parse_block()?
                } else {
                    self.parse_statement()?
                };
                Ok(Box::new(Statement::Block {
                    statements: vec![*block],
                    location: SourceLocation::new(),
                }))
            }
        } else if self.check(Token::KeywordElseIf) {
            // "elseif (condition) statement" - equivalent to "else if (condition) statement"
            self.consume(); // consume "elseif"
            let condition = self.parse_expression()?;
            let true_block = if self.check(Token::LBrace) {
                self.parse_block()?
            } else {
                self.parse_statement()?
            };
            // Check for more elseif/else after this elseif block
            let false_block = if self.check(Token::KeywordElse) || self.check(Token::KeywordElseIf) {
                Some(self.parse_else_chain()?)
            } else {
                None
            };
            Ok(Box::new(Statement::If {
                condition,
                true_block,
                false_block,
                location: SourceLocation::new(),
            }))
        } else {
            Err(CompileError::Parse {
                message: "expected 'else' or 'elseif'".to_string(),
                location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
            })
        }
    }

    /// Parse a while statement
    fn parse_while_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordWhile)?;
        let condition = self.parse_expression()?;

        // Parse body - can be a block or single statement
        let body = if self.check(Token::LBrace) {
            self.parse_block()?
        } else {
            self.parse_statement()?
        };

        Ok(Box::new(Statement::While {
            condition,
            body,
            location: SourceLocation::new(),
        }))
    }

    /// Parse a for statement
    ///
    /// Handles both regular C-style for loops: `for (init; condition; increment) { body }`
    /// and for-each loops: `for (item: array) { body }` or `for (item: array) statement`
    fn parse_for_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordFor)?;
        self.expect(Token::LParen)?;

        // Parse the first expression - could be:
        // - For regular for: init expression (optional)
        // - For for-each: item variable name
        let first_expr = if self.check(Token::Semicolon) {
            // Empty init, definitely not a for-each
            self.consume();
            None
        } else {
            Some(self.parse_expression()?)
        };

        // Check if this is a for-each loop (colon after first expression)
        if first_expr.is_some() && self.check(Token::Colon) {
            // This is a for-each loop: `for (item: array) statement`
            self.consume(); // consume colon

            // Parse the array expression
            let array_expr = self.parse_expression()?;
            self.expect(Token::RParen)?;

            // Extract the item identifier from the first expression
            // Supports both: `for (x: array)` and `for (temp.x: array)`
            let item = match first_expr.unwrap() {
                Expression::Identifier { identifier, .. } => *identifier,
                Expression::MemberAccess { property, .. } => property,
                _ => {
                    return Err(CompileError::Parse {
                        message: "for-each loop variable must be an identifier".to_string(),
                        location: SourceLocation::new(),
                    });
                }
            };

            // Parse body - can be a block or single statement
            let body = if self.check(Token::LBrace) {
                self.parse_block()?
            } else {
                self.parse_statement()?
            };

            return Ok(Box::new(Statement::ForEach {
                item,
                array: array_expr,
                body,
                location: SourceLocation::new(),
            }));
        }

        // Regular for loop: `for (init; condition; increment) statement`
        let init = match first_expr {
            Some(expr) => {
                self.expect(Token::Semicolon)?;
                Some(Box::new(Statement::Expression {
                    expr,
                    location: SourceLocation::new(),
                }))
            }
            None => None,
        };

        let condition = if self.check(Token::Semicolon) {
            self.consume();
            None
        } else {
            let cond = self.parse_expression()?;
            self.expect(Token::Semicolon)?;
            Some(cond)
        };

        let increment = if self.check(Token::RParen) {
            None
        } else {
            Some(Box::new(Statement::Expression {
                expr: self.parse_expression()?,
                location: SourceLocation::new(),
            }))
        };

        self.expect(Token::RParen)?;

        // Parse body - can be a block or single statement
        let body = if self.check(Token::LBrace) {
            self.parse_block()?
        } else {
            self.parse_statement()?
        };

        Ok(Box::new(Statement::For {
            init,
            condition,
            increment,
            body,
            location: SourceLocation::new(),
        }))
    }

    /// Parse a return statement
    fn parse_return_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordReturn)?;

        let expr = if matches!(
            self.peek(),
            Some(Token::Semicolon) | Some(Token::RBrace) | None
        ) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        // Consume optional semicolon
        if self.check(Token::Semicolon) {
            self.consume();
        }

        Ok(Box::new(Statement::Return {
            expr,
            location: SourceLocation::new(),
        }))
    }

    /// Parse a break statement
    fn parse_break_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordBreak)?;

        // Consume optional semicolon
        if self.check(Token::Semicolon) {
            self.consume();
        }

        Ok(Box::new(Statement::Break {
            location: SourceLocation::new(),
        }))
    }

    /// Parse a continue statement
    fn parse_continue_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordContinue)?;

        // Consume optional semicolon
        if self.check(Token::Semicolon) {
            self.consume();
        }

        Ok(Box::new(Statement::Continue {
            location: SourceLocation::new(),
        }))
    }

    /// Parse a with statement
    fn parse_with_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordWith)?;
        let obj = self.parse_expression()?;
        let body = self.parse_block()?;

        Ok(Box::new(Statement::With {
            obj,
            body,
            location: SourceLocation::new(),
        }))
    }

    /// Parse a switch statement
    fn parse_switch_statement(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::KeywordSwitch)?;
        let expr = self.parse_expression()?;
        self.expect(Token::LBrace)?;

        let mut cases = Vec::new();
        let mut default_case = None;

        while !self.check(Token::RBrace) && self.peek().is_some() {
            if self.check(Token::KeywordCase) {
                self.consume();
                let case_expr = self.parse_expression()?;
                self.expect(Token::Colon)?;
                let case_body = self.parse_block()?;
                cases.push((case_expr, case_body));
            } else if self.check(Token::KeywordDefault) {
                self.consume();
                self.expect(Token::Colon)?;
                default_case = Some(self.parse_block()?);
            } else {
                return Err(CompileError::Parse {
                    message: "expected 'case' or 'default' in switch statement".to_string(),
                    location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
                });
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Box::new(Statement::Switch {
            expr,
            cases,
            default_case,
            location: SourceLocation::new(),
        }))
    }

    /// Parse a block
    fn parse_block(&mut self) -> ParseResult<Box<Statement>> {
        self.expect(Token::LBrace)?;

        let mut statements = Vec::new();
        while !self.check(Token::RBrace) && self.peek().is_some() {
            statements.push(*self.parse_statement()?);
        }

        self.expect(Token::RBrace)?;

        Ok(Box::new(Statement::Block {
            statements,
            location: SourceLocation::new(),
        }))
    }

    /// Parse an expression statement
    fn parse_expression_statement(&mut self) -> ParseResult<Box<Statement>> {
        let expr = self.parse_expression()?;

        // Consume optional semicolon
        if self.check(Token::Semicolon) {
            self.consume();
        }

        Ok(Box::new(Statement::Expression {
            expr,
            location: SourceLocation::new(),
        }))
    }

    /// Parse an expression (highest precedence)
    fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_ternary()
    }

    /// Parse ternary expressions (right-associative)
    fn parse_ternary(&mut self) -> ParseResult<Expression> {
        let condition = self.parse_assignment()?;

        if self.check(Token::Question) {
            self.consume();
            let true_expr = self.parse_assignment()?;

            if self.check(Token::Colon) {
                self.consume();
                let false_expr = self.parse_ternary()?;
                return Ok(Expression::Ternary {
                    condition: Box::new(condition),
                    true_expr: Box::new(true_expr),
                    false_expr: Box::new(false_expr),
                    location: SourceLocation::new(),
                });
            }

            return Err(CompileError::Parse {
                message: "expected ':' in ternary expression".to_string(),
                location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
            });
        }

        Ok(condition)
    }

    /// Parse assignment expressions
    fn parse_assignment(&mut self) -> ParseResult<Expression> {
        let left = self.parse_logical_or()?;

        if let Some(Token::Equals) = self.peek() {
            self.consume();
            let right = self.parse_assignment()?;
            return Ok(Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Assign,
                right: Box::new(right),
                location: SourceLocation::new(),
            });
        }

        // Check for compound assignment operators
        if let Some(op) = match self.peek() {
            Some(Token::PlusEquals) => Some(BinaryOp::AddAssign),
            Some(Token::MinusEquals) => Some(BinaryOp::SubtractAssign),
            Some(Token::StarEquals) => Some(BinaryOp::MultiplyAssign),
            Some(Token::SlashEquals) => Some(BinaryOp::DivideAssign),
            Some(Token::PercentEquals) => Some(BinaryOp::ModuloAssign),
            Some(Token::CaretEquals) => Some(BinaryOp::PowerAssign),
            Some(Token::LShiftEquals) => Some(BinaryOp::LeftShiftAssign),
            Some(Token::RShiftEquals) => Some(BinaryOp::RightShiftAssign),
            Some(Token::AmpEquals) => Some(BinaryOp::BitwiseAndAssign),
            Some(Token::PipeEquals) => Some(BinaryOp::BitwiseOrAssign),
            _ => None,
        } {
            self.consume();
            let right = self.parse_assignment()?;
            return Ok(Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location: SourceLocation::new(),
            });
        }

        Ok(left)
    }

    /// Parse logical OR expressions
    fn parse_logical_or(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_logical_and()?;

        while self.check(Token::PipePipe) {
            self.consume();
            let right = self.parse_logical_and()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::LogicalOr,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse logical AND expressions
    fn parse_logical_and(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_or()?;

        while self.check(Token::AmpAmp) {
            self.consume();
            let right = self.parse_bitwise_or()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::LogicalAnd,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse bitwise OR expressions
    fn parse_bitwise_or(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_xor()?;

        while self.check(Token::Pipe) {
            self.consume();
            let right = self.parse_bitwise_xor()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::BitwiseOr,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse bitwise XOR expressions
    fn parse_bitwise_xor(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_and()?;

        while self.check(Token::Caret) {
            self.consume();
            let right = self.parse_bitwise_and()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::BitwiseXor,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse bitwise AND expressions
    fn parse_bitwise_and(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_equality()?;

        while self.check(Token::Amp) {
            self.consume();
            let right = self.parse_equality()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::BitwiseAnd,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse equality expressions
    fn parse_equality(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_comparison()?;

        // Check for 'in' operator first (has different syntax)
        if let Some(Token::KeywordIn) = self.peek() {
            self.consume();
            return self.parse_in_expression(left);
        }

        while let Some(op) = match self.peek() {
            Some(Token::EqualsEquals) => Some(BinaryOp::Equal),
            Some(Token::BangEquals) => Some(BinaryOp::NotEqual),
            _ => None,
        } {
            self.consume();
            let right = self.parse_comparison()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse an 'in' expression
    /// Supports:
    /// - value in array
    /// - value in |lower, upper|
    /// - value in <lower, upper>
    fn parse_in_expression(&mut self, value: Expression) -> ParseResult<Expression> {
        // Check for range syntax: |lower, upper| or <lower, upper>
        let (container, upper_bound) = match self.peek() {
            Some(Token::Pipe) => {
                // |lower, upper| syntax
                self.consume(); // consume |
                let lower = self.parse_shift()?; // Use shift to avoid consuming |
                self.expect(Token::Comma)?;
                let upper = self.parse_shift()?;
                self.expect(Token::Pipe)?;
                (lower, Some(upper))
            }
            Some(Token::Less) => {
                // <lower, upper> syntax
                self.consume(); // consume <
                let lower = self.parse_shift()?; // Use shift to avoid consuming >
                self.expect(Token::Comma)?;
                let upper = self.parse_shift()?;
                self.expect(Token::Greater)?;
                (lower, Some(upper))
            }
            _ => {
                // Simple array/object check: value in array
                let container = self.parse_shift()?; // Use shift to maintain precedence
                (container, None)
            }
        };

        Ok(Expression::In {
            value: Box::new(value),
            container: Box::new(container),
            upper_bound: upper_bound.map(Box::new),
            location: SourceLocation::new(),
        })
    }

    /// Parse comparison expressions
    fn parse_comparison(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_shift()?;

        while let Some(op) = match self.peek() {
            Some(Token::LessEquals) => Some(BinaryOp::LessThanOrEqual),
            Some(Token::GreaterEquals) => Some(BinaryOp::GreaterThanOrEqual),
            Some(Token::Less) => Some(BinaryOp::LessThan),
            Some(Token::Greater) => Some(BinaryOp::GreaterThan),
            _ => None,
        } {
            self.consume();
            let right = self.parse_shift()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse shift expressions
    fn parse_shift(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_additive()?;

        while let Some(op) = match self.peek() {
            Some(Token::LShift) => Some(BinaryOp::LeftShift),
            Some(Token::RShift) => Some(BinaryOp::RightShift),
            _ => None,
        } {
            self.consume();
            let right = self.parse_additive()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse additive expressions
    fn parse_additive(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_multiplicative()?;

        while let Some(op) = match self.peek() {
            Some(Token::Plus) => Some(BinaryOp::Add),
            Some(Token::Minus) => Some(BinaryOp::Subtract),
            _ => None,
        } {
            self.consume();
            let right = self.parse_multiplicative()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse multiplicative expressions
    fn parse_multiplicative(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_power()?;

        while let Some(op) = match self.peek() {
            Some(Token::Star) => Some(BinaryOp::Multiply),
            Some(Token::Slash) => Some(BinaryOp::Divide),
            Some(Token::Percent) => Some(BinaryOp::Modulo),
            _ => None,
        } {
            self.consume();
            let right = self.parse_power()?;
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location: SourceLocation::new(),
            };
        }

        Ok(left)
    }

    /// Parse power expressions (right-associative)
    fn parse_power(&mut self) -> ParseResult<Expression> {
        let left = self.parse_unary()?;

        if self.check(Token::Caret) {
            self.consume();
            let right = self.parse_power()?;
            return Ok(Expression::BinaryOp {
                left: Box::new(left),
                op: BinaryOp::Power,
                right: Box::new(right),
                location: SourceLocation::new(),
            });
        }

        Ok(left)
    }

    /// Parse unary expressions
    fn parse_unary(&mut self) -> ParseResult<Expression> {
        if let Some(op) = match self.peek() {
            Some(Token::Bang) => Some(UnaryOp::LogicalNot),
            Some(Token::Minus) => Some(UnaryOp::Negate),
            Some(Token::Tilde) => Some(UnaryOp::BitwiseInvert),
            _ => None,
        } {
            self.consume();
            let expr = self.parse_unary()?;
            return Ok(Expression::UnaryOp {
                op,
                expr: Box::new(expr),
                location: SourceLocation::new(),
            });
        }

        self.parse_postfix()
    }

    /// Parse postfix expressions (function calls, member access, array access, inc/dec)
    fn parse_postfix(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Some(Token::LParen) => {
                    self.consume();
                    let mut args = Vec::new();

                    if !self.check(Token::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.check(Token::Comma) {
                                break;
                            }
                            self.consume();
                        }
                    }

                    self.expect(Token::RParen)?;

                    // Check for cast_int(expr) and cast_float(expr)
                    if let Expression::Identifier { identifier, .. } = &expr {
                        if identifier.name == "cast_int" && args.len() == 1 {
                            expr = Expression::Cast {
                                expr: Box::new(args.into_iter().next().unwrap()),
                                target_type: crate::ast::expression::CastType::Integer,
                                location: SourceLocation::new(),
                            };
                        } else if identifier.name == "cast_float" && args.len() == 1 {
                            expr = Expression::Cast {
                                expr: Box::new(args.into_iter().next().unwrap()),
                                target_type: crate::ast::expression::CastType::Float,
                                location: SourceLocation::new(),
                            };
                        } else {
                            expr = Expression::FunctionCall {
                                target: Box::new(expr),
                                args,
                                location: SourceLocation::new(),
                            };
                        }
                    } else {
                        expr = Expression::FunctionCall {
                            target: Box::new(expr),
                            args,
                            location: SourceLocation::new(),
                        };
                    };
                }
                Some(Token::LBracket) => {
                    self.consume();
                    let index = self.parse_expression()?;
                    self.expect(Token::RBracket)?;

                    expr = Expression::ArrayAccess {
                        array: Box::new(expr),
                        index: Box::new(index),
                        location: SourceLocation::new(),
                    };
                }
                Some(Token::Dot) => {
                    self.consume();
                    if let Some(Token::Identifier(prop)) = self.consume() {
                        expr = Expression::MemberAccess {
                            object: Box::new(expr),
                            property: Identifier::with_name(prop),
                            location: SourceLocation::new(),
                        };
                    } else {
                        return Err(CompileError::Parse {
                            message: "expected property name after '.'".to_string(),
                            location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
                        });
                    }
                }
                Some(Token::Colon) => {
                    // Check for scope resolution operator :: (enum member access)
                    // Only treat as :: if the next token is also Colon
                    if self.peek_next() == Some(&Token::Colon) {
                        self.consume(); // consume first colon
                        self.consume(); // consume second colon

                        if let Some(Token::Identifier(member)) = self.consume() {
                            // Create a combined identifier like "EnumName::MemberName"
                            // For enum member access, we create a single identifier with the scope resolution
                            let enum_name = if let Expression::Identifier { identifier, .. } = &expr {
                                &identifier.name
                            } else {
                                return Err(CompileError::Parse {
                                    message: "expected enum name before ::".to_string(),
                                    location: SourceLocation::new(),
                                });
                            };
                            let scoped_name = format!("{}::{}", enum_name, member);
                            expr = Expression::Identifier {
                                identifier: Box::new(Identifier::with_name(scoped_name)),
                                location: SourceLocation::new(),
                            };
                        } else {
                            return Err(CompileError::Parse {
                                message: "expected enum member name after ::".to_string(),
                                location: SourceLocation::new(),
                            });
                        }
                    } else {
                        // Single colon - this is not part of ::, so it's for other syntax (ternary, for-each)
                        // Don't consume it, let it be handled by the parent parser
                        break;
                    }
                }
                Some(Token::PlusPlus) => {
                    self.consume();
                    expr = Expression::UnaryOp {
                        op: crate::ast::expression::UnaryOp::PostInc,
                        expr: Box::new(expr),
                        location: SourceLocation::new(),
                    };
                }
                Some(Token::MinusMinus) => {
                    self.consume();
                    expr = Expression::UnaryOp {
                        op: crate::ast::expression::UnaryOp::PostDec,
                        expr: Box::new(expr),
                        location: SourceLocation::new(),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse primary expressions
    fn parse_primary(&mut self) -> ParseResult<Expression> {
        match self.consume() {
            Some(Token::LBracket) => {
                // Array literal
                let mut elements = Vec::new();

                if !self.check(Token::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.check(Token::Comma) {
                            break;
                        }
                        self.consume();
                    }
                }

                self.expect(Token::RBracket)?;

                Ok(Expression::ArrayLiteral {
                    elements,
                    location: SourceLocation::new(),
                })
            }
            Some(Token::LBrace) => {
                // Object literal
                let mut properties = Vec::new();

                if !self.check(Token::RBrace) {
                    loop {
                        if let Some(Token::Identifier(key)) = self.consume() {
                            self.expect(Token::Colon)?;
                            let value = self.parse_expression()?;
                            properties.push((Identifier::with_name(key), value));

                            if !self.check(Token::Comma) {
                                break;
                            }
                            self.consume();
                        } else {
                            return Err(CompileError::Parse {
                                message: "expected property name in object literal".to_string(),
                                location: self.peek_with_loc().map(|(_, loc)| *loc).unwrap_or_default(),
                            });
                        }
                    }
                }

                self.expect(Token::RBrace)?;

                Ok(Expression::ObjectLiteral {
                    properties,
                    location: SourceLocation::new(),
                })
            }
            Some(Token::LParen) => {
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::Number(n)) => Ok(Expression::Literal {
                literal: Box::new(Literal::Number {
                    value: n,
                    location: SourceLocation::new(),
                }),
                location: SourceLocation::new(),
            }),
            Some(Token::String(s)) => Ok(Expression::Literal {
                literal: Box::new(Literal::String {
                    value: s,
                    location: SourceLocation::new(),
                }),
                location: SourceLocation::new(),
            }),
            Some(Token::KeywordTrue) => Ok(Expression::Literal {
                literal: Box::new(Literal::Boolean {
                    value: true,
                    location: SourceLocation::new(),
                }),
                location: SourceLocation::new(),
            }),
            Some(Token::KeywordFalse) => Ok(Expression::Literal {
                literal: Box::new(Literal::Boolean {
                    value: false,
                    location: SourceLocation::new(),
                }),
                location: SourceLocation::new(),
            }),
            Some(Token::KeywordNull) => Ok(Expression::Literal {
                literal: Box::new(Literal::Null {
                    location: SourceLocation::new(),
                }),
                location: SourceLocation::new(),
            }),
            Some(Token::Identifier(name)) => Ok(Expression::Identifier {
                identifier: Box::new(Identifier::with_name(name)),
                location: SourceLocation::new(),
            }),
            Some(token) => Err(CompileError::Parse {
                message: format!("unexpected token: {:?}", token),
                location: SourceLocation::new(),
            }),
            None => Err(CompileError::Parse {
                message: "unexpected end of input".to_string(),
                location: SourceLocation::new(),
            }),
        }
    }
}

/// Parse GS2 source code into an AST
pub fn parse(source: &str) -> Result<Program, CompileError> {
    let mut parser = Parser::new(source);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_expression() {
        let source = "1 + 2;";
        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_function_call() {
        let source = "foo();";
        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_member_access() {
        let source = "foo.bar;";
        let result = parse(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_function_declaration() {
        let source = "function test() { return 42; }";
        let result = parse(source);
        if let Err(e) = &result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_if_statement() {
        let source = "if (x) { return 1; } else { return 2; }";
        let result = parse(source);
        if let Err(e) = &result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok());
    }
}

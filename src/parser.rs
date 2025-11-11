use crate::ast::{Procedure, Program, Statement, Expr, BinaryOp, UnaryOp};
use crate::error::{LppoError, LppoResult};
use crate::lexer::{Lexer, Token};

/// Parses LPPO source code into an abstract syntax tree.
pub fn parse_program(source: &str) -> LppoResult<Program> {
    let mut parser = Parser::new(source)?;
    parser.parse_program()
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    lookahead: Token,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> LppoResult<Self> {
        let mut lexer = Lexer::new(source);
        let lookahead = lexer.next_token()?;
        let position = lexer.position();
        Ok(Self {
            lexer,
            lookahead,
            position,
        })
    }

    fn parse_program(&mut self) -> LppoResult<Program> {
        let mut procedures = Vec::new();
        while self.lookahead != Token::Eof {
            procedures.push(self.parse_procedure()?);
        }
        Ok(Program { procedures })
    }

    fn parse_procedure(&mut self) -> LppoResult<Procedure> {
        self.expect(Token::Proc)?;
        let name = match self.lookahead.clone() {
            Token::Identifier(name) => {
                self.advance()?;
                name
            }
            _ => {
                return Err(LppoError::UnexpectedToken {
                    expected: "procedure name",
                    position: self.position,
                })
            }
        };
        // optional parameter list
        let mut params = Vec::new();
        if self.lookahead == Token::LParen {
            self.advance()?; // consume '('
            if self.lookahead != Token::RParen {
                loop {
                    match self.lookahead.clone() {
                        Token::Identifier(name) => {
                            params.push(name);
                            self.advance()?;
                        }
                        _ => {
                            return Err(LppoError::UnexpectedToken {
                                expected: "parameter name",
                                position: self.position,
                            })
                        }
                    }
                    if self.lookahead == Token::Comma {
                        self.advance()?;
                        continue;
                    }
                    break;
                }
            }
            self.expect(Token::RParen)?;
        }

        self.expect(Token::LBrace)?;
        let mut statements = Vec::new();
        while self.lookahead != Token::RBrace {
            statements.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Procedure { name, params, statements })
    }

    fn parse_statement(&mut self) -> LppoResult<Statement> {
        match self.lookahead.clone() {
            Token::Let => self.parse_let(),
            Token::Out => self.parse_emit(),
            Token::Call => self.parse_call(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Loop => self.parse_loop(),
            Token::Break => {
                self.advance()?;
                self.expect(Token::Semicolon)?;
                Ok(Statement::Break)
            }
            Token::Continue => {
                self.advance()?;
                self.expect(Token::Semicolon)?;
                Ok(Statement::Continue)
            }
            Token::Return => self.parse_return(),
            Token::Parallel => self.parse_parallel(),
            Token::Eof | Token::RBrace => Err(LppoError::UnexpectedEndOfInput),
            _ => Err(LppoError::UnexpectedToken {
                expected: "statement",
                position: self.position,
            }),
        }
    }

    fn parse_if(&mut self) -> LppoResult<Statement> {
        self.expect(Token::If)?;
        let cond = self.parse_expression()?;
        self.expect(Token::LBrace)?;
        let mut then_branch = Vec::new();
        while self.lookahead != Token::RBrace {
            then_branch.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;

        let mut elifs = Vec::new();
        while self.lookahead == Token::Elif {
            self.advance()?; // consume 'elif'
            let econd = self.parse_expression()?;
            self.expect(Token::LBrace)?;
            let mut estmts = Vec::new();
            while self.lookahead != Token::RBrace {
                estmts.push(self.parse_statement()?);
            }
            self.expect(Token::RBrace)?;
            elifs.push((econd, estmts));
        }

        let mut else_branch = None;
        if self.lookahead == Token::Else {
            self.advance()?;
            self.expect(Token::LBrace)?;
            let mut estmts = Vec::new();
            while self.lookahead != Token::RBrace {
                estmts.push(self.parse_statement()?);
            }
            self.expect(Token::RBrace)?;
            else_branch = Some(estmts);
        }

        Ok(Statement::If { cond, then_branch, elifs, else_branch })
    }

    fn parse_while(&mut self) -> LppoResult<Statement> {
        self.expect(Token::While)?;
        let cond = self.parse_expression()?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.lookahead != Token::RBrace {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::While { cond, body })
    }

    fn parse_for(&mut self) -> LppoResult<Statement> {
        self.expect(Token::For)?;
        let var = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::UnexpectedToken {
                    expected: "variable name",
                    position: self.position,
                })
            }
        };
        self.expect(Token::In)?;
        let iterable = self.parse_expression()?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.lookahead != Token::RBrace {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::For { var, iterable, body })
    }

    fn parse_loop(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Loop)?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.lookahead != Token::RBrace {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::Loop { body })
    }

    fn parse_return(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Return)?;
        if self.lookahead == Token::Semicolon {
            self.advance()?;
            Ok(Statement::Return(None))
        } else {
            let expr = self.parse_expression()?;
            self.expect(Token::Semicolon)?;
            Ok(Statement::Return(Some(expr)))
        }
    }

    fn parse_emit(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Out)?;
        let mut exprs = Vec::new();
        exprs.push(self.parse_expression()?);
        while self.lookahead == Token::Comma {
            self.advance()?; // consume comma
            exprs.push(self.parse_expression()?);
        }
        self.expect(Token::Semicolon)?;
        Ok(Statement::Emit(exprs))
    }

    fn parse_let(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Let)?;
        let name = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::UnexpectedToken {
                    expected: "identifier",
                    position: self.position,
                })
            }
        };
        // expect an equals token
        self.expect(Token::Equals)?;
        let expr = self.parse_expression()?;
        self.expect(Token::Semicolon)?;
        Ok(Statement::Let(name, expr))
    }

    fn parse_expression(&mut self) -> LppoResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> LppoResult<Expr> {
        let mut node = self.parse_and()?;
        while self.lookahead == Token::Or {
            self.advance()?;
            let rhs = self.parse_and()?;
            node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Or, right: Box::new(rhs) };
        }
        Ok(node)
    }

    fn parse_and(&mut self) -> LppoResult<Expr> {
        let mut node = self.parse_equality()?;
        while self.lookahead == Token::And {
            self.advance()?;
            let rhs = self.parse_equality()?;
            node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::And, right: Box::new(rhs) };
        }
        Ok(node)
    }

    fn parse_equality(&mut self) -> LppoResult<Expr> {
        let mut node = self.parse_add_sub()?;
        loop {
            match self.lookahead {
                Token::EqEq => {
                    self.advance()?;
                    let rhs = self.parse_add_sub()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Eq, right: Box::new(rhs) };
                }
                Token::NotEq => {
                    self.advance()?;
                    let rhs = self.parse_add_sub()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Ne, right: Box::new(rhs) };
                }
                Token::Less => {
                    self.advance()?;
                    let rhs = self.parse_add_sub()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Lt, right: Box::new(rhs) };
                }
                Token::LessEq => {
                    self.advance()?;
                    let rhs = self.parse_add_sub()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Le, right: Box::new(rhs) };
                }
                Token::Greater => {
                    self.advance()?;
                    let rhs = self.parse_add_sub()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Gt, right: Box::new(rhs) };
                }
                Token::GreaterEq => {
                    self.advance()?;
                    let rhs = self.parse_add_sub()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Ge, right: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(node)
    }

    fn parse_add_sub(&mut self) -> LppoResult<Expr> {
        let mut node = self.parse_mul_div()?;
        loop {
            match self.lookahead {
                Token::Plus => {
                    self.advance()?;
                    let rhs = self.parse_mul_div()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Add, right: Box::new(rhs) };
                }
                Token::Minus => {
                    self.advance()?;
                    let rhs = self.parse_mul_div()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Sub, right: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(node)
    }

    fn parse_mul_div(&mut self) -> LppoResult<Expr> {
        let mut node = self.parse_power()?;
        loop {
            match self.lookahead {
                Token::Star => {
                    self.advance()?;
                    let rhs = self.parse_power()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Mul, right: Box::new(rhs) };
                }
                Token::Slash => {
                    self.advance()?;
                    let rhs = self.parse_power()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Div, right: Box::new(rhs) };
                }
                Token::Modulo => {
                    self.advance()?;
                    let rhs = self.parse_power()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Mod, right: Box::new(rhs) };
                }
                Token::Amp => {
                    self.advance()?;
                    let rhs = self.parse_power()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::BitAnd, right: Box::new(rhs) };
                }
                Token::Pipe => {
                    self.advance()?;
                    let rhs = self.parse_power()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::BitOr, right: Box::new(rhs) };
                }
                Token::Caret => {
                    self.advance()?;
                    let rhs = self.parse_power()?;
                    node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::BitXor, right: Box::new(rhs) };
                }
                _ => break,
            }
        }
        Ok(node)
    }

    fn parse_power(&mut self) -> LppoResult<Expr> {
        let mut node = self.parse_unary()?;
        if self.lookahead == Token::Power {
            self.advance()?;
            let rhs = self.parse_power()?; // right-associative
            node = Expr::BinaryOp { left: Box::new(node), op: BinaryOp::Pow, right: Box::new(rhs) };
        }
        Ok(node)
    }

    fn parse_unary(&mut self) -> LppoResult<Expr> {
        match self.lookahead.clone() {
            Token::Minus => {
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Neg, operand: Box::new(operand) })
            }
            Token::Not => {
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::Not, operand: Box::new(operand) })
            }
            Token::Tilde => {
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp { op: UnaryOp::BitNot, operand: Box::new(operand) })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> LppoResult<Expr> {
        let mut node = self.parse_primary()?;
        loop {
            match self.lookahead {
                Token::LBracket => {
                    self.advance()?;
                    let index = self.parse_expression()?;
                    self.expect(Token::RBracket)?;
                    node = Expr::Index { target: Box::new(node), index: Box::new(index) };
                }
                Token::Dot => {
                    self.advance()?;
                    let name = match self.lookahead.clone() {
                        Token::Identifier(n) => {
                            self.advance()?;
                            n
                        }
                        _ => {
                            return Err(LppoError::UnexpectedToken {
                                expected: "field or method name",
                                position: self.position,
                            })
                        }
                    };
                    // check if it's a method call
                    if self.lookahead == Token::LParen {
                        self.advance()?;
                        let mut args = Vec::new();
                        if self.lookahead != Token::RParen {
                            loop {
                                args.push(self.parse_expression()?);
                                if self.lookahead == Token::Comma {
                                    self.advance()?;
                                    continue;
                                }
                                break;
                            }
                        }
                        self.expect(Token::RParen)?;
                        node = Expr::MethodCall { target: Box::new(node), method: name, args };
                    } else {
                        node = Expr::FieldAccess { target: Box::new(node), field: name };
                    }
                }
                _ => break,
            }
        }
        Ok(node)
    }

    fn parse_primary(&mut self) -> LppoResult<Expr> {
        match self.lookahead.clone() {
            Token::Number(n) => { 
                self.advance()?; 
                Ok(Expr::Int(n)) 
            }
            Token::StringLiteral(s) => { 
                self.advance()?; 
                Ok(Expr::Str(s)) 
            }
            Token::Identifier(name) if name == "true" => { 
                self.advance()?; 
                Ok(Expr::Bool(true)) 
            }
            Token::Identifier(name) if name == "false" => { 
                self.advance()?; 
                Ok(Expr::Bool(false)) 
            }
            Token::Identifier(name) => { 
                self.advance()?; 
                Ok(Expr::Var(name)) 
            }
            Token::LParen => { 
                self.advance()?; 
                let e = self.parse_expression()?; 
                self.expect(Token::RParen)?; 
                Ok(e) 
            }
            Token::LBracket => {
                self.advance()?;
                let mut elements = Vec::new();
                if self.lookahead != Token::RBracket {
                    loop {
                        elements.push(self.parse_expression()?);
                        if self.lookahead == Token::Comma {
                            self.advance()?;
                            continue;
                        }
                        break;
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(elements))
            }
            _ => Err(LppoError::UnexpectedToken { expected: "expression", position: self.position }),
        }
    }

    fn parse_call(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Call)?;
        let name = match self.lookahead.clone() {
            Token::Identifier(name) => {
                self.advance()?;
                name
            }
            _ => {
                return Err(LppoError::UnexpectedToken {
                    expected: "procedure name",
                    position: self.position,
                })
            }
        };

        // optional argument list: call name(arg, ...);
        let mut args = Vec::new();
        if self.lookahead == Token::LParen {
            self.advance()?; // consume '('
            if self.lookahead != Token::RParen {
                loop {
                    args.push(self.parse_expression()?);
                    if self.lookahead == Token::Comma {
                        self.advance()?;
                        continue;
                    }
                    break;
                }
            }
            self.expect(Token::RParen)?;
        }

        self.expect(Token::Semicolon)?;
        Ok(Statement::Call(name, args))
    }

    fn parse_parallel(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Parallel)?;
        self.expect(Token::LBrace)?;
        let mut branches = Vec::new();
        while self.lookahead != Token::RBrace {
            match self.lookahead.clone() {
                Token::Out => {
                    if let Statement::Emit(exprs) = self.parse_emit()? {
                        // each expression becomes its own parallel branch
                        for e in exprs {
                            branches.push(e);
                        }
                    }
                }
                Token::Parallel => {
                    return Err(LppoError::UnexpectedToken {
                        expected: "out statement inside parallel block",
                        position: self.position,
                    });
                }
                Token::RBrace => break,
                _ => {
                    return Err(LppoError::UnexpectedToken {
                        expected: "out statement inside parallel block",
                        position: self.position,
                    });
                }
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::Parallel(branches))
    }

    fn expect(&mut self, expected: Token) -> LppoResult<()> {
        if self.lookahead == expected {
            self.advance()?;
            Ok(())
        } else {
            Err(LppoError::UnexpectedToken {
                expected: match expected {
                    Token::Proc => "keyword 'proc'",
                    Token::Parallel => "keyword 'parallel'",
                    Token::Out => "keyword 'out'",
                    Token::Call => "keyword 'call'",
                    Token::Identifier(_) => "identifier",
                    Token::StringLiteral(_) => "string literal",
                    Token::LBrace => "'{'",
                    Token::RBrace => "'}'",
                    Token::Semicolon => "';'",
                    Token::Eof => "end of file",
                    _ => "token",
                },
                position: self.position,
            })
        }
    }

    fn advance(&mut self) -> LppoResult<()> {
        self.lookahead = self.lexer.next_token()?;
        self.position = self.lexer.position();
        Ok(())
    }
}

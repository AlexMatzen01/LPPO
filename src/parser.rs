use crate::ast::{Procedure, Program, Statement, Expr, BinaryOp, UnaryOp, TypeAnnotation, Parameter, QualifiedName, MemoryLocation, OwnershipKind, Module, UseStatement};
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
        let mut uses = Vec::new();
        let mut modules = Vec::new();
        let mut procedures = Vec::new();
        
        while self.lookahead != Token::Eof {
            match &self.lookahead {
                Token::Use => {
                    uses.push(self.parse_use_statement()?);
                }
                Token::Module => {
                    modules.push(self.parse_module()?);
                }
                Token::Proc | Token::Pub | Token::Unsafe => {
                    procedures.push(self.parse_procedure()?);
                }
                _ => {
                    return Err(LppoError::unexpected_token("use, module or procedure", &format!("{:?}", self.lookahead), self.position,
                    ));
                }
            }
        }
        Ok(Program { 
            uses,
            modules, 
            procedures,
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
        })
    }
    
    fn parse_use_statement(&mut self) -> LppoResult<UseStatement> {
        self.expect(Token::Use)?;
        
        // Parse the path: std::collections::Vec or std::collections::{Vec, HashMap}
        let mut path = Vec::new();
        
        loop {
            let name = match self.lookahead.clone() {
                Token::Identifier(name) => name,
                // Allow type tokens as identifiers in use statements
                Token::TypeVec => "Vec".to_string(),
                Token::TypeHashMap => "HashMap".to_string(),
                Token::TypeHashSet => "HashSet".to_string(),
                Token::TypeBTreeMap => "BTreeMap".to_string(),
                Token::TypeBTreeSet => "BTreeSet".to_string(),
                Token::TypeOption => "Option".to_string(),
                Token::TypeResult => "Result".to_string(),
                Token::TypeBox => "Box".to_string(),
                Token::TypeRc => "Rc".to_string(),
                Token::TypeArc => "Arc".to_string(),
                _ => break,
            };
            
            path.push(name);
            self.advance()?;
            
            if self.lookahead == Token::DoubleColon {
                self.advance()?;
            } else {
                break;
            }
        }
        
        // Check for specific items import: use std::collections::{Vec, HashMap};
        let items = if self.lookahead == Token::LBrace {
            self.advance()?;
            let mut item_list = Vec::new();
            
            loop {
                let name = match self.lookahead.clone() {
                    Token::Identifier(name) => name,
                    // Allow type tokens as identifiers in use statements
                    Token::TypeVec => "Vec".to_string(),
                    Token::TypeHashMap => "HashMap".to_string(),
                    Token::TypeHashSet => "HashSet".to_string(),
                    Token::TypeBTreeMap => "BTreeMap".to_string(),
                    Token::TypeBTreeSet => "BTreeSet".to_string(),
                    Token::TypeOption => "Option".to_string(),
                    Token::TypeResult => "Result".to_string(),
                    Token::TypeBox => "Box".to_string(),
                    Token::TypeRc => "Rc".to_string(),
                    Token::TypeArc => "Arc".to_string(),
                    _ => break,
                };
                
                item_list.push(name);
                self.advance()?;
                
                if self.lookahead == Token::Comma {
                    self.advance()?;
                } else {
                    break;
                }
            }
            
            self.expect(Token::RBrace)?;
            Some(item_list)
        } else {
            None
        };
        
        // Expect semicolon at the end
        self.expect(Token::Semicolon)?;
        
        Ok(UseStatement { path, items })
    }
    
    fn parse_module(&mut self) -> LppoResult<Module> {
        self.expect(Token::Module)?;
        let name = match self.lookahead.clone() {
            Token::Identifier(name) => {
                self.advance()?;
                name
            }
            _ => {
                return Err(LppoError::unexpected_token("module name", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        self.expect(Token::LBrace)?;
        let mut procedures = Vec::new();
        while self.lookahead != Token::RBrace {
            procedures.push(self.parse_procedure()?);
        }
        self.expect(Token::RBrace)?;
        
        Ok(Module { 
            name, 
            procedures,
            structs: Vec::new(),
            enums: Vec::new(),
        })
    }

    fn parse_procedure(&mut self) -> LppoResult<Procedure> {
        // Check for pub and unsafe modifiers
        let is_public = if self.lookahead == Token::Pub {
            self.advance()?;
            true
        } else {
            false
        };
        
        let is_unsafe = if self.lookahead == Token::Unsafe {
            self.advance()?;
            true
        } else {
            false
        };
        
        self.expect(Token::Proc)?;
        let name = match self.lookahead.clone() {
            Token::Identifier(name) => {
                self.advance()?;
                name
            }
            _ => {
                return Err(LppoError::unexpected_token("procedure name", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        // Parse lifetime parameters <'a, 'b>
        let mut lifetimes = Vec::new();
        if self.lookahead == Token::Less {
            self.advance()?;
            loop {
                match self.lookahead.clone() {
                    Token::Lifetime(lt) => {
                        lifetimes.push(lt);
                        self.advance()?;
                    }
                    _ => {
                        return Err(LppoError::unexpected_token("lifetime annotation", &format!("{:?}", self.lookahead), self.position,
                        ))
                    }
                }
                if self.lookahead == Token::Comma {
                    self.advance()?;
                    continue;
                }
                break;
            }
            self.expect(Token::Greater)?;
        }
        
        // REQUIRED parameter list with EXPLICIT TYPES
        self.expect(Token::LParen)?;
        let mut params = Vec::new();
        if self.lookahead != Token::RParen {
            loop {
                let name = match self.lookahead.clone() {
                    Token::Identifier(name) => {
                        self.advance()?;
                        name
                    }
                    _ => {
                        return Err(LppoError::unexpected_token("parameter name", &format!("{:?}", self.lookahead), self.position,
                        ))
                    }
                };
                
                // REQUIRE colon and type
                self.expect(Token::Colon)?;
                let typ = self.parse_type()?;
                
                // Optional lifetime
                let lifetime = if let Token::Lifetime(lt) = &self.lookahead {
                    let lt = lt.clone();
                    self.advance()?;
                    Some(lt)
                } else {
                    None
                };
                
                params.push(Parameter { name, typ, lifetime });
                
                if self.lookahead == Token::Comma {
                    self.advance()?;
                    continue;
                }
                break;
            }
        }
        self.expect(Token::RParen)?;
        
        // REQUIRE return type annotation
        self.expect(Token::Arrow)?;
        let return_type = self.parse_type()?;

        self.expect(Token::LBrace)?;
        let mut statements = Vec::new();
        while self.lookahead != Token::RBrace {
            statements.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Procedure { 
            name, 
            params, 
            return_type,
            lifetimes,
            statements,
            is_public,
            is_unsafe,
        })
    }
    
    fn parse_type(&mut self) -> LppoResult<TypeAnnotation> {
        match &self.lookahead {
            Token::TypeInt64 => {
                self.advance()?;
                Ok(TypeAnnotation::Int64)
            }
            Token::TypeBool => {
                self.advance()?;
                Ok(TypeAnnotation::Bool)
            }
            Token::TypeStr => {
                self.advance()?;
                Ok(TypeAnnotation::Str)
            }
            Token::TypeVoid => {
                self.advance()?;
                Ok(TypeAnnotation::Void)
            }
            Token::TypeArr => {
                self.advance()?;
                self.expect(Token::Less)?;
                let inner = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(TypeAnnotation::Arr(Box::new(inner)))
            }
            Token::TypeOption => {
                self.advance()?;
                self.expect(Token::Less)?;
                let inner = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(TypeAnnotation::Option(Box::new(inner)))
            }
            Token::TypeResult => {
                self.advance()?;
                self.expect(Token::Less)?;
                let ok_type = self.parse_type()?;
                self.expect(Token::Comma)?;
                let err_type = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(TypeAnnotation::Result(Box::new(ok_type), Box::new(err_type)))
            }
            // V5.2 Generic container types - Vec, HashMap, etc.
            Token::TypeVec | Token::TypeHashMap | Token::TypeHashSet | 
            Token::TypeBTreeMap | Token::TypeBTreeSet | Token::TypeBox |
            Token::TypeRc | Token::TypeArc => {
                let type_name = match &self.lookahead {
                    Token::TypeVec => "Vec",
                    Token::TypeHashMap => "HashMap",
                    Token::TypeHashSet => "HashSet",
                    Token::TypeBTreeMap => "BTreeMap",
                    Token::TypeBTreeSet => "BTreeSet",
                    Token::TypeBox => "Box",
                    Token::TypeRc => "Rc",
                    Token::TypeArc => "Arc",
                    _ => unreachable!(),
                };
                self.advance()?;
                
                // Check for generic parameters
                if self.lookahead == Token::Less {
                    self.advance()?;
                    let mut type_params = Vec::new();
                    
                    loop {
                        type_params.push(self.parse_type()?);
                        
                        if self.lookahead == Token::Comma {
                            self.advance()?;
                        } else {
                            break;
                        }
                    }
                    
                    self.expect(Token::Greater)?;
                    Ok(TypeAnnotation::Generic(type_name.to_string(), type_params))
                } else {
                    Ok(TypeAnnotation::Custom(type_name.to_string()))
                }
            }
            Token::Ref => {
                self.advance()?;
                let lifetime = if let Token::Lifetime(lt) = &self.lookahead {
                    let lt = lt.clone();
                    self.advance()?;
                    Some(lt)
                } else {
                    None
                };
                self.expect(Token::Less)?;
                let inner = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(TypeAnnotation::Ref(Box::new(inner), lifetime))
            }
            Token::Mut => {
                self.advance()?;
                self.expect(Token::Less)?;
                let inner = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(TypeAnnotation::Mut(Box::new(inner)))
            }
            Token::Own => {
                self.advance()?;
                self.expect(Token::Less)?;
                let inner = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(TypeAnnotation::Own(Box::new(inner)))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance()?;
                
                // Check for generic parameters
                if self.lookahead == Token::Less {
                    self.advance()?;
                    let mut type_params = Vec::new();
                    
                    loop {
                        type_params.push(self.parse_type()?);
                        
                        if self.lookahead == Token::Comma {
                            self.advance()?;
                        } else {
                            break;
                        }
                    }
                    
                    self.expect(Token::Greater)?;
                    Ok(TypeAnnotation::Generic(name, type_params))
                } else {
                    Ok(TypeAnnotation::Custom(name))
                }
            }
            _ => {
                Err(LppoError::unexpected_token("type annotation", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        }
    }

    fn parse_statement(&mut self) -> LppoResult<Statement> {
        match self.lookahead.clone() {
            Token::Let => self.parse_let(),
            Token::Out => self.parse_emit(),
            Token::Input => self.parse_input(),
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
            Token::Alloc => self.parse_alloc(),
            Token::Dealloc => self.parse_dealloc(),
            Token::Move => self.parse_move(),
            Token::Copy => self.parse_copy(),
            Token::Scope => self.parse_scope(),
            Token::Unsafe => self.parse_unsafe_block(),
            Token::Eof | Token::RBrace => Err(LppoError::unexpected_eof("statement or closing brace")),
            _ => Err(LppoError::unexpected_token("statement", &format!("{:?}", self.lookahead), self.position,
            )),
        }
    }
    
    fn parse_alloc(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Alloc)?;
        self.expect(Token::Less)?;
        let typ = self.parse_type()?;
        self.expect(Token::Greater)?;
        
        let var = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("variable name", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        // Parse on heap or stack
        self.expect(Token::On)?;
        let location = match &self.lookahead {
            Token::Heap => {
                self.advance()?;
                MemoryLocation::Heap
            }
            Token::Stack => {
                self.advance()?;
                MemoryLocation::Stack
            }
            _ => {
                return Err(LppoError::unexpected_token("heap or stack", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        // Parse size expression
        self.expect(Token::LBracket)?;
        let size = self.parse_expression()?;
        self.expect(Token::RBracket)?;
        self.expect(Token::Semicolon)?;
        
        Ok(Statement::Alloc { var, typ, location, size })
    }
    
    fn parse_dealloc(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Dealloc)?;
        let var = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("variable name", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        self.expect(Token::Semicolon)?;
        Ok(Statement::Dealloc(var))
    }
    
    fn parse_move(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Move)?;
        let source = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("source variable", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        self.expect(Token::Arrow)?; // ->
        let target = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("target variable", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        self.expect(Token::Semicolon)?;
        Ok(Statement::Move { source, target })
    }
    
    fn parse_copy(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Copy)?;
        let source = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("source variable", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        self.expect(Token::Arrow)?;
        let target = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("target variable", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        self.expect(Token::Semicolon)?;
        Ok(Statement::Copy { source, target })
    }
    
    fn parse_scope(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Scope)?;
        let name = if let Token::Identifier(n) = &self.lookahead {
            let n = n.clone();
            self.advance()?;
            Some(n)
        } else {
            None
        };
        
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.lookahead != Token::RBrace {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        
        Ok(Statement::Scope { name, body })
    }
    
    fn parse_unsafe_block(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Unsafe)?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.lookahead != Token::RBrace {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::Unsafe { body })
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
                return Err(LppoError::unexpected_token("variable name", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        // REQUIRE explicit type for loop variable
        self.expect(Token::Colon)?;
        let typ = self.parse_type()?;
        
        self.expect(Token::In)?;
        let iterable = self.parse_expression()?;
        self.expect(Token::LBrace)?;
        let mut body = Vec::new();
        while self.lookahead != Token::RBrace {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RBrace)?;
        Ok(Statement::For { var, typ, iterable, body })
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

    fn parse_input(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Input)?;
        
        // Get variable name
        let var = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("variable name", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        // Expect colon and type
        self.expect(Token::Colon)?;
        let typ = self.parse_type()?;
        
        // Optional prompt with "with" keyword
        let prompt = if let Token::Identifier(id) = &self.lookahead {
            if id == "with" {
                self.advance()?; // consume "with"
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };
        
        self.expect(Token::Semicolon)?;
        Ok(Statement::Input { var, typ, prompt })
    }

    fn parse_let(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Let)?;
        
        // Check for mut keyword
        let is_mut = if self.lookahead == Token::Mut {
            self.advance()?;
            true
        } else {
            false
        };
        
        // Check for ownership qualifier (own, ref, borrow)
        let ownership = match &self.lookahead {
            Token::Own => {
                self.advance()?;
                OwnershipKind::Own
            }
            Token::Ref => {
                self.advance()?;
                OwnershipKind::Ref
            }
            Token::Borrow => {
                self.advance()?;
                OwnershipKind::Borrow
            }
            _ => OwnershipKind::Own, // default
        };
        
        let name = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("identifier", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        // REQUIRE explicit type annotation
        self.expect(Token::Colon)?;
        let typ = self.parse_type()?;
        
        // expect an equals token
        self.expect(Token::Equals)?;
        let expr = self.parse_expression()?;
        self.expect(Token::Semicolon)?;
        Ok(Statement::Let { name, typ, expr, is_mut, ownership })
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
            Token::Ref => {
                self.advance()?;
                let is_mut = if self.lookahead == Token::Mut {
                    self.advance()?;
                    true
                } else {
                    false
                };
                let lifetime = if let Token::Lifetime(lt) = &self.lookahead {
                    let lt = lt.clone();
                    self.advance()?;
                    Some(lt)
                } else {
                    None
                };
                let target = self.parse_unary()?;
                Ok(Expr::Ref {
                    target: Box::new(target),
                    lifetime,
                    is_mut,
                })
            }
            Token::Deref => {
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expr::Deref(Box::new(operand)))
            }
            Token::Borrow => {
                self.advance()?;
                let lifetime = if let Token::Lifetime(lt) = &self.lookahead {
                    let lt = lt.clone();
                    self.advance()?;
                    Some(lt)
                } else {
                    None
                };
                let target = self.parse_unary()?;
                Ok(Expr::Borrow {
                    target: Box::new(target),
                    lifetime,
                })
            }
            Token::SizeOf => {
                self.advance()?;
                self.expect(Token::Less)?;
                let typ = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(Expr::SizeOf(typ))
            }
            Token::AlignOf => {
                self.advance()?;
                self.expect(Token::Less)?;
                let typ = self.parse_type()?;
                self.expect(Token::Greater)?;
                Ok(Expr::AlignOf(typ))
            }
            Token::Cast => {
                self.advance()?;
                self.expect(Token::Less)?;
                let to_type = self.parse_type()?;
                self.expect(Token::Greater)?;
                self.expect(Token::LParen)?;
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Cast {
                    expr: Box::new(expr),
                    to_type,
                })
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
                            return Err(LppoError::unexpected_token("field or method name", &format!("{:?}", self.lookahead), self.position,
                            ))
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
            Token::Identifier(_) => {
                // Check if this is a qualified name
                let qualified = self.parse_qualified_name()?;
                
                // If it's just a simple name, return Var, otherwise QualifiedVar
                if qualified.path.is_empty() {
                    Ok(Expr::Var(qualified.name))
                } else {
                    Ok(Expr::QualifiedVar(qualified))
                }
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
            _ => Err(LppoError::unexpected_token("expression", &format!("{:?}", self.lookahead), self.position )),
        }
    }

    fn parse_call(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Call)?;
        
        // Parse qualified name (module::submodule::procedure)
        let qualified_name = self.parse_qualified_name()?;

        // REQUIRE argument list with parentheses
        self.expect(Token::LParen)?;
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

        self.expect(Token::Semicolon)?;
        Ok(Statement::Call(qualified_name, args))
    }
    
    fn parse_qualified_name(&mut self) -> LppoResult<QualifiedName> {
        let mut path = Vec::new();
        let mut name = match self.lookahead.clone() {
            Token::Identifier(n) => {
                self.advance()?;
                n
            }
            _ => {
                return Err(LppoError::unexpected_token("identifier", &format!("{:?}", self.lookahead), self.position,
                ))
            }
        };
        
        // Parse qualified path separators
        while self.lookahead == Token::DoubleColon {
            self.advance()?;
            path.push(name);
            name = match self.lookahead.clone() {
                Token::Identifier(n) => {
                    self.advance()?;
                    n
                }
                _ => {
                    return Err(LppoError::unexpected_token("identifier after ::", &format!("{:?}", self.lookahead), self.position,
                    ))
                }
            };
        }
        
        Ok(QualifiedName { path, name })
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
                    return Err(LppoError::unexpected_token("out statement inside parallel block", &format!("{:?}", self.lookahead), self.position,
                    ));
                }
                Token::RBrace => break,
                _ => {
                    return Err(LppoError::unexpected_token("out statement inside parallel block", &format!("{:?}", self.lookahead), self.position,
                    ));
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
            let expected_str = match expected {
                Token::Proc => "keyword 'proc'".to_string(),
                Token::Parallel => "keyword 'parallel'".to_string(),
                Token::Out => "keyword 'out'".to_string(),
                Token::Call => "keyword 'call'".to_string(),
                Token::Identifier(_) => "identifier".to_string(),
                Token::StringLiteral(_) => "string literal".to_string(),
                Token::LBrace => "'{'".to_string(),
                Token::RBrace => "'}'".to_string(),
                Token::Semicolon => "';'".to_string(),
                Token::Eof => "end of file".to_string(),
                _ => "token".to_string(),
            };
            let found_str = format!("{:?}", self.lookahead);
            Err(LppoError::unexpected_token(&expected_str, &found_str, self.position))
        }
    }

    fn advance(&mut self) -> LppoResult<()> {
        self.lookahead = self.lexer.next_token()?;
        self.position = self.lexer.position();
        Ok(())
    }
}

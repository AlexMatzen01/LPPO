use crate::ast::{Procedure, Program, Statement, Expr, BinaryOp, UnaryOp, TypeAnnotation, Parameter, QualifiedName, MemoryLocation, OwnershipKind, Module, UseDeclaration, UseItems, TypeAlias};
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
    position: crate::error::SourceLocation,
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
        let mut use_declarations = Vec::new();
        let mut type_aliases = Vec::new();
        let mut modules = Vec::new();
        let mut procedures = Vec::new();
        
        while self.lookahead != Token::Eof {
            match &self.lookahead {
                Token::Use => {
                    use_declarations.push(self.parse_use_declaration()?);
                }
                Token::TypeDef => {
                    type_aliases.push(self.parse_type_alias()?);
                }
                Token::Module => {
                    modules.push(self.parse_module()?);
                }
                Token::Proc | Token::Pub | Token::Unsafe => {
                    procedures.push(self.parse_procedure()?);
                }
                _ => {
                    return Err(LppoError::unexpected_token_at("use, type, module or procedure", &format!("{:?}", self.lookahead), self.position.clone()));
                }
            }
        }
        Ok(Program { 
            use_declarations,
            type_aliases,
            modules, 
            procedures,
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
        })
    }
    
    fn parse_module(&mut self) -> LppoResult<Module> {
        self.expect(Token::Module)?;
        let name = match self.lookahead.clone() {
            Token::Identifier(name) => {
                self.advance()?;
                name
            }
            _ => {
                return Err(LppoError::unexpected_token_at("module name", &format!("{:?}", self.lookahead), self.position.clone()))
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
    
    fn parse_use_declaration(&mut self) -> LppoResult<UseDeclaration> {
        self.expect(Token::Use)?;
        
        // Parse the path and items: std::collections::{Vec, HashMap}
        let mut path = Vec::new();
        loop {
            if let Some(name) = self.token_as_identifier(&self.lookahead.clone()) {
                path.push(name);
                self.advance()?;
                } else {
                return Err(LppoError::unexpected_token_at("module path", &format!("{:?}", self.lookahead), self.position.clone()));
            }
            
            if self.lookahead == Token::DoubleColon {
                self.advance()?;
                // Check if next is * or { - if so, we're at the import list
                if self.lookahead == Token::Star || self.lookahead == Token::LBrace {
                    break;
                }
                // Otherwise continue parsing the path
            } else {
                break;
            }
        }
        
        // Now parse the items to import
        let items = if self.lookahead == Token::Star {
            self.advance()?;
            UseItems::All
        } else if self.lookahead == Token::LBrace {
            self.advance()?;
            let mut items = Vec::new();
            loop {
                if let Some(name) = self.token_as_identifier(&self.lookahead.clone()) {
                    items.push(name);
                    self.advance()?;
                } else {
                    return Err(LppoError::unexpected_token_at("item name", &format!("{:?}", self.lookahead), self.position.clone()));
                }
                
                if self.lookahead == Token::Comma {
                    self.advance()?;
                } else {
                    break;
                }
            }
            self.expect(Token::RBrace)?;
            UseItems::Items(items)
        } else {
            // Single item - import the last element in path
            let last = path.pop().ok_or_else(|| LppoError::unexpected_token_at("module name", "empty path", self.position.clone()))?;
            UseItems::Single(last)
        };
        
        self.expect(Token::Semicolon)?;
        
        Ok(UseDeclaration { path, items })
    }
    
    fn parse_type_alias(&mut self) -> LppoResult<TypeAlias> {
        // Check for pub modifier
        let is_public = if self.lookahead == Token::Pub {
            self.advance()?;
            true
        } else {
            false
        };
        
        self.expect(Token::TypeDef)?;
        
        let name = match self.lookahead.clone() {
            Token::Identifier(name) => {
                self.advance()?;
                name
            }
            _ => {
                return Err(LppoError::unexpected_token_at("type alias name", &format!("{:?}", self.lookahead), self.position.clone()));
            }
        };
        
        // Parse generic parameters if present: <T, U>
        let mut generic_params = Vec::new();
        if self.lookahead == Token::Less {
            self.advance()?;
            loop {
                match self.lookahead.clone() {
                    Token::Identifier(param) => {
                        generic_params.push(param);
                        self.advance()?;
                    }
                    _ => {
                        return Err(LppoError::unexpected_token_at("generic parameter", &format!("{:?}", self.lookahead), self.position.clone()));
                    }
                }
                
                if self.lookahead == Token::Comma {
                    self.advance()?;
                } else {
                    break;
                }
            }
            self.expect(Token::Greater)?;
        }
        
        self.expect(Token::Equals)?;
        
        let typ = self.parse_type()?;
        
        self.expect(Token::Semicolon)?;
        
        Ok(TypeAlias {
            name,
            typ,
            is_public,
            generic_params,
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
        let name = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("procedure name", &format!("{:?}", self.lookahead), self.position.clone()));
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
                            return Err(LppoError::unexpected_token_at("lifetime annotation", &format!("{:?}", self.lookahead), self.position.clone()));
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
        
        // Parameter list - support both old and new syntax
        let mut params = Vec::new();
        if self.lookahead == Token::LParen {
            self.advance()?;  // consume '('
            if self.lookahead != Token::RParen {
                loop {
                    let name = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
                        self.advance()?;
                        n
                    } else {
                        return Err(LppoError::unexpected_token_at("parameter name", &format!("{:?}", self.lookahead), self.position.clone()));
                    };
                    
                    // Optional colon and type (for old syntax compatibility)
                    let typ = if self.lookahead == Token::Colon {
                        self.advance()?;
                        self.parse_type()?
                    } else {
                        // Default to Void for old syntax without types
                        TypeAnnotation::Void
                    };
                    
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
        }
        
        // Optional return type annotation (for old syntax compatibility)
        let return_type = if self.lookahead == Token::Arrow {
            self.advance()?;
            self.parse_type()?
        } else {
            // Default to Void for old syntax
            TypeAnnotation::Void
        };

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
            // Handle generic type tokens like Vec, HashMap, etc.
            tok if self.token_as_identifier(tok).is_some() => {
                let type_name = self.token_as_identifier(tok).unwrap();
                self.advance()?;
                
                // Check for generic parameters: <T, U, ...>
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
                    Ok(TypeAnnotation::Generic(type_name, type_params))
                } else {
                    // Simple custom type (might be a type alias)
                    Ok(TypeAnnotation::Custom(type_name))
                }
            }
            _ => Err(LppoError::unexpected_token_at("type annotation", &format!("{:?}", self.lookahead), self.position.clone())),
        }
    }
    
    fn parse_match_arm(&mut self) -> LppoResult<crate::ast::MatchArm> {
        use crate::ast::MatchArm;
        
        // Parse pattern
        let pattern = self.parse_pattern()?;
        
        // Check for guard: if condition
        let guard = if self.lookahead == Token::If {
            self.advance()?;
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        // Expect =>
        self.expect(Token::FatArrow)?;
        
        // Parse body expression or special statement-like expressions
        let body = if self.lookahead == Token::Out {
            // Handle "out ..." as a special expression
            self.advance()?; // consume 'out'
            
            // Parse expressions to output
            let mut exprs = Vec::new();
            loop {
                exprs.push(self.parse_expression()?);
                if self.lookahead == Token::Comma && exprs.len() < 10 {
                    // Check if this is the arm separator or part of the out args
                    // Peek ahead - if next is a pattern start, break
                    self.advance()?;
                    
                    // If we see tokens that start a pattern (identifiers, Some, None, etc.), break
                    match self.lookahead {
                        Token::Identifier(_) | Token::Some | Token::None | Token::Underscore | Token::RBrace => {
                            // This comma is the arm separator, not part of out
                            break;
                        }
                        _ => {
                            // Continue parsing out arguments
                            continue;
                        }
                    }
                } else {
                    break;
                }
            }
            
            // Create a placeholder expression that represents the out
            // For now, we'll use the first expression or Int(0)
            if exprs.is_empty() {
                Expr::Int(0)
            } else {
                exprs.into_iter().next().unwrap()
            }
        } else {
            self.parse_expression()?
        };
        
        Ok(MatchArm {
            pattern,
            guard,
            body,
        })
    }
    
    fn parse_pattern(&mut self) -> LppoResult<crate::ast::Pattern> {
        use crate::ast::Pattern;
        
        match self.lookahead.clone() {
            Token::Underscore => {
                self.advance()?;
                Ok(Pattern::Wildcard)
            }
            Token::Number(n) => {
                self.advance()?;
                Ok(Pattern::Literal(Expr::Int(n)))
            }
            Token::StringLiteral(s) => {
                self.advance()?;
                Ok(Pattern::Literal(Expr::Str(s)))
            }
            Token::Identifier(name) if name == "true" => {
                self.advance()?;
                Ok(Pattern::Literal(Expr::Bool(true)))
            }
            Token::Identifier(name) if name == "false" => {
                self.advance()?;
                Ok(Pattern::Literal(Expr::Bool(false)))
            }
            Token::Identifier(name) => {
                // Could be a variable binding or enum variant
                self.advance()?;
                
                // Check if it's an enum variant (followed by optional fields)
                if self.lookahead == Token::LParen {
                    // Enum variant with fields: Some(x)
                    self.advance()?;
                    let mut fields = Vec::new();
                    if self.lookahead != Token::RParen {
                        loop {
                            fields.push(self.parse_pattern()?);
                            if self.lookahead == Token::Comma {
                                self.advance()?;
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Pattern::EnumVariant {
                        enum_name: String::new(), // Inferred from context
                        variant: name,
                        fields,
                    })
                } else {
                    // Simple variable binding
                    Ok(Pattern::Variable(name))
                }
            }
            tok if self.token_as_identifier(&tok).is_some() => {
                // Handle type tokens as identifiers (e.g., Some, None, Ok, Err)
                let name = self.token_as_identifier(&tok).unwrap();
                self.advance()?;
                
                if self.lookahead == Token::LParen {
                    // Enum variant with fields
                    self.advance()?;
                    let mut fields = Vec::new();
                    if self.lookahead != Token::RParen {
                        loop {
                            fields.push(self.parse_pattern()?);
                            if self.lookahead == Token::Comma {
                                self.advance()?;
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Pattern::EnumVariant {
                        enum_name: String::new(),
                        variant: name,
                        fields,
                    })
                } else {
                    Ok(Pattern::Variable(name))
                }
            }
            _ => {
                Err(LppoError::unexpected_token_at("pattern", &format!("{:?}", self.lookahead), self.position.clone()))
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
            _ => {
                // Try to parse as expression statement
                let expr = self.parse_expression()?;
                self.expect(Token::Semicolon)?;
                Ok(Statement::ExprStatement(expr))
            }
        }
    }
    
    fn parse_alloc(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Alloc)?;
        self.expect(Token::Less)?;
        let typ = self.parse_type()?;
        self.expect(Token::Greater)?;
        
        let var = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("variable name", &format!("{:?}", self.lookahead), self.position.clone()));
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
                return Err(LppoError::unexpected_token_at("heap or stack", &format!("{:?}", self.lookahead), self.position.clone()));
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
        let var = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("variable name", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        self.expect(Token::Semicolon)?;
        Ok(Statement::Dealloc(var))
    }
    
    fn parse_move(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Move)?;
        let source = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("source variable", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        self.expect(Token::Arrow)?; // ->
        let target = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("target variable", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        self.expect(Token::Semicolon)?;
        Ok(Statement::Move { source, target })
    }
    
    fn parse_copy(&mut self) -> LppoResult<Statement> {
        self.expect(Token::Copy)?;
        let source = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("source variable", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        self.expect(Token::Arrow)?;
        let target = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("target variable", &format!("{:?}", self.lookahead), self.position.clone()));
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
        let var = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("variable name", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        
        // Optional explicit type for loop variable (for old syntax compatibility)
        let typ = if self.lookahead == Token::Colon {
            self.advance()?;
            self.parse_type()?
        } else {
            // Default to Void - will be inferred at runtime
            TypeAnnotation::Void
        };
        
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
        let var = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("variable name", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        
        // Expect colon and type
        self.expect(Token::Colon)?;
        let typ = self.parse_type()?;
        
        // Optional prompt with "with" keyword
        let prompt = if self.lookahead == Token::With {
            self.advance()?; // consume "with"
            Some(self.parse_expression()?)
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
        
        let name = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("identifier", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        
        // Optional explicit type annotation (for old syntax compatibility)
        let typ = if self.lookahead == Token::Colon {
            self.advance()?;
            self.parse_type()?
        } else {
            // Default to Void for old syntax - will be inferred at runtime
            TypeAnnotation::Void
        };
        
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
                        Token::Insert => {
                            self.advance()?;
                            "insert".to_string()
                        }
                        _ if self.token_as_identifier(&self.lookahead.clone()).is_some() => {
                            let name = self.token_as_identifier(&self.lookahead.clone()).unwrap();
                            self.advance()?;
                            name
                        }
                        _ => {
                            return Err(LppoError::unexpected_token_at("field or method name", &format!("{:?}", self.lookahead), self.position.clone()));
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
                Token::LParen => {
                    // Function call: func(args) or Type::new()
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
                    node = Expr::Call {
                        func: Box::new(node),
                        args,
                    };
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
                let saved_name = name.clone();
                self.advance()?; // consume identifier
                
                // Check if this is a macro call: identifier![...]
                if saved_name == "vec" && self.lookahead == Token::Bang {
                    self.advance()?; // consume '!'
                    self.expect(Token::LBracket)?;
                    
                    // Parse array elements
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
                    // Return array - runtime will treat it as Vec
                    return Ok(Expr::Array(elements));
                }
                
                // Check if this is a qualified name (we already consumed the first identifier)
                // We need to back up and reparse
                // For simplicity, handle common cases
                if self.lookahead == Token::DoubleColon {
                    // It's a qualified name, need to parse rest of path
                    let mut path = Vec::new();
                    let mut name = saved_name;
                    
                    while self.lookahead == Token::DoubleColon {
                        path.push(name);
                        self.advance()?; // consume ::
                        
                        name = match self.lookahead.clone() {
                            Token::Identifier(n) => {
                                self.advance()?;
                                n
                            }
                            _ => {
                                return Err(LppoError::unexpected_token_at("identifier after ::", &format!("{:?}", self.lookahead), self.position.clone()));
                            }
                        };
                    }
                    
                    if path.is_empty() {
                        Ok(Expr::Var(name))
                    } else {
                        Ok(Expr::QualifiedVar(QualifiedName { path, name }))
                    }
                } else {
                    // Simple identifier
                    Ok(Expr::Var(saved_name))
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
            // Handle type tokens as identifiers in expression context (e.g., HashMap::new())
            tok if self.token_as_identifier(&tok).is_some() => {
                let saved_name = self.token_as_identifier(&tok).unwrap();
                self.advance()?;
                
                // Check if this is a qualified name with ::
                if self.lookahead == Token::DoubleColon {
                    let mut path = Vec::new();
                    let mut name = saved_name;
                    
                    while self.lookahead == Token::DoubleColon {
                        path.push(name);
                        self.advance()?; // consume ::
                        
                        name = match self.lookahead.clone() {
                            Token::Identifier(n) => {
                                self.advance()?;
                                n
                            }
                            Token::New => {
                                self.advance()?;
                                "new".to_string()
                            }
                            _ => {
                                return Err(LppoError::unexpected_token_at("identifier after ::", &format!("{:?}", self.lookahead), self.position.clone()));
                            }
                        };
                    }
                    
                    if path.is_empty() {
                        Ok(Expr::Var(name))
                    } else {
                        Ok(Expr::QualifiedVar(QualifiedName { path, name }))
                    }
                } else {
                    // Simple identifier
                    Ok(Expr::Var(saved_name))
                }
            }
            Token::Pipe => {
                // Closure: |params| -> type { body } or |params| expr
                self.advance()?; // consume opening |
                
                let mut params = Vec::new();
                if self.lookahead != Token::Pipe {
                    loop {
                        let name = match self.lookahead.clone() {
                            Token::Identifier(n) => {
                                self.advance()?;
                                n
                            }
                            _ => {
                                return Err(LppoError::unexpected_token_at("parameter name", &format!("{:?}", self.lookahead), self.position.clone()));
                            }
                        };
                        
                        // Expect type annotation: x: Int64
                        self.expect(Token::Colon)?;
                        let typ = self.parse_type()?;
                        
                        params.push(Parameter {
                            name,
                            typ,
                            lifetime: None,
                        });
                        
                        if self.lookahead == Token::Comma {
                            self.advance()?;
                        } else {
                            break;
                        }
                    }
                }
                
                self.expect(Token::Pipe)?; // closing |
                
                // Check for optional return type: -> Type
                let return_type = if self.lookahead == Token::Arrow {
                    self.advance()?;
                    Some(self.parse_type()?)
                } else {
                    None
                };
                
                // Parse body: either { statements } or expression
                let body = if self.lookahead == Token::LBrace {
                    self.advance()?;
                    // For now, parse as a single return expression
                    // A full implementation would parse statements
                    if self.lookahead == Token::Return {
                        self.advance()?;
                        let expr = self.parse_expression()?;
                        self.expect(Token::Semicolon)?;
                        self.expect(Token::RBrace)?;
                        Box::new(expr)
                    } else {
                        // Try to parse as expression
                        let expr = self.parse_expression()?;
                        if self.lookahead == Token::Semicolon {
                            self.advance()?;
                        }
                        self.expect(Token::RBrace)?;
                        Box::new(expr)
                    }
                } else {
                    // Simple expression body
                    Box::new(self.parse_expression()?)
                };
                
                Ok(Expr::Closure {
                    params,
                    return_type,
                    body,
                })
            }
            Token::Match => {
                // Match expression: match expr { arms }
                self.advance()?; // consume 'match'
                let expr = Box::new(self.parse_expression()?);
                self.expect(Token::LBrace)?;
                
                let mut arms = Vec::new();
                while self.lookahead != Token::RBrace {
                    let arm = self.parse_match_arm()?;
                    arms.push(arm);
                    
                    // Match arms are separated by commas
                    if self.lookahead == Token::Comma {
                        self.advance()?;
                    }
                }
                
                self.expect(Token::RBrace)?;
                
                Ok(Expr::MatchExpr {
                    expr,
                    arms,
                })
            }
            _ => Err(LppoError::unexpected_token_at("expression", &format!("{:?}", self.lookahead), self.position.clone())),
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
        let mut name = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
            self.advance()?;
            n
        } else {
            return Err(LppoError::unexpected_token_at("identifier", &format!("{:?}", self.lookahead), self.position.clone()));
        };
        
        // Parse qualified path separators
        while self.lookahead == Token::DoubleColon {
            self.advance()?;
            path.push(name);
            name = if let Some(n) = self.token_as_identifier(&self.lookahead.clone()) {
                self.advance()?;
                n
            } else {
                return Err(LppoError::unexpected_token_at("identifier after ::", &format!("{:?}", self.lookahead), self.position.clone()));
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
                    return Err(LppoError::unexpected_token_at("out statement inside parallel block", &format!("{:?}", self.lookahead), self.position.clone()));
                }
                Token::RBrace => break,
                _ => {
                    return Err(LppoError::unexpected_token_at("out statement inside parallel block", &format!("{:?}", self.lookahead), self.position.clone()));
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
            Err(LppoError::unexpected_token_at(&expected_str, &found_str, self.position.clone()))
        }
    }

    fn advance(&mut self) -> LppoResult<()> {
        self.lookahead = self.lexer.next_token()?;
        self.position = self.lexer.position();
        Ok(())
    }
    
    /// Extract an identifier name from tokens that can act as identifiers (including type tokens)
    fn token_as_identifier(&self, tok: &Token) -> Option<String> {
        match tok {
            Token::Identifier(name) => Some(name.clone()),
            // Type tokens that can be used as identifiers in use paths
            Token::TypeVec => Some("Vec".to_string()),
            Token::TypeHashMap => Some("HashMap".to_string()),
            Token::TypeHashSet => Some("HashSet".to_string()),
            Token::TypeLinkedList => Some("LinkedList".to_string()),
            Token::TypeBTreeMap => Some("BTreeMap".to_string()),
            Token::TypeBTreeSet => Some("BTreeSet".to_string()),
            Token::TypeBox => Some("Box".to_string()),
            Token::TypeRc => Some("Rc".to_string()),
            Token::TypeArc => Some("Arc".to_string()),
            Token::TypeOption => Some("Option".to_string()),
            Token::TypeResult => Some("Result".to_string()),
            Token::TypeString => Some("String".to_string()),
            Token::TypeFile => Some("File".to_string()),
            Token::TypeBufReader => Some("BufReader".to_string()),
            Token::TypeBufWriter => Some("BufWriter".to_string()),
            Token::TypeTcpStream => Some("TcpStream".to_string()),
            Token::TypeUdpSocket => Some("UdpSocket".to_string()),
            Token::TypeMutex => Some("Mutex".to_string()),
            Token::TypeDuration => Some("Duration".to_string()),
            Token::TypeInstant => Some("Instant".to_string()),
            Token::TypeDateTime => Some("DateTime".to_string()),
            Token::TypeFuture => Some("Future".to_string()),
            Token::TypeTask => Some("Task".to_string()),
            Token::TypeAtomic => Some("Atomic".to_string()),
            Token::Async => Some("async".to_string()),
            Token::Await => Some("await".to_string()),
            // Enum variant tokens for pattern matching
            Token::Some => Some("Some".to_string()),
            Token::None => Some("None".to_string()),
            Token::Ok => Some("Ok".to_string()),
            Token::Err => Some("Err".to_string()),
            // Common tokens that are over-reserved but should work as identifiers
            Token::Json => Some("json".to_string()),
            Token::Csv => Some("csv".to_string()),
            Token::Xml => Some("xml".to_string()),
            Token::Yaml => Some("yaml".to_string()),
            Token::Toml => Some("toml".to_string()),
            Token::Base => Some("base".to_string()),
            Token::Test => Some("test".to_string()),
            Token::Info => Some("info".to_string()),
            Token::Warning => Some("warning".to_string()),
            Token::Error => Some("error".to_string()),
            Token::Log => Some("log".to_string()),
            Token::Hash => Some("hash".to_string()),
            Token::Random => Some("random".to_string()),
            Token::Binary => Some("binary".to_string()),
            Token::Hex => Some("hex".to_string()),
            Token::Base64 => Some("base64".to_string()),
            Token::Final => Some("final".to_string()),
            Token::Virtual => Some("virtual".to_string()),
            Token::Abstract => Some("abstract".to_string()),
            Token::Partial => Some("partial".to_string()),
            Token::Sealed => Some("sealed".to_string()),
            Token::Property => Some("property".to_string()),
            Token::Extension => Some("extension".to_string()),
            Token::Constraint => Some("constraint".to_string()),
            Token::Where => Some("where".to_string()),
            Token::Simd => Some("simd".to_string()),
            Token::Gpu => Some("gpu".to_string()),
            Token::Jit => Some("jit".to_string()),
            Token::Kernel => Some("kernel".to_string()),
            Token::Grid => Some("grid".to_string()),
            Token::Block => Some("block".to_string()),
            Token::Device => Some("device".to_string()),
            Token::Thread => Some("thread".to_string()),
            Token::AssertEq => Some("assert_eq".to_string()),
            Token::AssertNe => Some("assert_ne".to_string()),
            Token::Assert => Some("assert".to_string()),
            Token::Panic => Some("panic".to_string()),
            Token::Debug => Some("debug".to_string()),
            Token::Print => Some("print".to_string()),
            Token::Println => Some("println".to_string()),
            Token::Env => Some("env".to_string()),
            Token::File => Some("file".to_string()),
            Token::Alloc => Some("alloc".to_string()),
            Token::Range => Some("range".to_string()),
            Token::Reduce => Some("reduce".to_string()),
            // Allow most soft keywords to be used as identifiers
            _ => None,
        }
    }
}

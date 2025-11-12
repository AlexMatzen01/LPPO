use crate::error::{LppoError, LppoResult};

/// Represents the lexical units understood by the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Proc,
    Parallel,
    Out,
    Call,
    Let,
        If,
        Elif,
        Else,
        While,
        For,
        In,
        Loop,
        Break,
        Continue,
        Return,
        Struct,
        New,
        Dot,
        Equals,
        Comma,
        EqEq,
        NotEq,
        Less,
        LessEq,
        Greater,
        GreaterEq,
    And,
    Or,
    Not,
    LBracket,
    RBracket,
    Modulo,
    Power,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Arrow,
    // Type system tokens
    Colon,
    DoubleColon,
    TypeInt64,
    TypeBool,
    TypeStr,
    TypeArr,
    TypeVoid,
    // Memory management tokens
    Alloc,
    Dealloc,
    Ref,
    Deref,
    Move,
    Copy,
    Borrow,
    Mut,
    Own,
    // Lifetime tokens
    Lifetime(String), // 'a, 'b, etc.
    // Namespace tokens
    Module,
    Use,
    Pub,
    // Explicit scope tokens
    Scope,
    End,
    // Advanced memory
    Heap,
    Stack,
    SizeOf,
    AlignOf,
    Cast,
    Unsafe,
    On, // for "on heap" or "on stack"
    Number(i64),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Identifier(String),
    StringLiteral(String),
    LBrace,
    RBrace,
    Semicolon,
    Eof,
}

/// Transforms raw LPPO source code into a stream of tokens.
pub struct Lexer<'a> {
    input: &'a [u8],
    position: usize,
    current: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
            current: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.current).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let ch = self.peek()?;
        self.current += 1;
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.bump();
                self.position = self.current;
            }
            // line comment: // ... to end of line
            if self.peek() == Some(b'/') && self.input.get(self.current + 1) == Some(&b'/') {
                while let Some(ch) = self.bump() {
                    if ch == b'\n' {
                        break;
                    }
                }
                self.position = self.current;
                continue;
            }
            // block comment: /* ... */
            if self.peek() == Some(b'/') && self.input.get(self.current + 1) == Some(&b'*') {
                // consume '/*'
                self.bump();
                self.bump();
                // scan until '*/' or EOF
                let mut found_end = false;
                while let Some(ch) = self.bump() {
                    if ch == b'*' && self.peek() == Some(b'/') {
                        // consume '/'
                        self.bump();
                        found_end = true;
                        break;
                    }
                }
                // update position to current (either at end of comment or EOF)
                self.position = self.current;
                // continue scanning for further whitespace/comments
                continue;
            }
            break;
        }
    }

    fn read_identifier(&mut self, first: u8) -> String {
        let mut ident = String::new();
        ident.push(first as char);
        while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')) {
            ident.push(self.bump().unwrap() as char);
        }
        ident
    }

    fn read_string(&mut self) -> LppoResult<String> {
        let mut value = String::new();
        while let Some(ch) = self.bump() {
            match ch {
                b'"' => {
                    self.position = self.current;
                    return Ok(value);
                }
                b'\\' => {
                    let escaped = self.bump().ok_or(LppoError::UnterminatedString {
                        position: self.current,
                    })?;
                    let esc_char = match escaped {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'"' => '"',
                        b'\\' => '\\',
                        other => other as char,
                    };
                    value.push(esc_char);
                }
                other => value.push(other as char),
            }
        }
        Err(LppoError::UnterminatedString {
            position: self.current,
        })
    }

    /// Returns the next token in the input stream.
    pub fn next_token(&mut self) -> LppoResult<Token> {
        self.skip_whitespace_and_comments();
        self.position = self.current;
        let ch = match self.bump() {
            Some(ch) => ch,
            None => return Ok(Token::Eof),
        };

        let token = match ch {
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b';' => Token::Semicolon,
            b'"' => {
                let string = self.read_string()?;
                Token::StringLiteral(string)
            }
                b'0'..=b'9' => {
                    // read number (integer)
                    let mut num = (ch - b'0') as i64;
                    while let Some(d) = self.peek() {
                        if (b'0'..=b'9').contains(&d) {
                            self.bump();
                            num = num * 10 + (d - b'0') as i64;
                        } else {
                            break;
                        }
                    }
                    Token::Number(num)
                }
                b'=' => {
                    if self.peek() == Some(b'=') {
                        self.bump();
                        Token::EqEq
                    } else {
                        Token::Equals
                    }
                }
                b'!' => {
                    if self.peek() == Some(b'=') {
                        self.bump();
                        Token::NotEq
                    } else {
                        return Err(LppoError::UnexpectedCharacter { ch: '!' , position: self.position });
                    }
                }
                b',' => Token::Comma,
                b'.' => Token::Dot,
                b':' => {
                    if self.peek() == Some(b':') {
                        self.bump();
                        Token::DoubleColon
                    } else {
                        Token::Colon
                    }
                }
                b'\'' => {
                    // Lifetime annotation like 'a or 'static
                    let mut lifetime = String::new();
                    while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'_')) {
                        lifetime.push(self.bump().unwrap() as char);
                    }
                    if lifetime.is_empty() {
                        return Err(LppoError::UnexpectedCharacter { ch: '\'', position: self.position });
                    }
                    Token::Lifetime(lifetime)
                }
                b'[' => Token::LBracket,
                b']' => Token::RBracket,
                b'%' => Token::Modulo,
                b'^' => Token::Caret,
                b'~' => Token::Tilde,
                b'&' => Token::Amp,
                b'|' => Token::Pipe,
                b'<' => {
                    if self.peek() == Some(b'=') { self.bump(); Token::LessEq } else { Token::Less }
                }
                b'>' => {
                    if self.peek() == Some(b'=') { self.bump(); Token::GreaterEq } else { Token::Greater }
                }
                b'+' => Token::Plus,
                b'-' => {
                    if self.peek() == Some(b'>') {
                        self.bump();
                        Token::Arrow
                    } else {
                        Token::Minus
                    }
                }
                b'*' => {
                    if self.peek() == Some(b'*') {
                        self.bump();
                        Token::Power
                    } else {
                        Token::Star
                    }
                }
                b'/' => Token::Slash,
                b'(' => Token::LParen,
                b')' => Token::RParen,
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                let ident = self.read_identifier(ch);
                match ident.as_str() {
                    "proc" => Token::Proc,
                    "parallel" => Token::Parallel,
                    "out" => Token::Out,
                        "let" => Token::Let,
                        "if" => Token::If,
                        "elif" => Token::Elif,
                        "else" => Token::Else,
                        "while" => Token::While,
                        "for" => Token::For,
                        "in" => Token::In,
                        "loop" => Token::Loop,
                        "break" => Token::Break,
                        "continue" => Token::Continue,
                        "return" => Token::Return,
                        "struct" => Token::Struct,
                        "new" => Token::New,
                        "true" => Token::Identifier("true".into()),
                        "false" => Token::Identifier("false".into()),
                            "and" => Token::And,
                            "or" => Token::Or,
                            "not" => Token::Not,
                    "call" => Token::Call,
                    // Type keywords
                    "Int64" => Token::TypeInt64,
                    "Bool" => Token::TypeBool,
                    "Str" => Token::TypeStr,
                    "Arr" => Token::TypeArr,
                    "Void" => Token::TypeVoid,
                    // Memory management keywords
                    "alloc" => Token::Alloc,
                    "dealloc" => Token::Dealloc,
                    "ref" => Token::Ref,
                    "deref" => Token::Deref,
                    "move" => Token::Move,
                    "copy" => Token::Copy,
                    "borrow" => Token::Borrow,
                    "mut" => Token::Mut,
                    "own" => Token::Own,
                    // Namespace keywords
                    "module" => Token::Module,
                    "use" => Token::Use,
                    "pub" => Token::Pub,
                    // Scope keywords
                    "scope" => Token::Scope,
                    "end" => Token::End,
                    // Advanced memory
                    "heap" => Token::Heap,
                    "stack" => Token::Stack,
                    "sizeof" => Token::SizeOf,
                    "alignof" => Token::AlignOf,
                    "cast" => Token::Cast,
                    "unsafe" => Token::Unsafe,
                    "on" => Token::On,
                    _ => Token::Identifier(ident),
                }
            }
            other => {
                return Err(LppoError::UnexpectedCharacter {
                    ch: other as char,
                    position: self.position,
                })
            }
        };
        Ok(token)
    }

    /// Returns the index in the input of the last consumed character.
    pub fn position(&self) -> usize {
        self.position
    }
}

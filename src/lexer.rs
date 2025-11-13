use crate::error::{LppoError, LppoResult};

/// Represents the lexical units understood by the parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Proc,
    Parallel,
    Out,
    Input,
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
        Enum,
        Trait,
        Impl,
        Match,
        New,
        Dot,
        Equals,
        Comma,
        Underscore, // _ for wildcard
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
    TypeOption,
    TypeResult,
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
    // V4.0: Ultra-verbose block markers
    BeginSeg,
    EndSeg,
    MemAlign,
    TypeChk,
    LifetmChk,
    StackChk,
    HeapChk,
    // V4.0: Advanced type system
    TypeDef,
    ConstExpr,
    GenRc,
    UnionDef,
    PhantomTyp,
    // V4.0: Memory management extensions
    RegionDef,
    MemPool,
    MemBarrier,
    CacheAln,
    MemLayout,
    // V4.0: Control flow expansion
    ComptLp,
    AsyncBlk,
    AwaitExpr,
    ExceptionHdl,
    CatchBlk,
    FinallyBlk,
    ThrowExpr,
    AssertChk,
    GotoLabl,
    JumpTo,
    // V4.0: Concurrency & parallelism
    ThreadPool,
    AtomicOp,
    LockDef,
    UnlockOp,
    ChanelDef,
    SendMsg,
    RecvMsg,
    FutureDef,
    PromiseDef,
    // V4.0: Metaprogramming
    MacroDef,
    MacroExpand,
    ReflectTyp,
    CompTime,
    AttribDef,
    CodeGen,
    // V4.0: Advanced I/O
    FileHandle,
    OpenFile,
    CloseFile,
    ReadData,
    WriteData,
    NetHandle,
    ConnectNet,
    ListenNet,
    StreamDef,
    BufferDef,
    Serialize,
    Deserialize,
    // V4.0: Safety & verification
    PreCondit,
    PostCondit,
    Invariant,
    ProofHint,
    BoundsCheck,
    OverflowCheck,
    NullCheck,
    // V4.0: Documentation & metadata
    DocString,
    Metadata,
    Benchmark,
    TestCase,
    Deprecate,
    // V4.0: Advanced features
    ClosureDef,
    CaptureDef,
    IteratorDef,
    YieldExpr,
    EffectSys,
    LinearTyp,
    DependTyp,
    AffineTyp,
    // V4.0: Additional type qualifiers
    Volatile,
    Const,
    Static,
    Inline,
    NoInline,
    Packed,
    Aligned,
    // V4.0: Additional control flow
    Select,
    Case,
    Default,
    Fallthrough,
    // V4.0: Additional concurrency
    Barrier,
    Semaphore,
    CondVar,
    RwLock,
    Spinlock,
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
                    let escaped = self.bump().ok_or_else(|| {
                        LppoError::unterminated_string(self.current)
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
        Err(LppoError::unterminated_string(self.current))
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
                        return Err(LppoError::unexpected_char('!', self.position));
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
                        return Err(LppoError::unexpected_char('\'', self.position));
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
                    // V3.0: Cryptic abbreviations replacing conversational keywords
                    "PROCDEF" | "proc" => Token::Proc,  // Keep backward compat during transition
                    "PARLBLK" | "parallel" => Token::Parallel,
                    "EMIT" | "out" => Token::Out,
                    "STDIN" | "input" => Token::Input,
                        "DECLVAR" | "let" => Token::Let,
                        "CJMP" | "if" => Token::If,  // Conditional jump
                        "CJMPALT" | "elif" => Token::Elif,
                        "JMPDEF" | "else" => Token::Else,
                        "WHLLP" | "while" => Token::While,
                        "ITRLP" | "for" => Token::For,  // Iterator loop
                        "INSET" | "in" => Token::In,
                        "INFLP" | "loop" => Token::Loop,  // Infinite loop
                        "LPEXIT" | "break" => Token::Break,
                        "LPSKIP" | "continue" => Token::Continue,
                        "RETVL" | "return" => Token::Return,
                        "STRUCTDEF" | "struct" => Token::Struct,
                        "ENUMDEF" | "enum" => Token::Enum,
                        "TRAITDEF" | "trait" => Token::Trait,
                        "IMPLBLK" | "impl" => Token::Impl,
                        "PATMCH" | "match" => Token::Match,
                        "INSTCRT" | "new" => Token::New,
                        "BOOLT" | "true" => Token::Identifier("true".into()),
                        "BOOLF" | "false" => Token::Identifier("false".into()),
                            "LOGAND" | "and" => Token::And,
                            "LOGOR" | "or" => Token::Or,
                            "LOGNOT" | "not" => Token::Not,
                    "PROCCALL" | "call" => Token::Call,
                    // Type keywords - V3.0 with explicit bit sizes
                    "I64" | "Int64" => Token::TypeInt64,
                    "BOOL8" | "Bool" => Token::TypeBool,
                    "STRPTR" | "Str" => Token::TypeStr,
                    "ARRPTR" | "Arr" => Token::TypeArr,
                    "VOID0" | "Void" => Token::TypeVoid,
                    "OPT" | "Option" => Token::TypeOption,
                    "RES" | "Result" => Token::TypeResult,
                    // Memory management keywords - V3.0 low-level
                    "HEAPALLOC" | "alloc" => Token::Alloc,
                    "HEAPFREE" | "dealloc" => Token::Dealloc,
                    "MEMREF" | "ref" => Token::Ref,
                    "MEMDEREF" | "deref" => Token::Deref,
                    "MEMXFER" | "move" => Token::Move,
                    "MEMCPY" | "copy" => Token::Copy,
                    "BORROW" | "borrow" => Token::Borrow,
                    "MUTBL" | "mut" => Token::Mut,
                    "OWNR" | "own" => Token::Own,
                    // Namespace keywords
                    "MODDEF" | "module" => Token::Module,
                    "MODIMP" | "use" => Token::Use,
                    "PUBEXP" | "pub" => Token::Pub,
                    // Scope keywords - V3.0 explicit scope management
                    "STKFRAME" | "scope" => Token::Scope,
                    "FRAMEEND" | "end" => Token::End,
                    // Advanced memory - V3.0 machine-level
                    "HEAPSEG" | "heap" => Token::Heap,
                    "STKSEG" | "stack" => Token::Stack,
                    "TYPESZ" | "sizeof" => Token::SizeOf,
                    "TYPEALN" | "alignof" => Token::AlignOf,
                    "TYPECAST" | "cast" => Token::Cast,
                    "UNSAFEBLK" | "unsafe" => Token::Unsafe,
                    "MEMSEG" | "on" => Token::On,
                    // V4.0: Ultra-verbose block markers
                    "BEGINSEG" => Token::BeginSeg,
                    "ENDSEG" => Token::EndSeg,
                    "MEMALIGN" => Token::MemAlign,
                    "TYPECHK" => Token::TypeChk,
                    "LIFETMCHK" => Token::LifetmChk,
                    "STACKCHK" => Token::StackChk,
                    "HEAPCHK" => Token::HeapChk,
                    // V4.0: Advanced type system
                    "TYPDEF" => Token::TypeDef,
                    "CONSTEXPR" => Token::ConstExpr,
                    "GENRC" => Token::GenRc,
                    "UNIONDEF" => Token::UnionDef,
                    "PHANTOMTYP" => Token::PhantomTyp,
                    // V4.0: Memory management extensions
                    "REGIONDEF" => Token::RegionDef,
                    "MEMPOOL" => Token::MemPool,
                    "MEMBARRIER" => Token::MemBarrier,
                    "CACHEALN" => Token::CacheAln,
                    "MEMLAYOUT" => Token::MemLayout,
                    // V4.0: Control flow expansion
                    "COMPTLP" => Token::ComptLp,
                    "ASYNCBLK" => Token::AsyncBlk,
                    "AWAITEXPR" => Token::AwaitExpr,
                    "EXCEPTIONHDL" => Token::ExceptionHdl,
                    "CATCHBLK" => Token::CatchBlk,
                    "FINALLYBLK" => Token::FinallyBlk,
                    "THROWEXPR" => Token::ThrowExpr,
                    "ASSERTCHK" => Token::AssertChk,
                    "GOTOLABL" => Token::GotoLabl,
                    "JUMPTO" => Token::JumpTo,
                    // V4.0: Concurrency & parallelism
                    "THREADPOOL" => Token::ThreadPool,
                    "ATOMICOP" => Token::AtomicOp,
                    "LOCKDEF" => Token::LockDef,
                    "UNLOCKOP" => Token::UnlockOp,
                    "CHANELDEF" => Token::ChanelDef,
                    "SENDMSG" => Token::SendMsg,
                    "RECVMSG" => Token::RecvMsg,
                    "FUTUREDEF" => Token::FutureDef,
                    "PROMISEDEF" => Token::PromiseDef,
                    // V4.0: Metaprogramming
                    "MACRODEF" => Token::MacroDef,
                    "MACROEXPAND" => Token::MacroExpand,
                    "REFLECTTYP" => Token::ReflectTyp,
                    "COMPTIME" => Token::CompTime,
                    "ATTRIBDEF" => Token::AttribDef,
                    "CODEGEN" => Token::CodeGen,
                    // V4.0: Advanced I/O
                    "FILEHANDLE" => Token::FileHandle,
                    "OPENFILE" => Token::OpenFile,
                    "CLOSEFILE" => Token::CloseFile,
                    "READDATA" => Token::ReadData,
                    "WRITEDATA" => Token::WriteData,
                    "NETHANDLE" => Token::NetHandle,
                    "CONNECTNET" => Token::ConnectNet,
                    "LISTENNET" => Token::ListenNet,
                    "STREAMDEF" => Token::StreamDef,
                    "BUFFERDEF" => Token::BufferDef,
                    "SERIALIZE" => Token::Serialize,
                    "DESERIALIZE" => Token::Deserialize,
                    // V4.0: Safety & verification
                    "PRECONDIT" => Token::PreCondit,
                    "POSTCONDIT" => Token::PostCondit,
                    "INVARIANT" => Token::Invariant,
                    "PROOFHINT" => Token::ProofHint,
                    "BOUNDSCHECK" => Token::BoundsCheck,
                    "OVERFLOWCHECK" => Token::OverflowCheck,
                    "NULLCHECK" => Token::NullCheck,
                    // V4.0: Documentation & metadata
                    "DOCSTRING" => Token::DocString,
                    "METADATA" => Token::Metadata,
                    "BENCHMARK" => Token::Benchmark,
                    "TESTCASE" => Token::TestCase,
                    "DEPRECATE" => Token::Deprecate,
                    // V4.0: Advanced features
                    "CLOSUREDEF" => Token::ClosureDef,
                    "CAPTUREDEF" => Token::CaptureDef,
                    "ITERATORDEF" => Token::IteratorDef,
                    "YIELDEXPR" => Token::YieldExpr,
                    "EFFECTSYS" => Token::EffectSys,
                    "LINEARTYP" => Token::LinearTyp,
                    "DEPENDTYP" => Token::DependTyp,
                    "AFFINETYP" => Token::AffineTyp,
                    // V4.0: Additional type qualifiers
                    "VOLATILE" => Token::Volatile,
                    "CONST" => Token::Const,
                    "STATIC" => Token::Static,
                    "INLINE" => Token::Inline,
                    "NOINLINE" => Token::NoInline,
                    "PACKED" => Token::Packed,
                    "ALIGNED" => Token::Aligned,
                    // V4.0: Additional control flow
                    "SELECT" => Token::Select,
                    "CASE" => Token::Case,
                    "DEFAULT" => Token::Default,
                    "FALLTHROUGH" => Token::Fallthrough,
                    // V4.0: Additional concurrency
                    "BARRIER" => Token::Barrier,
                    "SEMAPHORE" => Token::Semaphore,
                    "CONDVAR" => Token::CondVar,
                    "RWLOCK" => Token::RwLock,
                    "SPINLOCK" => Token::Spinlock,
                    _ => Token::Identifier(ident),
                }
            }
            other => {
                return Err(LppoError::unexpected_char(other as char, self.position))
            }
        };
        Ok(token)
    }

    /// Returns the index in the input of the last consumed character.
    pub fn position(&self) -> usize {
        self.position
    }
}

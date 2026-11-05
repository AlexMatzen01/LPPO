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
    Class,
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
    Bang, // ! for macros like vec![...]
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
    Ring,
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

    // V5.2: Extended Type System (150+ new type features)
    TypeF32,
    TypeF64,
    TypeU8,
    TypeU16,
    TypeU32,
    TypeU64,
    TypeU128,
    TypeI8,
    TypeI16,
    TypeI32,
    TypeI128,
    TypeChar,
    TypeByte,
    TypeFloat,
    TypeDouble,
    TypeUsize,
    TypeIsize,
    TypeTuple,
    TypeUnion,
    TypeIntersection,
    TypeAny,
    TypeNever,
    TypeUnknown,
    TypeDynamic,
    TypeAbstract,
    TypeSealed,
    TypeFinal,
    TypeVirtual,
    TypeOverride,
    TypeInterface,
    TypeDelegate,
    TypeEvent,
    TypeProperty,
    TypeIndexer,
    TypeExtension,
    TypePartial,
    TypeRecord,
    TypeNominal,
    TypeStructural,
    TypeVariant,
    TypeNewtype,
    TypeOpaque,
    TypeTransparent,
    TypeExistential,
    TypeUniversal,
    TypeForall,
    TypeExists,
    TypeConstraint,
    TypeBound,
    TypeWhere,
    TypeAssoc,
    TypeFamily,
    TypeInstance,
    TypeClass,
    TypeCovariant,
    TypeContravariant,
    TypeInvariant,
    TypeBivariant,
    TypePhantom,
    TypeZeroSized,
    TypeUninit,
    TypeMaybeUninit,

    // V5.2: Advanced Numeric Types
    TypeDecimal,
    TypeRational,
    TypeComplex,
    TypeBigInt,
    TypeBigDecimal,
    TypeFixed,
    TypeSaturating,
    TypeWrapping,
    TypeChecked,
    TypeNonZero,

    // V5.2: String & Text Types
    TypeString,
    TypeCStr,
    TypeOsStr,
    TypePath,
    TypePathBuf,
    TypeCString,
    TypeOsString,
    TypeBytes,
    TypeByteString,
    TypeRawStr,
    TypeInterpolated,

    // V5.2: Container Types
    TypeVec,
    TypeArray,
    TypeSlice,
    TypeHashMap,
    TypeHashSet,
    TypeBTreeMap,
    TypeBTreeSet,
    TypeLinkedList,
    TypeVecDeque,
    TypeBinaryHeap,
    TypeSmallVec,
    TypeTinyVec,
    TypeArrayVec,

    // V5.2: Pointer & Reference Types
    TypeBox,
    TypeRc,
    TypeArc,
    TypeCell,
    TypeRefCell,
    TypeOnceCell,
    TypeLazyCell,
    TypeWeak,
    TypeRawPtr,
    TypeNonNull,
    TypePin,
    TypeManualDrop,
    TypeMaybeOwned,
    TypeCow,

    // V5.2: Async Types
    TypeFuture,
    TypeStream,
    TypeSink,
    TypeTask,
    TypeJoinHandle,
    TypeAsyncBlock,
    TypeAsyncFn,
    TypeAsyncClosure,

    // V5.2: Function Types
    TypeFn,
    TypeFnOnce,
    TypeFnMut,
    TypeClosure,
    TypeGenerator,
    TypeCoroutine,
    TypeProc,
    TypeMethod,
    TypeConstructor,
    TypeDestructor,

    // V5.2: Error Types
    TypeError,
    TypeException,
    TypePanic,
    TypeAbort,
    TypeTrap,

    // V5.2: Time & Duration Types
    TypeDuration,
    TypeInstant,
    TypeSystemTime,
    TypeDateTime,
    TypeDate,
    TypeTime,
    TypeTimeZone,
    TypeChrono,

    // V5.2: I/O & Network Types
    TypeFile,
    TypeDir,
    TypeStdin,
    TypeStdout,
    TypeStderr,
    TypeBufReader,
    TypeBufWriter,
    TypeTcpStream,
    TypeTcpListener,
    TypeUdpSocket,
    TypeUnixStream,
    TypeUnixListener,

    // V5.2: Synchronization Types
    TypeMutex,
    TypeRwLock,
    TypeSemaphore,
    TypeBarrier,
    TypeCondVar,
    TypeAtomic,
    TypeOnce,
    TypeLatch,

    // V5.2: Range Types
    TypeRange,
    TypeRangeInclusive,
    TypeRangeFrom,
    TypeRangeTo,
    TypeRangeFull,

    // V5.2: Advanced Keywords (150+ new language features)
    Async,
    Await,
    Yield,
    Resume,
    Suspend,
    Generator,
    Coroutine,
    Closure,
    Lambda,
    Capture,
    Try,
    Catch,
    Finally,
    Throw,
    Panic,
    Abort,
    Unreachable,
    Todo,
    Unimplemented,
    Assert,
    AssertEq,
    AssertNe,
    Debug,
    DebugAssert,
    // Static, Const, Inline, NoInline, Volatile already defined in V4.0
    Constexpr,
    ConstEval,
    ConstFn,
    AlwaysInline,
    Naked,
    Cold,
    Hot,
    Pure,
    Impure,
    Extern,
    Export,
    Import,
    Package,
    Crate,
    Super,
    SelfType,
    SelfValue,
    This,
    Base,
    Parent,
    Virtual,
    Override,
    Abstract,
    Final,
    Sealed,
    Partial,
    Extension,
    Operator,
    Implicit,
    Explicit,
    As,
    Is,
    // In already defined earlier
    Of,
    With,
    Without,
    Where,
    When,
    Unless,
    Until,
    Repeat,
    Do,
    Then,
    // Elif already defined earlier
    Elseif,
    Otherwise,
    // Select, Case, Default, Fallthrough already defined in V4.0
    Switch,
    Goto,
    Label,
    Target,
    Jump,
    Skip,
    Defer,
    Lazy,
    Eager,
    Memo,
    Cached,
    Weak,
    Strong,
    Pinned,
    Unpinned,
    Sized,
    Unsized,
    ZeroSized,
    // Packed, Aligned already defined in V4.0
    Repr,
    Layout,
    Transparent,
    Opaque,
    Alias,
    Newtype,
    Phantom,
    Marker,
    Auto,
    Builtin,
    Lang,
    Intrinsic,
    Primitive,
    Native,
    Foreign,
    Abi,
    Cdecl,
    Stdcall,
    Fastcall,
    Vectorcall,
    Thiscall,
    Aapcs,
    Win64,
    SysV64,

    // V5.2: Pattern Matching Extensions
    Some,
    None,
    Ok,
    Err,
    Guard,
    At,
    Range,
    Rest,
    Wildcard,
    Binding,
    Refutable,
    Irrefutable,

    // V5.2: Macro System
    Macro,
    MacroRules,
    MacroUse,
    MacroExport,
    Stringify,
    Concat,
    Include,
    IncludeStr,
    IncludeBytes,
    File,
    Line,
    Column,
    ModulePath,
    OptionEnv,
    Env,
    Cfg,
    CfgAttr,
    Feature,

    // V5.2: Attributes & Annotations
    Attribute,
    Derive,
    Test,
    Bench,
    Should,
    Ignore,
    Allow,
    Warn,
    Deny,
    Forbid,
    Deprecated,
    MustUse,
    NoMangle,
    LinkSection,
    Used,
    // Repr already defined above
    NonExhaustive,
    Doc,
    Since,
    Unstable,
    Stable,

    // V5.2: Lifetime & Variance
    LifetimeStatic,
    LifetimeAnon,
    LifetimeElided,
    LifetimeBound,
    Covariant,
    Contravariant,
    // Invariant already defined in V4.0
    Bivariant,

    // V5.2: Effect System
    Effect,
    Handler,
    // Resume already defined above
    // Continue already defined earlier
    Algebraic,
    Perform,
    Handle,
    // With already defined above

    // V5.2: Linear & Affine Types
    Linear,
    Affine,
    Relevant,
    Consume,
    Drop,
    Forget,
    Leak,

    // V5.2: Dependent Types
    Dependent,
    Indexed,
    Singleton,
    Refinement,
    Predicate,
    Constraint,
    Proof,
    QED,
    Axiom,
    Lemma,
    Theorem,

    // V5.2: Memory Operations
    Memcpy,
    Memmove,
    Memset,
    Memcmp,
    Malloc,
    Calloc,
    Realloc,
    Free,
    AlignedAlloc,
    Zalloc,

    // V5.2: Synchronization Primitives
    AtomicLoad,
    AtomicStore,
    AtomicSwap,
    AtomicCompareExchange,
    AtomicFetchAdd,
    AtomicFetchSub,
    AtomicFetchAnd,
    AtomicFetchOr,
    AtomicFetchXor,
    AtomicFence,
    Acquire,
    Release,
    AcqRel,
    SeqCst,
    Relaxed,

    // V5.2: SIMD & Vectorization
    Simd,
    Vector,
    Splat,
    Extract,
    Insert,
    Shuffle,
    Swizzle,
    Reduce,
    Horizontal,
    Vertical,

    // V5.2: GPU & Parallel Computing
    Gpu,
    Kernel,
    Grid,
    Block,
    Thread,
    Warp,
    Global,
    Shared,
    Local,
    Constant,
    Device,
    Host,

    // V5.2: JIT & Compilation
    Jit,
    Compile,
    Interpret,
    Optimize,
    // Inline already defined in V4.0
    Specialize,
    Monomorphize,
    Devirtualize,

    // V5.2: Reflection & Metaprogramming
    TypeOf,
    TypeId,
    TypeName,
    // SizeOf, AlignOf already defined earlier
    OffsetOf,
    StrideOf,
    FieldOf,
    MethodOf,
    TraitOf,
    ImplOf,

    // V5.2: Serialization & Encoding
    // Serialize, Deserialize already defined in V4.0
    Encode,
    Decode,
    Marshal,
    Unmarshal,
    Pack,
    Unpack,
    Json,
    Toml,
    Yaml,
    Xml,
    Csv,
    Binary,
    Base64,
    Hex,

    // V5.2: Cryptography
    Hash,
    Sha256,
    Sha512,
    Md5,
    Blake2,
    Blake3,
    Hmac,
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    Random,
    SecureRandom,

    // V5.2: Compression
    Compress,
    Decompress,
    Gzip,
    Zlib,
    Bzip2,
    Lz4,
    Zstd,

    // V5.2: Logging & Debugging
    Log,
    Trace,
    // Debug already defined above
    Info,
    Warning,
    Error,
    Critical,
    Fatal,
    Print,
    Println,
    Eprint,
    Eprintln,
    Dbg,
    Pause,

    // V5.2: Testing & Benchmarking
    // Test, Bench already defined above
    Fuzz,
    Property,
    Mock,
    Stub,
    Spy,
    Fixture,
    Setup,
    Teardown,
    Given,
    // When, Then already defined above
    Expect,
    // Should already defined above

    // V5.2: Documentation
    // Doc, Warning, Deprecated, Since, Return already defined above or earlier
    Example,
    Note,
    See,
    Param,
    Throws,

    // V5.2: Build & Compilation
    Build,
    Clean,
    Check,
    Clippy,
    Fmt,
    Publish,
    Install,
    Run,

    // V5.2: Additional Operators
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    ModEq,
    PowerEq,
    AmpEq,
    PipeEq,
    CaretEq,
    LShift,
    RShift,
    LShiftEq,
    RShiftEq,
    Question,
    Elvis,
    Spaceship,
    Pipeline,
    Compose,
    Dollar,
    // At already defined above
    // Hash already defined above
    Backslash,
    Backtick,
    TripleDot,
    DoubleDot,
    FatArrow,
    ThinArrow,
    LArrow,
    RArrow,
    DoubleArrow,
    TripleArrow,
    // At, Hash already defined above or cause conflicts
    Number(i64),
    FloatLiteral(String), // Store as string to maintain Eq
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Identifier(String),
    StringLiteral(String),
    CharLiteral(char),
    ByteLiteral(u8),
    RawStringLiteral(String),
    ByteStringLiteral(Vec<u8>),
    LBrace,
    RBrace,
    Semicolon,
    Eof,
}

/// Transforms raw LPPO source code into a stream of tokens.
#[derive(Clone)]
pub struct Lexer<'a> {
    input: &'a [u8],
    position: usize,
    current: usize,
    line: usize,
    column: usize,
    token_start_position: usize,
    token_start_line: usize,
    token_start_column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            position: 0,
            current: 0,
            line: 0,
            column: 0,
            token_start_position: 0,
            token_start_line: 0,
            token_start_column: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.current).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let ch = self.peek()?;
        self.current += 1;
        // update line/column based on the character we just consumed
        if ch == b'\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
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
                while let Some(ch) = self.bump() {
                    if ch == b'*' && self.peek() == Some(b'/') {
                        // consume '/'
                        self.bump();
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

    fn current_token_location(&self) -> crate::error::SourceLocation {
        crate::error::SourceLocation::with_line_col(
            self.token_start_line,
            self.token_start_column,
            self.token_start_position,
        )
    }

    fn read_identifier(&mut self, first: u8) -> String {
        let mut ident = String::new();
        ident.push(first as char);
        while matches!(
            self.peek(),
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
        ) {
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
                        LppoError::unterminated_string_at(self.current_token_location())
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
        Err(LppoError::unterminated_string_at(
            self.current_token_location(),
        ))
    }

    /// Returns the next token in the input stream.
    pub fn next_token(&mut self) -> LppoResult<Token> {
        self.skip_whitespace_and_comments();
        // mark token start location before consuming any character
        self.token_start_position = self.current;
        self.token_start_line = self.line;
        self.token_start_column = self.column;
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
                // read number (integer or float)
                let mut num_str = String::new();
                num_str.push(ch as char);
                let mut is_float = false;

                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        self.bump();
                        num_str.push(d as char);
                    } else if d == b'.' && !is_float {
                        // Check if this is a decimal point (not method call)
                        // Peek ahead to see if followed by digit
                        self.bump(); // consume '.'
                        if let Some(next) = self.peek() {
                            if next.is_ascii_digit() {
                                is_float = true;
                                num_str.push('.');
                            } else {
                                // Not a float, it's a method call or range
                                // Put the dot back by decrementing position
                                self.position -= 1;
                                self.current -= 1;
                                self.column -= 1;
                                break;
                            }
                        } else {
                            // EOF after dot, not a float
                            self.position -= 1;
                            self.current -= 1;
                            self.column -= 1;
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if is_float {
                    Token::FloatLiteral(num_str)
                } else {
                    Token::Number(num_str.parse().unwrap())
                }
            }
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Token::EqEq
                } else if self.peek() == Some(b'>') {
                    self.bump();
                    Token::FatArrow
                } else {
                    Token::Equals
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Token::NotEq
                } else {
                    Token::Bang
                }
            }
            b',' => Token::Comma,
            b'.' => Token::Dot,
            b'?' => Token::Question,
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
                    return Err(LppoError::unexpected_char_at(
                        '\'',
                        self.current_token_location(),
                    ));
                }
                Token::Lifetime(lifetime)
            }
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b'%' => Token::Modulo,
            b'^' => Token::Caret,
            b'~' => Token::Tilde,
            b'#' => Token::Hash,
            b'&' => Token::Amp,
            b'|' => Token::Pipe,
            b'<' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Token::LessEq
                } else {
                    Token::Less
                }
            }
            b'>' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Token::GreaterEq
                } else {
                    Token::Greater
                }
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
                    "PROCDEF" | "proc" => Token::Proc, // Keep backward compat during transition
                    "PARLBLK" | "parallel" => Token::Parallel,
                    "EMIT" | "out" => Token::Out,
                    "STDIN" | "input" => Token::Input,
                    "DECLVAR" | "let" => Token::Let,
                    "CJMP" | "if" => Token::If, // Conditional jump
                    "CJMPALT" | "elif" => Token::Elif,
                    "JMPDEF" | "else" => Token::Else,
                    "WHLLP" | "while" => Token::While,
                    "ITRLP" | "for" => Token::For, // Iterator loop
                    "INSET" | "in" => Token::In,
                    "INFLP" | "loop" => Token::Loop, // Infinite loop
                    "LPEXIT" | "break" => Token::Break,
                    "LPSKIP" | "continue" => Token::Continue,
                    "RETVL" | "return" => Token::Return,
                    "STRUCTDEF" | "struct" => Token::Struct,
                    "CLASSDEF" | "class" => Token::Class,
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
                    "RING0" | "ring" => Token::Ring,
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
                    "TYPDEF" | "type" => Token::TypeDef,
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

                    // V5.2: Extended Type System
                    "F32" | "Float32" => Token::TypeF32,
                    "F64" | "Float64" => Token::TypeF64,
                    "U8" | "UInt8" => Token::TypeU8,
                    "U16" | "UInt16" => Token::TypeU16,
                    "U32" | "UInt32" => Token::TypeU32,
                    "U64" | "UInt64" => Token::TypeU64,
                    "U128" | "UInt128" => Token::TypeU128,
                    "I8" | "Int8" => Token::TypeI8,
                    "I16" | "Int16" => Token::TypeI16,
                    "I32" | "Int32" => Token::TypeI32,
                    "I128" | "Int128" => Token::TypeI128,
                    "Char" => Token::TypeChar,
                    "Byte" => Token::TypeByte,
                    // Treat generic keyword `Float` and `Double` as 64-bit floats
                    "Float" => Token::TypeF64,
                    "Double" => Token::TypeF64,
                    "Usize" => Token::TypeUsize,
                    "Isize" => Token::TypeIsize,
                    "Tuple" => Token::TypeTuple,
                    "Union" => Token::TypeUnion,
                    "Intersection" => Token::TypeIntersection,
                    "Any" => Token::TypeAny,
                    "Never" => Token::TypeNever,
                    "Unknown" => Token::TypeUnknown,
                    "Dynamic" => Token::TypeDynamic,

                    // V5.2: Advanced Keywords
                    "async" => Token::Async,
                    "await" => Token::Await,
                    "yield" => Token::Yield,
                    "resume" => Token::Resume,
                    "suspend" => Token::Suspend,
                    "generator" => Token::Generator,
                    "coroutine" => Token::Coroutine,
                    "closure" => Token::Closure,
                    "lambda" => Token::Lambda,
                    "capture" => Token::Capture,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "finally" => Token::Finally,
                    "throw" => Token::Throw,
                    "panic" => Token::Panic,
                    "abort" => Token::Abort,
                    "unreachable" => Token::Unreachable,
                    "todo" => Token::Todo,
                    "unimplemented" => Token::Unimplemented,
                    "assert" => Token::Assert,
                    "assert_eq" => Token::AssertEq,
                    "assert_ne" => Token::AssertNe,
                    "debug" => Token::Debug,
                    "debug_assert" => Token::DebugAssert,
                    "static" => Token::Static,
                    "const" => Token::Const,
                    "constexpr" => Token::Constexpr,
                    "const_eval" => Token::ConstEval,
                    "const_fn" => Token::ConstFn,
                    "inline" => Token::Inline,
                    "noinline" => Token::NoInline,
                    "always_inline" => Token::AlwaysInline,
                    "naked" => Token::Naked,
                    "cold" => Token::Cold,
                    "hot" => Token::Hot,
                    "pure" => Token::Pure,
                    "impure" => Token::Impure,
                    "volatile" => Token::Volatile,
                    "extern" => Token::Extern,
                    "export" => Token::Export,
                    "import" => Token::Import,
                    "package" => Token::Package,
                    "crate" => Token::Crate,
                    "super" => Token::Super,
                    "Self" => Token::SelfType,
                    "self" => Token::SelfValue,
                    "this" => Token::This,
                    "base" => Token::Base,
                    "parent" => Token::Parent,
                    "virtual" => Token::Virtual,
                    "override" => Token::Override,
                    "abstract" => Token::Abstract,
                    "final" => Token::Final,
                    "sealed" => Token::Sealed,
                    "partial" => Token::Partial,
                    "extension" => Token::Extension,
                    "operator" => Token::Operator,
                    "implicit" => Token::Implicit,
                    "explicit" => Token::Explicit,
                    "as" => Token::As,
                    "is" => Token::Is,
                    "of" => Token::Of,
                    "with" => Token::With,
                    "without" => Token::Without,
                    "where" => Token::Where,
                    "when" => Token::When,
                    "unless" => Token::Unless,
                    "until" => Token::Until,
                    "repeat" => Token::Repeat,
                    "do" => Token::Do,
                    "then" => Token::Then,
                    "elseif" => Token::Elseif,
                    "otherwise" => Token::Otherwise,
                    "select" => Token::Select,
                    "case" => Token::Case,
                    "default" => Token::Default,
                    "switch" => Token::Switch,
                    "fallthrough" => Token::Fallthrough,
                    "goto" => Token::Goto,
                    // "label" => Token::Label,  // Commented out - conflicts with common variable name
                    // "target" => Token::Target,  // Commented out - conflicts with common variable name
                    "jump" => Token::Jump,
                    "skip" => Token::Skip,
                    "defer" => Token::Defer,
                    "lazy" => Token::Lazy,
                    "eager" => Token::Eager,
                    "memo" => Token::Memo,
                    "cached" => Token::Cached,
                    "weak" => Token::Weak,
                    "strong" => Token::Strong,
                    "pinned" => Token::Pinned,
                    "unpinned" => Token::Unpinned,
                    "sized" => Token::Sized,
                    "unsized" => Token::Unsized,
                    "zero_sized" => Token::ZeroSized,
                    "packed" => Token::Packed,
                    "aligned" => Token::Aligned,
                    "repr" => Token::Repr,
                    "layout" => Token::Layout,
                    "transparent" => Token::Transparent,
                    "opaque" => Token::Opaque,
                    "alias" => Token::Alias,
                    "newtype" => Token::Newtype,
                    "phantom" => Token::Phantom,
                    "marker" => Token::Marker,
                    "auto" => Token::Auto,
                    "builtin" => Token::Builtin,
                    "lang" => Token::Lang,
                    "intrinsic" => Token::Intrinsic,
                    "primitive" => Token::Primitive,
                    "native" => Token::Native,
                    "foreign" => Token::Foreign,

                    // V5.2: Pattern Matching
                    "Some" => Token::Some,
                    "None" => Token::None,
                    "Ok" => Token::Ok,
                    "Err" => Token::Err,
                    "guard" => Token::Guard,
                    "at" => Token::At,
                    "range" => Token::Range,
                    "rest" => Token::Rest,

                    // V5.2: Macro System
                    "macro" => Token::Macro,
                    "macro_rules" => Token::MacroRules,
                    "macro_use" => Token::MacroUse,
                    "macro_export" => Token::MacroExport,
                    "stringify" => Token::Stringify,
                    "concat" => Token::Concat,
                    "include" => Token::Include,
                    "include_str" => Token::IncludeStr,
                    "include_bytes" => Token::IncludeBytes,
                    "file" => Token::File,
                    "line" => Token::Line,
                    "column" => Token::Column,

                    // V5.2: Attributes
                    "derive" => Token::Derive,
                    "test" => Token::Test,
                    "bench" => Token::Bench,
                    "should" => Token::Should,
                    "ignore" => Token::Ignore,
                    "allow" => Token::Allow,
                    "warn" => Token::Warn,
                    "deny" => Token::Deny,
                    "forbid" => Token::Forbid,
                    "deprecated" => Token::Deprecated,
                    "must_use" => Token::MustUse,

                    // V5.2: Effect System
                    "effect" => Token::Effect,
                    "handler" => Token::Handler,
                    "perform" => Token::Perform,
                    "handle" => Token::Handle,

                    // V5.2: Linear & Affine Types
                    "linear" => Token::Linear,
                    "affine" => Token::Affine,
                    "relevant" => Token::Relevant,
                    "consume" => Token::Consume,
                    "drop" => Token::Drop,
                    "forget" => Token::Forget,
                    "leak" => Token::Leak,

                    // V5.2: Memory Operations
                    "memcpy" => Token::Memcpy,
                    "memmove" => Token::Memmove,
                    "memset" => Token::Memset,
                    "memcmp" => Token::Memcmp,
                    "malloc" => Token::Malloc,
                    "calloc" => Token::Calloc,
                    "realloc" => Token::Realloc,
                    "free" => Token::Free,

                    // V5.2: Synchronization
                    "atomic_load" => Token::AtomicLoad,
                    "atomic_store" => Token::AtomicStore,
                    "atomic_swap" => Token::AtomicSwap,
                    "acquire" => Token::Acquire,
                    "release" => Token::Release,
                    "relaxed" => Token::Relaxed,

                    // V5.2: SIMD & Vectorization
                    "simd" => Token::Simd,
                    "vector" => Token::Vector,
                    "splat" => Token::Splat,
                    "extract" => Token::Extract,
                    "insert" => Token::Insert,
                    "shuffle" => Token::Shuffle,
                    "swizzle" => Token::Swizzle,
                    "reduce" => Token::Reduce,

                    // V5.2: GPU & Parallel Computing
                    "gpu" => Token::Gpu,
                    "kernel" => Token::Kernel,
                    "grid" => Token::Grid,
                    "block" => Token::Block,
                    "thread" => Token::Thread,
                    "warp" => Token::Warp,
                    "global" => Token::Global,
                    "shared" => Token::Shared,
                    "local" => Token::Local,
                    "constant" => Token::Constant,
                    "device" => Token::Device,
                    "host" => Token::Host,

                    // V5.2: JIT & Compilation
                    "jit" => Token::Jit,
                    "compile" => Token::Compile,
                    "interpret" => Token::Interpret,
                    "optimize" => Token::Optimize,
                    "specialize" => Token::Specialize,

                    // V5.2: Reflection
                    "typeof" => Token::TypeOf,
                    "typeid" => Token::TypeId,
                    "typename" => Token::TypeName,
                    // sizeof, alignof already mapped above
                    "offsetof" => Token::OffsetOf,

                    // V5.2: Serialization
                    // serialize, deserialize already mapped above
                    "encode" => Token::Encode,
                    "decode" => Token::Decode,
                    "marshal" => Token::Marshal,
                    "unmarshal" => Token::Unmarshal,
                    "pack" => Token::Pack,
                    "unpack" => Token::Unpack,
                    "json" => Token::Json,
                    "toml" => Token::Toml,
                    "yaml" => Token::Yaml,
                    "xml" => Token::Xml,
                    "csv" => Token::Csv,
                    "binary" => Token::Binary,
                    "base64" => Token::Base64,
                    "hex" => Token::Hex,

                    // V5.2: Cryptography
                    "hash" => Token::Hash,
                    "sha256" => Token::Sha256,
                    "sha512" => Token::Sha512,
                    "md5" => Token::Md5,
                    "blake2" => Token::Blake2,
                    "blake3" => Token::Blake3,
                    "hmac" => Token::Hmac,
                    "encrypt" => Token::Encrypt,
                    "decrypt" => Token::Decrypt,
                    "sign" => Token::Sign,
                    "verify" => Token::Verify,
                    "secure_random" => Token::SecureRandom,

                    // V5.2: Compression
                    "compress" => Token::Compress,
                    "decompress" => Token::Decompress,
                    "gzip" => Token::Gzip,
                    "zlib" => Token::Zlib,
                    "bzip2" => Token::Bzip2,
                    "lz4" => Token::Lz4,
                    "zstd" => Token::Zstd,

                    // V5.2: Logging
                    "log" => Token::Log,
                    "trace" => Token::Trace,
                    "info" => Token::Info,
                    "warning" => Token::Warning,
                    "error" => Token::Error,
                    "critical" => Token::Critical,
                    "fatal" => Token::Fatal,
                    "print" => Token::Print,
                    "println" => Token::Println,
                    "pause" => Token::Pause,
                    "eprint" => Token::Eprint,
                    "eprintln" => Token::Eprintln,
                    "dbg" => Token::Dbg,

                    // V5.2: Testing
                    "fuzz" => Token::Fuzz,
                    "property" => Token::Property,
                    "mock" => Token::Mock,
                    "stub" => Token::Stub,
                    "spy" => Token::Spy,
                    "fixture" => Token::Fixture,
                    "setup" => Token::Setup,
                    "teardown" => Token::Teardown,
                    "given" => Token::Given,
                    // when, then already mapped above
                    "expect" => Token::Expect,

                    // V5.2: Documentation
                    "doc" => Token::Doc,
                    "example" => Token::Example,
                    "note" => Token::Note,
                    "see" => Token::See,
                    "param" => Token::Param,
                    "throws" => Token::Throws,

                    // V5.2: Build & Compilation
                    "build" => Token::Build,
                    "clean" => Token::Clean,
                    "check" => Token::Check,
                    "clippy" => Token::Clippy,
                    "fmt" => Token::Fmt,
                    "publish" => Token::Publish,
                    "install" => Token::Install,
                    "run" => Token::Run,

                    // V5.2: Container Types
                    "Vec" => Token::TypeVec,
                    "Array" => Token::TypeArray,
                    "Slice" => Token::TypeSlice,
                    "HashMap" => Token::TypeHashMap,
                    "HashSet" => Token::TypeHashSet,
                    "BTreeMap" => Token::TypeBTreeMap,
                    "BTreeSet" => Token::TypeBTreeSet,
                    "LinkedList" => Token::TypeLinkedList,
                    "VecDeque" => Token::TypeVecDeque,
                    "BinaryHeap" => Token::TypeBinaryHeap,

                    // V5.2: Pointer Types
                    "Box" => Token::TypeBox,
                    "Rc" => Token::TypeRc,
                    "Arc" => Token::TypeArc,
                    "Cell" => Token::TypeCell,
                    "RefCell" => Token::TypeRefCell,
                    "Pin" => Token::TypePin,

                    // V5.2: Async Types
                    "Future" => Token::TypeFuture,
                    "Stream" => Token::TypeStream,
                    "Task" => Token::TypeTask,

                    // V5.2: Time Types
                    "Duration" => Token::TypeDuration,
                    "Instant" => Token::TypeInstant,
                    "DateTime" => Token::TypeDateTime,

                    // V5.2: I/O Types
                    "File" => Token::TypeFile,
                    "TcpStream" => Token::TypeTcpStream,
                    "TcpListener" => Token::TypeTcpListener,
                    "UdpSocket" => Token::TypeUdpSocket,

                    // V5.2: Synchronization Types
                    "Mutex" => Token::TypeMutex,
                    "RwLock" => Token::TypeRwLock,
                    "Semaphore" => Token::TypeSemaphore,
                    "Barrier" => Token::TypeBarrier,

                    _ => Token::Identifier(ident),
                }
            }
            other => {
                return Err(LppoError::unexpected_char_at(
                    other as char,
                    self.current_token_location(),
                ))
            }
        };
        Ok(token)
    }

    /// Returns the source location for the last-produced token start.
    pub fn position(&self) -> crate::error::SourceLocation {
        self.current_token_location()
    }
}

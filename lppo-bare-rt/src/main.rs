#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};

extern crate alloc;
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use serde_json::Value;

const VGA_WIDTH: usize = 80;
const VGA_HEIGHT: usize = 25;
const VGA_MEMORY: *mut u16 = 0xB8000 as *mut u16;

const HEAP_START: usize = 0x200000;
const HEAP_SIZE: usize = 256 * 1024;

static HEAP_NEXT: AtomicUsize = AtomicUsize::new(HEAP_START);

struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align().max(1);
        let next = HEAP_NEXT.load(Ordering::Relaxed);
        let aligned = (next.wrapping_add(align).wrapping_sub(1)) & !(align - 1);
        let end = aligned.wrapping_add(size);
        if end > HEAP_START + HEAP_SIZE {
            panic!("allocation failed: bare metal heap exhausted");
        }
        HEAP_NEXT.store(end, Ordering::Relaxed);
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator: memory is reclaimed when the system resets.
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga_write_str("[panic] ");
    if let Some(location) = info.location() {
        vga_write_str(location.file());
        vga_write_str(":");
        vga_write_str(&i64_to_string(location.line() as i64));
        vga_write_str(":");
        vga_write_str(&i64_to_string(location.column() as i64));
        vga_write_str(" ");
    }
    vga_write_str("\n");
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

struct Cursor {
    x: usize,
    y: usize,
}

static mut CURSOR: Cursor = Cursor { x: 0, y: 0 };

fn vga_put_char(ch: char) {
    unsafe {
        match ch {
            '\n' => {
                CURSOR.x = 0;
                CURSOR.y += 1;
            }
            '\t' => {
                for _ in 0..4 {
                    vga_put_char(' ');
                }
            }
            ch => {
                let offset = (CURSOR.y * VGA_WIDTH + CURSOR.x) * 2;
                let byte = ch as u8;
                let color: u8 = 0x0F; // white on black
                core::ptr::write_volatile(
                    VGA_MEMORY.add(offset),
                    (byte as u16) | ((color as u16) << 8),
                );
                CURSOR.x += 1;
                if CURSOR.x >= VGA_WIDTH {
                    CURSOR.x = 0;
                    CURSOR.y += 1;
                }
            }
        }
        if CURSOR.y >= VGA_HEIGHT {
            vga_scroll();
            CURSOR.y = VGA_HEIGHT - 1;
        }
    }
}

fn vga_scroll() {
    unsafe {
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let src = ((row * VGA_WIDTH + col) * 2) as isize;
                let dst = (((row - 1) * VGA_WIDTH + col) * 2) as isize;
                let val = core::ptr::read_volatile(VGA_MEMORY.offset(src));
                core::ptr::write_volatile(VGA_MEMORY.offset(dst), val);
            }
        }
        for col in 0..VGA_WIDTH {
            let offset = (((VGA_HEIGHT - 1) * VGA_WIDTH + col) * 2) as isize;
            core::ptr::write_volatile(
                VGA_MEMORY.offset(offset),
                (0x20 as u16) | ((0x0F as u16) << 8),
            );
        }
    }
}

fn vga_write_str(s: &str) {
    for ch in s.chars() {
        vga_put_char(ch);
    }
}

fn vga_clear() {
    unsafe {
        for i in 0..(VGA_WIDTH * VGA_HEIGHT) {
            core::ptr::write_volatile(VGA_MEMORY.add(i), (0x20 as u16) | ((0x0F as u16) << 8));
        }
        CURSOR.x = 0;
        CURSOR.y = 0;
    }
}

fn i64_to_string(n: i64) -> String {
    if n == i64::MIN {
        return "-9223372036854775808".to_owned();
    }
    let mut s = String::new();
    let neg = n < 0;
    let mut m: u64 = if neg { (0i64 - n) as u64 } else { n as u64 };
    if neg {
        s.push('-');
    }
    if m == 0 {
        s.push('0');
    } else {
        let mut buf = [0u8; 20];
        let mut len = 0usize;
        while m > 0 {
            buf[len] = b'0' + (m % 10) as u8;
            m /= 10;
            len += 1;
        }
        while len > 0 {
            len -= 1;
            s.push(buf[len] as char);
        }
    }
    s
}

fn vga_write_value(val: &LppoValue) {
    match val {
        LppoValue::Int(i) => vga_write_str(&i64_to_string(*i)),
        LppoValue::Bool(b) => vga_write_str(if *b { "true" } else { "false" }),
        LppoValue::Str(s) => vga_write_str(s),
        LppoValue::Array(arr) => {
            vga_write_str("[");
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    vga_write_str(", ");
                }
                vga_write_value(item);
            }
            vga_write_str("]");
        }
    }
}

fn value_to_string(val: &LppoValue) -> String {
    match val {
        LppoValue::Int(i) => i64_to_string(*i),
        LppoValue::Bool(b) => {
            if *b {
                "true".to_owned()
            } else {
                "false".to_owned()
            }
        }
        LppoValue::Str(s) => s.clone(),
        LppoValue::Array(arr) => {
            let mut out = String::new();
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push_str(&value_to_string(item));
            }
            out
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LppoValue {
    Int(i64),
    Bool(bool),
    Str(String),
    Array(Vec<LppoValue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlFlow {
    None,
    Break,
    Continue,
    Return(Option<LppoValue>),
}

type LppoResult<T> = Result<T, String>;

#[derive(Clone)]
struct Procedure {
    params: Vec<String>,
    operations: Value,
}

fn truthy(val: &LppoValue) -> bool {
    match val {
        LppoValue::Int(i) => *i != 0,
        LppoValue::Bool(b) => *b,
        LppoValue::Str(s) => !s.is_empty(),
        LppoValue::Array(arr) => !arr.is_empty(),
    }
}

fn eval_expr(
    expr: &Value,
    env: &mut BTreeMap<String, LppoValue>,
    procs: &BTreeMap<String, Procedure>,
) -> LppoResult<LppoValue> {
    let obj = expr
        .as_object()
        .ok_or("expected an expression object".to_owned())?;
    let (tag, payload) = obj
        .iter()
        .next()
        .ok_or("empty expression object".to_owned())?;

    match tag.as_str() {
        "Int" => {
            let n = payload.as_i64().ok_or("Int expects a number".to_owned())?;
            Ok(LppoValue::Int(n))
        }
        "Str" => {
            let s = payload.as_str().ok_or("Str expects a string".to_owned())?;
            Ok(LppoValue::Str(s.to_owned()))
        }
        "Bool" => {
            let b = payload
                .as_bool()
                .ok_or("Bool expects a boolean".to_owned())?;
            Ok(LppoValue::Bool(b))
        }
        "Var" => {
            let name = payload.as_str().ok_or("Var expects a name".to_owned())?;
            env.get(name)
                .cloned()
                .ok_or_else(|| format_unknown_var(name))
        }
        "BinaryOp" => {
            let bo = payload
                .as_object()
                .ok_or("BinaryOp expects an object".to_owned())?;
            let left = eval_expr(
                bo.get("left").ok_or("BinaryOp missing left".to_owned())?,
                env,
                procs,
            )?;
            let right = eval_expr(
                bo.get("right").ok_or("BinaryOp missing right".to_owned())?,
                env,
                procs,
            )?;
            let op = bo
                .get("op")
                .and_then(|o| o.as_str())
                .ok_or("BinaryOp missing op".to_owned())?;
            apply_binary_op(op, &left, &right)
        }
        "UnaryOp" => {
            let uo = payload
                .as_object()
                .ok_or("UnaryOp expects an object".to_owned())?;
            let operand = eval_expr(
                uo.get("operand")
                    .ok_or("UnaryOp missing operand".to_owned())?,
                env,
                procs,
            )?;
            let op = uo
                .get("op")
                .and_then(|o| o.as_str())
                .ok_or("UnaryOp missing op".to_owned())?;
            apply_unary_op(op, &operand)
        }
        "Array" => {
            let arr = payload
                .as_array()
                .ok_or("Array expects an array".to_owned())?;
            let mut items = Vec::new();
            for e in arr {
                items.push(eval_expr(e, env, procs)?);
            }
            Ok(LppoValue::Array(items))
        }
        "Call" => {
            let call_obj = payload
                .as_object()
                .ok_or("Call expects an object".to_owned())?;
            let func = call_obj
                .get("func")
                .and_then(|f| f.as_str())
                .ok_or("Call missing func".to_owned())?;
            let args = call_obj
                .get("args")
                .and_then(|a| a.as_array())
                .ok_or("Call missing args".to_owned())?;

            match func {
                "outb" => {
                    if args.len() != 2 {
                        return Err("outb requires 2 arguments (port, value)".to_owned());
                    }
                    let port = eval_expr(&args[0], env, procs)?;
                    let val = eval_expr(&args[1], env, procs)?;
                    match (port, val) {
                        (LppoValue::Int(port), LppoValue::Int(val)) => unsafe {
                            core::arch::asm!("outb %al, %dx",
                                    in("al") (val as u8),
                                    in("dx") (port as u16),
                                    options(nostack, att_syntax));
                        },
                        _ => return Err("outb requires integer arguments".to_owned()),
                    }
                    Ok(LppoValue::Int(0))
                }
                "outw" => {
                    if args.len() != 2 {
                        return Err("outw requires 2 arguments (port, value)".to_owned());
                    }
                    let port = eval_expr(&args[0], env, procs)?;
                    let val = eval_expr(&args[1], env, procs)?;
                    match (port, val) {
                        (LppoValue::Int(port), LppoValue::Int(val)) => unsafe {
                            core::arch::asm!("outw %ax, %dx",
                                    in("ax") (val as u16),
                                    in("dx") (port as u16),
                                    options(nostack, att_syntax));
                        },
                        _ => return Err("outw requires integer arguments".to_owned()),
                    }
                    Ok(LppoValue::Int(0))
                }
                "outl" => {
                    if args.len() != 2 {
                        return Err("outl requires 2 arguments (port, value)".to_owned());
                    }
                    let port = eval_expr(&args[0], env, procs)?;
                    let val = eval_expr(&args[1], env, procs)?;
                    match (port, val) {
                        (LppoValue::Int(port), LppoValue::Int(val)) => unsafe {
                            core::arch::asm!("outl %eax, %dx",
                                    in("eax") (val as u32),
                                    in("dx") (port as u16),
                                    options(nostack, att_syntax));
                        },
                        _ => return Err("outl requires integer arguments".to_owned()),
                    }
                    Ok(LppoValue::Int(0))
                }
                "inb" => {
                    if args.len() != 1 {
                        return Err("inb requires 1 argument (port)".to_owned());
                    }
                    let port = eval_expr(&args[0], env, procs)?;
                    let port = match port {
                        LppoValue::Int(p) => p as u16,
                        _ => return Err("inb requires integer port".to_owned()),
                    };
                    let val: u8;
                    unsafe {
                        core::arch::asm!("inb %dx, %al",
                            out("al") val,
                            in("dx") port,
                            options(nostack, att_syntax));
                    }
                    Ok(LppoValue::Int(val as i64))
                }
                "inw" => {
                    if args.len() != 1 {
                        return Err("inw requires 1 argument (port)".to_owned());
                    }
                    let port = eval_expr(&args[0], env, procs)?;
                    let port = match port {
                        LppoValue::Int(p) => p as u16,
                        _ => return Err("inw requires integer port".to_owned()),
                    };
                    let val: u16;
                    unsafe {
                        core::arch::asm!("inw %dx, %ax",
                            out("ax") val,
                            in("dx") port,
                            options(nostack, att_syntax));
                    }
                    Ok(LppoValue::Int(val as i64))
                }
                "inl" => {
                    if args.len() != 1 {
                        return Err("inl requires 1 argument (port)".to_owned());
                    }
                    let port = eval_expr(&args[0], env, procs)?;
                    let port = match port {
                        LppoValue::Int(p) => p as u16,
                        _ => return Err("inl requires integer port".to_owned()),
                    };
                    let val: u32;
                    unsafe {
                        core::arch::asm!("inl %dx, %eax",
                            out("eax") val,
                            in("dx") port,
                            options(nostack, att_syntax));
                    }
                    Ok(LppoValue::Int(val as i64))
                }
                "rdmsr" => {
                    if args.len() != 1 {
                        return Err("rdmsr requires 1 argument (ecx)".to_owned());
                    }
                    let ecx = eval_expr(&args[0], env, procs)?;
                    let ecx = match ecx {
                        LppoValue::Int(c) => c as u32,
                        _ => return Err("rdmsr requires integer ecx".to_owned()),
                    };
                    let eax: u32;
                    let edx: u32;
                    unsafe {
                        core::arch::asm!("rdmsr",
                            in("ecx") ecx,
                            out("eax") eax,
                            out("edx") edx,
                            options(nostack));
                    }
                    Ok(LppoValue::Int(((edx as u64) << 32 | eax as u64) as i64))
                }
                "wrmsr" => {
                    if args.len() != 2 {
                        return Err("wrmsr requires 2 arguments (ecx, value)".to_owned());
                    }
                    let ecx = eval_expr(&args[0], env, procs)?;
                    let val = eval_expr(&args[1], env, procs)?;
                    match (ecx, val) {
                        (LppoValue::Int(c), LppoValue::Int(v)) => unsafe {
                            core::arch::asm!("wrmsr",
                                    in("ecx") c,
                                    in("eax") (v as u32),
                                    in("edx") ((v >> 32) as u32),
                                    options(nostack));
                        },
                        _ => return Err("wrmsr requires integer arguments".to_owned()),
                    }
                    Ok(LppoValue::Int(0))
                }
                "cpuid" => {
                    if args.len() != 1 {
                        return Err("cpuid requires 1 argument (eax)".to_owned());
                    }
                    let eax_in = eval_expr(&args[0], env, procs)?;
                    let eax_in = match eax_in {
                        LppoValue::Int(e) => e as u32,
                        _ => return Err("cpuid requires integer eax".to_owned()),
                    };
                    let mut eax: u32 = eax_in;
                    let mut cpuid_ebx: u32 = 0;
                    let mut ecx: u32 = 0;
                    let mut edx: u32 = 0;
                    unsafe {
                        core::arch::asm!(
                            "cpuid",
                            inout("eax") eax,
                            out("ecx") ecx,
                            out("edx") edx,
                            options(nostack)
                        );
                        // rbx is reserved by LLVM, so read it via a hardcoded register name.
                        core::arch::asm!("mov %ebx, %eax", out("eax") cpuid_ebx, options(nostack, att_syntax));
                    }
                    Ok(LppoValue::Array(vec![
                        LppoValue::Int(eax as i64),
                        LppoValue::Int(cpuid_ebx as i64),
                        LppoValue::Int(ecx as i64),
                        LppoValue::Int(edx as i64),
                    ]))
                }
                "cli" => {
                    if args.len() != 0 {
                        return Err("cli takes no arguments".to_owned());
                    }
                    unsafe {
                        core::arch::asm!("cli", options(nostack));
                    }
                    Ok(LppoValue::Int(0))
                }
                "sti" => {
                    if args.len() != 0 {
                        return Err("sti takes no arguments".to_owned());
                    }
                    unsafe {
                        core::arch::asm!("sti", options(nostack));
                    }
                    Ok(LppoValue::Int(0))
                }
                "hlt" => {
                    if args.len() != 0 {
                        return Err("hlt takes no arguments".to_owned());
                    }
                    unsafe {
                        core::arch::asm!("hlt", options(nostack));
                    }
                    Ok(LppoValue::Int(0))
                }
                _ => return Err(format!("unsupported call target: {}", func)),
            }
        }
        _ => Err(format_unsupported_expr(tag)),
    }
}

fn format_unknown_var(name: &str) -> String {
    let mut s = String::new();
    s.push_str("unknown variable '");
    s.push_str(name);
    s.push_str("'");
    s
}

fn format_unsupported_expr(tag: &str) -> String {
    let mut s = String::new();
    s.push_str("expression not supported in bare metal: ");
    s.push_str(tag);
    s
}

fn apply_binary_op(op: &str, left: &LppoValue, right: &LppoValue) -> LppoResult<LppoValue> {
    match (op, left, right) {
        ("Add", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Int(a.wrapping_add(*b))),
        ("Sub", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Int(a.wrapping_sub(*b))),
        ("Mul", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Int(a.wrapping_mul(*b))),
        ("Div", LppoValue::Int(a), LppoValue::Int(b)) => {
            if *b == 0 {
                Err("division by zero".to_owned())
            } else {
                Ok(LppoValue::Int(a.wrapping_div(*b)))
            }
        }
        ("Mod", LppoValue::Int(a), LppoValue::Int(b)) => {
            if *b == 0 {
                Err("modulo by zero".to_owned())
            } else {
                Ok(LppoValue::Int(a.wrapping_rem(*b)))
            }
        }
        ("Eq", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Bool(a == b)),
        ("Ne", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Bool(a != b)),
        ("Lt", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Bool(a < b)),
        ("Le", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Bool(a <= b)),
        ("Gt", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Bool(a > b)),
        ("Ge", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Bool(a >= b)),
        ("Add", LppoValue::Str(a), LppoValue::Str(b)) => {
            let mut s = String::new();
            s.push_str(a);
            s.push_str(b);
            Ok(LppoValue::Str(s))
        }
        ("Eq", LppoValue::Str(a), LppoValue::Str(b)) => Ok(LppoValue::Bool(a == b)),
        ("Ne", LppoValue::Str(a), LppoValue::Str(b)) => Ok(LppoValue::Bool(a != b)),
        ("And", LppoValue::Bool(a), LppoValue::Bool(b)) => Ok(LppoValue::Bool(*a && *b)),
        ("Or", LppoValue::Bool(a), LppoValue::Bool(b)) => Ok(LppoValue::Bool(*a || *b)),
        ("BitAnd", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Int(a & b)),
        ("BitOr", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Int(a | b)),
        ("BitXor", LppoValue::Int(a), LppoValue::Int(b)) => Ok(LppoValue::Int(a ^ b)),
        _ => Err(format_unsupported_binop(op)),
    }
}

fn format_unsupported_binop(op: &str) -> String {
    let mut s = String::new();
    s.push_str("binary operation not supported in bare metal: ");
    s.push_str(op);
    s
}

fn apply_unary_op(op: &str, operand: &LppoValue) -> LppoResult<LppoValue> {
    match (op, operand) {
        ("Neg", LppoValue::Int(i)) => Ok(LppoValue::Int(i.wrapping_neg())),
        ("Not", LppoValue::Bool(b)) => Ok(LppoValue::Bool(!b)),
        ("BitNot", LppoValue::Int(i)) => Ok(LppoValue::Int(!i)),
        _ => Err(format_unsupported_unaryop(op)),
    }
}

fn format_unsupported_unaryop(op: &str) -> String {
    let mut s = String::new();
    s.push_str("unary operation not supported in bare metal: ");
    s.push_str(op);
    s
}

fn execute_ops(
    ops: &Value,
    env: &mut BTreeMap<String, LppoValue>,
    procs: &BTreeMap<String, Procedure>,
) -> LppoResult<ControlFlow> {
    let arr = ops
        .as_array()
        .ok_or("operations must be an array".to_owned())?;
    for op in arr {
        let obj = op
            .as_object()
            .ok_or("expected an operation object".to_owned())?;
        let (tag, payload) = obj
            .iter()
            .next()
            .ok_or("empty operation object".to_owned())?;

        match tag.as_str() {
            "Nop" => {}
            "Emit" => {
                let arr = payload
                    .as_array()
                    .ok_or("Emit expects an array of expressions".to_owned())?;
                let mut parts = Vec::new();
                for e in arr {
                    parts.push(value_to_string(&eval_expr(e, env, procs)?));
                }
                vga_write_str(&parts.join(" "));
                vga_write_str("\n");
            }
            "Let" => {
                let pair = payload
                    .as_array()
                    .ok_or("Let expects [name, expr]".to_owned())?;
                if pair.len() != 2 {
                    return Err("Let expects exactly [name, expr]".to_owned());
                }
                let name = pair[0]
                    .as_str()
                    .ok_or("Let name must be a string".to_owned())?;
                let val = eval_expr(&pair[1], env, procs)?;
                env.insert(name.to_owned(), val);
            }
            "Assignment" => {
                let obj = payload
                    .as_object()
                    .ok_or("Assignment expects an object".to_owned())?;
                let target = obj
                    .get("target")
                    .and_then(|t| t.as_str())
                    .ok_or("Assignment missing target".to_owned())?;
                let val = eval_expr(
                    obj.get("value")
                        .ok_or("Assignment missing value".to_owned())?,
                    env,
                    procs,
                )?;
                env.insert(target.to_owned(), val);
            }
            "ExprStatement" => {
                let _ = eval_expr(payload, env, procs)?;
            }
            "Call" => {
                let pair = payload
                    .as_array()
                    .ok_or("Call expects [name, args]".to_owned())?;
                if pair.len() != 2 {
                    return Err("Call expects exactly [name, args]".to_owned());
                }
                let name = pair[0]
                    .as_str()
                    .ok_or("Call name must be a string".to_owned())?;
                let args = pair[1]
                    .as_array()
                    .ok_or("Call args must be an array".to_owned())?;
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(eval_expr(a, env, procs)?);
                }
                let proc = procs.get(name).ok_or_else(|| {
                    let mut s = String::new();
                    s.push_str("unknown procedure '");
                    s.push_str(name);
                    s.push_str("'");
                    s
                })?;
                if proc.params.len() != arg_vals.len() {
                    let mut s = String::new();
                    s.push_str("argument count mismatch for '");
                    s.push_str(name);
                    s.push_str("'");
                    return Err(s);
                }
                let mut call_env = BTreeMap::new();
                for (p, v) in proc.params.iter().zip(arg_vals.into_iter()) {
                    call_env.insert(p.clone(), v);
                }
                match execute_ops(&proc.operations, &mut call_env, procs)? {
                    ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                    ControlFlow::Break | ControlFlow::Continue => {
                        let mut s = String::new();
                        s.push_str("break/continue outside loop in '");
                        s.push_str(name);
                        s.push_str("'");
                        return Err(s);
                    }
                    ControlFlow::None => {}
                }
            }
            "If" => {
                let cond_obj = payload
                    .as_object()
                    .ok_or("If expects an object".to_owned())?;
                let cond = eval_expr(
                    cond_obj.get("cond").ok_or("If missing cond".to_owned())?,
                    env,
                    procs,
                )?;
                if truthy(&cond) {
                    let then_ops = cond_obj
                        .get("then_ops")
                        .ok_or("If missing then_ops".to_owned())?;
                    return execute_ops(then_ops, env, procs);
                }
                if let Some(elifs) = cond_obj.get("elifs").and_then(|e| e.as_array()) {
                    for elif in elifs {
                        let pair = elif
                            .as_array()
                            .ok_or("elif must be [cond, ops]".to_owned())?;
                        if pair.len() != 2 {
                            return Err("elif must be exactly [cond, ops]".to_owned());
                        }
                        let ev = eval_expr(&pair[0], env, procs)?;
                        if truthy(&ev) {
                            return execute_ops(&pair[1], env, procs);
                        }
                    }
                }
                if let Some(else_ops) = cond_obj.get("else_ops") {
                    return execute_ops(else_ops, env, procs);
                }
            }
            "While" => {
                let wo = payload
                    .as_object()
                    .ok_or("While expects an object".to_owned())?;
                loop {
                    let cond = eval_expr(
                        wo.get("cond").ok_or("While missing cond".to_owned())?,
                        env,
                        procs,
                    )?;
                    if !truthy(&cond) {
                        break;
                    }
                    match execute_ops(
                        wo.get("ops").ok_or("While missing ops".to_owned())?,
                        env,
                        procs,
                    )? {
                        ControlFlow::None => continue,
                        ControlFlow::Break => break,
                        ControlFlow::Continue => continue,
                        ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                    }
                }
            }
            "Loop" => match execute_ops(payload, env, procs)? {
                ControlFlow::None => continue,
                ControlFlow::Break => break,
                ControlFlow::Continue => continue,
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
            },
            "Break" => return Ok(ControlFlow::Break),
            "Continue" => return Ok(ControlFlow::Continue),
            "Return" => {
                let v = if payload.is_null() {
                    None
                } else {
                    Some(eval_expr(payload, env, procs)?)
                };
                return Ok(ControlFlow::Return(v));
            }
            "Scope" | "Unsafe" => match execute_ops(payload, env, procs)? {
                ControlFlow::None => {}
                cf => return Ok(cf),
            },
            "Input" | "Pause" | "Parallel" | "For" | "IndexedAssignment" | "Match"
            | "StructInit" => {
                let mut s = String::new();
                s.push_str("statement not supported in bare metal: ");
                s.push_str(tag);
                return Err(s);
            }
            _ => {
                let mut s = String::new();
                s.push_str("unknown operation: ");
                s.push_str(tag);
                return Err(s);
            }
        }
    }
    Ok(ControlFlow::None)
}

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

extern "C" {
    static _stack_top: usize;
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        core::arch::asm!("mov rsp, {0}", in(reg) _stack_top, options(nostack));
    }
    kmain();
}

fn kmain() -> ! {
    vga_clear();
    vga_write_str("LPPO bare metal runtime v5.3\n");
    vga_write_str("----------------------------------------\n");

    let data: Value = match serde_json::from_slice(BYTECODE) {
        Ok(d) => d,
        Err(_) => {
            vga_write_str("failed to parse LPPO bytecode (invalid JSON)\n");
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    let root = match data.as_object() {
        Some(r) => r,
        None => {
            vga_write_str("bytecode must be a JSON object\n");
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    let mut procs: BTreeMap<String, Procedure> = BTreeMap::new();
    for (name, proc_val) in root {
        let params = proc_val
            .get("params")
            .and_then(|p| p.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|p| p.as_str().map(|s| s.to_owned()))
                    .collect()
            })
            .unwrap_or_default();
        let operations = proc_val
            .get("operations")
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        procs.insert(name.clone(), Procedure { params, operations });
    }

    let main_proc = match procs.get("main") {
        Some(p) => p,
        None => {
            vga_write_str("no 'main' procedure in bytecode\n");
            loop {
                unsafe {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    let mut env: BTreeMap<String, LppoValue> = BTreeMap::new();
    match execute_ops(&main_proc.operations, &mut env, &procs) {
        Ok(ControlFlow::None) => {
            vga_write_str("\n[program finished]\n");
        }
        Ok(ControlFlow::Return(Some(v))) => {
            vga_write_str("\n[return: ");
            vga_write_value(&v);
            vga_write_str("]\n");
        }
        Ok(ControlFlow::Return(None)) => {
            vga_write_str("\n[return]\n");
        }
        Ok(ControlFlow::Break) => vga_write_str("\n[break]\n"),
        Ok(ControlFlow::Continue) => vga_write_str("\n[continue]\n"),
        Err(e) => {
            vga_write_str("\n[runtime error: ");
            vga_write_str(&e);
            vga_write_str("]\n");
        }
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

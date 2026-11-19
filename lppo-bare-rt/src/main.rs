#![no_std]
#![no_main]

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};

extern crate alloc;
use serde_json::Value;

mod arch;
mod drivers;

use arch::gdt;
use arch::idt;
use arch::io;
use arch::multiboot::{boot_memory_regions, MemRegion, MULTIBOOT_MEMORY_AVAILABLE};
use arch::paging;
use arch::pmm;
use drivers::keyboard;
use drivers::serial;
use drivers::vga;

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
    vga::vga_write_str("[panic] ");
    if let Some(location) = info.location() {
        vga::vga_write_str(location.file());
        vga::vga_write_str(":");
        vga::vga_write_str(&vga::i64_to_string(location.line() as i64));
        vga::vga_write_str(":");
        vga::vga_write_str(&vga::i64_to_string(location.column() as i64));
        vga::vga_write_str(" ");
    }
    vga::vga_write_str("\n");
    serial::write_str("[panic] ");
    if let Some(location) = info.location() {
        serial::write_str(location.file());
        serial::write_str(":");
        serial::write_str(&vga::i64_to_string(location.line() as i64));
        serial::write_str(":");
        serial::write_str(&vga::i64_to_string(location.column() as i64));
        serial::write_str(" ");
    }
    serial::write_str("\n");
    loop {
        unsafe {
            core::arch::asm!("hlt");
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

fn vga_write_value(val: &LppoValue) {
    match val {
        LppoValue::Int(i) => vga::vga_write_str(&vga::i64_to_string(*i)),
        LppoValue::Bool(b) => vga::vga_write_str(if *b { "true" } else { "false" }),
        LppoValue::Str(s) => vga::vga_write_str(s),
        LppoValue::Array(arr) => {
            vga::vga_write_str("[");
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    vga::vga_write_str(", ");
                }
                vga_write_value(item);
            }
            vga::vga_write_str("]");
        }
    }
}

fn serial_write_value(val: &LppoValue) {
    match val {
        LppoValue::Int(i) => serial::write_str(&vga::i64_to_string(*i)),
        LppoValue::Bool(b) => serial::write_str(if *b { "true" } else { "false" }),
        LppoValue::Str(s) => serial::write_str(s),
        LppoValue::Array(arr) => {
            serial::write_str("[");
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    serial::write_str(", ");
                }
                serial_write_value(item);
            }
            serial::write_str("]");
        }
    }
}

fn value_to_string(val: &LppoValue) -> String {
    match val {
        LppoValue::Int(i) => vga::i64_to_string(*i),
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

fn as_int(val: &LppoValue) -> LppoResult<i64> {
    match val {
        LppoValue::Int(i) => Ok(*i),
        _ => Err("expected an integer argument".to_owned()),
    }
}

fn eval_expr(
    expr: &Value,
    env: &mut BTreeMap<String, LppoValue>,
    procs: &BTreeMap<String, Procedure>,
) -> LppoResult<LppoValue> {
    use alloc::collections::BTreeMap;
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
        "Index" => {
            let idx_obj = payload
                .as_object()
                .ok_or("Index expects an object".to_owned())?;
            let target = eval_expr(
                idx_obj.get("target").ok_or("Index missing target".to_owned())?,
                env,
                procs,
            )?;
            let index = eval_expr(
                idx_obj.get("index").ok_or("Index missing index".to_owned())?,
                env,
                procs,
            )?;
            let i = match &index {
                LppoValue::Int(n) => *n as usize,
                _ => return Err("Index index must be an integer".to_owned()),
            };
            match &target {
                LppoValue::Array(arr) => arr
                    .get(i)
                    .cloned()
                    .ok_or("Index out of bounds".to_owned()),
                _ => Err("Index target must be an array".to_owned()),
            }
        }
        "Call" => {
            let call_obj = payload
                .as_object()
                .ok_or("Call expects an object".to_owned())?;
            let func_expr = call_obj
                .get("func")
                .ok_or("Call missing func".to_owned())?;
            let args_exprs = call_obj
                .get("args")
                .and_then(|a| a.as_array())
                .ok_or("Call missing args".to_owned())?;
            let func_name = match func_expr {
                Value::Object(o) => match o.iter().next() {
                    Some((key, v)) if key == "Var" => v
                        .as_str()
                        .ok_or("Call func Var must be a string".to_owned())?,
                    _ => return Err("Call func must be a Var expression".to_owned()),
                },
                _ => return Err("Call func must be a Var expression".to_owned()),
            };
            eval_call(func_name, args_exprs, env, procs)
        }
        _ => Err(format_unsupported_expr(tag)),
    }
}

fn eval_call(
    func: &str,
    args: &[Value],
    env: &mut BTreeMap<String, LppoValue>,
    procs: &BTreeMap<String, Procedure>,
) -> LppoResult<LppoValue> {
    use alloc::collections::BTreeMap;
    match func {
        "outb" => {
            if args.len() != 2 {
                return Err("outb requires 2 arguments (port, value)".to_owned());
            }
            let port = as_int(&eval_expr(&args[0], env, procs)?)? as u16;
            let val = as_int(&eval_expr(&args[1], env, procs)?)? as u8;
            unsafe { io::outb(port, val); }
            Ok(LppoValue::Int(0))
        }
        "outw" => {
            if args.len() != 2 {
                return Err("outw requires 2 arguments (port, value)".to_owned());
            }
            let port = as_int(&eval_expr(&args[0], env, procs)?)? as u16;
            let val = as_int(&eval_expr(&args[1], env, procs)?)? as u16;
            unsafe { io::outw(port, val); }
            Ok(LppoValue::Int(0))
        }
        "outl" => {
            if args.len() != 2 {
                return Err("outl requires 2 arguments (port, value)".to_owned());
            }
            let port = as_int(&eval_expr(&args[0], env, procs)?)? as u16;
            let val = as_int(&eval_expr(&args[1], env, procs)?)? as u32;
            unsafe { io::outl(port, val); }
            Ok(LppoValue::Int(0))
        }
        "inb" => {
            if args.len() != 1 {
                return Err("inb requires 1 argument (port)".to_owned());
            }
            let port = as_int(&eval_expr(&args[0], env, procs)?)? as u16;
            let val = unsafe { io::inb(port) };
            Ok(LppoValue::Int(val as i64))
        }
        "inw" => {
            if args.len() != 1 {
                return Err("inw requires 1 argument (port)".to_owned());
            }
            let port = as_int(&eval_expr(&args[0], env, procs)?)? as u16;
            let val = unsafe { io::inw(port) };
            Ok(LppoValue::Int(val as i64))
        }
        "inl" => {
            if args.len() != 1 {
                return Err("inl requires 1 argument (port)".to_owned());
            }
            let port = as_int(&eval_expr(&args[0], env, procs)?)? as u16;
            let val = unsafe { io::inl(port) };
            Ok(LppoValue::Int(val as i64))
        }
        "io_wait" => {
            if args.len() != 0 {
                return Err("io_wait takes no arguments".to_owned());
            }
            unsafe { io::io_wait(); }
            Ok(LppoValue::Int(0))
        }
        "rdmsr" => {
            if args.len() != 1 {
                return Err("rdmsr requires 1 argument (ecx)".to_owned());
            }
            let ecx = as_int(&eval_expr(&args[0], env, procs)?)? as u32;
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
            let ecx = as_int(&eval_expr(&args[0], env, procs)?)? as u32;
            let val = as_int(&eval_expr(&args[1], env, procs)?)?;
            unsafe {
                core::arch::asm!("wrmsr",
                        in("ecx") ecx,
                        in("eax") (val as u32),
                        in("edx") ((val >> 32) as u32),
                        options(nostack));
            }
            Ok(LppoValue::Int(0))
        }
        "cpuid" => {
            if args.len() != 1 {
                return Err("cpuid requires 1 argument (eax)".to_owned());
            }
            let eax_in = as_int(&eval_expr(&args[0], env, procs)?)? as u32;
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
                core::arch::asm!("mov %ebx, %eax", out("eax") cpuid_ebx, options(nostack, att_syntax));
            }
            let mut arr = Vec::new();
            arr.push(LppoValue::Int(eax as i64));
            arr.push(LppoValue::Int(cpuid_ebx as i64));
            arr.push(LppoValue::Int(ecx as i64));
            arr.push(LppoValue::Int(edx as i64));
            Ok(LppoValue::Array(arr))
        }
        "cli" => {
            if args.len() != 0 {
                return Err("cli takes no arguments".to_owned());
            }
            unsafe { core::arch::asm!("cli", options(nostack)); }
            Ok(LppoValue::Int(0))
        }
        "sti" => {
            if args.len() != 0 {
                return Err("sti takes no arguments".to_owned());
            }
            unsafe { core::arch::asm!("sti", options(nostack)); }
            Ok(LppoValue::Int(0))
        }
        "hlt" => {
            if args.len() != 0 {
                return Err("hlt takes no arguments".to_owned());
            }
            unsafe { core::arch::asm!("hlt", options(nostack)); }
            Ok(LppoValue::Int(0))
        }
        // ---- kernel built-ins ----
        "lgdt" => {
            if args.len() != 0 {
                return Err("lgdt takes no arguments".to_owned());
            }
            gdt::init();
            Ok(LppoValue::Int(0))
        }
        "lidt" => {
            if args.len() != 0 {
                return Err("lidt takes no arguments".to_owned());
            }
            idt::init();
            Ok(LppoValue::Int(0))
        }
        "idt_set" => {
            if args.len() != 2 {
                return Err("idt_set requires 2 arguments (index, handler_addr)".to_owned());
            }
            let index = as_int(&eval_expr(&args[0], env, procs)?)? as u16;
            let handler = as_int(&eval_expr(&args[1], env, procs)?)? as u64;
            idt::idt_set(index, handler);
            Ok(LppoValue::Int(0))
        }
        "pic_remap" => {
            if args.len() != 0 {
                return Err("pic_remap takes no arguments".to_owned());
            }
            idt::pic_remap();
            Ok(LppoValue::Int(0))
        }
        "pic_eoi" => {
            if args.len() != 1 {
                return Err("pic_eoi requires 1 argument (irq)".to_owned());
            }
            let irq = as_int(&eval_expr(&args[0], env, procs)?)? as u8;
            idt::pic_eoi(irq);
            Ok(LppoValue::Int(0))
        }
        "alloc_frame" => {
            if args.len() != 0 {
                return Err("alloc_frame takes no arguments".to_owned());
            }
            match pmm::alloc_frame() {
                Some(addr) => Ok(LppoValue::Int(addr as i64)),
                None => Err("out of physical memory".to_owned()),
            }
        }
        "free_frame" => {
            if args.len() != 1 {
                return Err("free_frame requires 1 argument (address)".to_owned());
            }
            let addr = as_int(&eval_expr(&args[0], env, procs)?)? as u64;
            pmm::free_frame(addr);
            Ok(LppoValue::Int(0))
        }
        "map_page" => {
            if args.len() != 2 {
                return Err("map_page requires 2 arguments (vaddr, paddr)".to_owned());
            }
            let vaddr = as_int(&eval_expr(&args[0], env, procs)?)? as u64;
            let paddr = as_int(&eval_expr(&args[1], env, procs)?)? as u64;
            paging::map_page(vaddr, paddr).map_err(|e| e.to_owned())?;
            Ok(LppoValue::Int(0))
        }
        "enable_paging" => {
            if args.len() != 0 {
                return Err("enable_paging takes no arguments".to_owned());
            }
            paging::enable_paging();
            Ok(LppoValue::Int(0))
        }
        "serial_init" => {
            if args.len() != 0 {
                return Err("serial_init takes no arguments".to_owned());
            }
            serial::init();
            Ok(LppoValue::Int(0))
        }
        "serial_write" => {
            if args.len() != 1 {
                return Err("serial_write requires 1 argument (byte)".to_owned());
            }
            let byte = as_int(&eval_expr(&args[0], env, procs)?)? as u8;
            serial::write_byte(byte);
            Ok(LppoValue::Int(0))
        }
        "serial_write_str" => {
            if args.len() != 1 {
                return Err("serial_write_str requires 1 argument (string)".to_owned());
            }
            let val = eval_expr(&args[0], env, procs)?;
            if let LppoValue::Str(s) = val {
                serial::write_str(&s);
            } else {
                serial::write_str(&value_to_string(&val));
            }
            Ok(LppoValue::Int(0))
        }
        "serial_write_hex" => {
            if args.len() != 1 {
                return Err("serial_write_hex requires 1 argument (value)".to_owned());
            }
            let val = as_int(&eval_expr(&args[0], env, procs)?)? as u64;
            serial::write_hex(val);
            serial::write_str("\n");
            Ok(LppoValue::Int(0))
        }
        "keyboard_init" => {
            if args.len() != 0 {
                return Err("keyboard_init takes no arguments".to_owned());
            }
            keyboard::init();
            Ok(LppoValue::Int(0))
        }
        "keyboard_poll_scan" => {
            if args.len() != 0 {
                return Err("keyboard_poll_scan takes no arguments".to_owned());
            }
            match keyboard::poll_scan() {
                Some(sc) => Ok(LppoValue::Int(sc as i64)),
                None => Ok(LppoValue::Int(-1)),
            }
        }
        "keyboard_wait_scan" => {
            if args.len() != 0 {
                return Err("keyboard_wait_scan takes no arguments".to_owned());
            }
            Ok(LppoValue::Int(keyboard::wait_scan() as i64))
        }
        "vga_clear" => {
            if args.len() != 0 {
                return Err("vga_clear takes no arguments".to_owned());
            }
            vga::vga_clear();
            Ok(LppoValue::Int(0))
        }
        _ => {
            // Fall back to a user-defined procedure call.
            let name = func.to_owned();
            let mut arg_vals = Vec::new();
            for a in args {
                arg_vals.push(eval_expr(a, env, procs)?);
            }
            let proc = procs.get(&name).ok_or_else(|| {
                let mut s = String::new();
                s.push_str("unknown procedure or built-in '");
                s.push_str(&name);
                s.push_str("'");
                s
            })?;
            if proc.params.len() != arg_vals.len() {
                let mut s = String::new();
                s.push_str("argument count mismatch for '");
                s.push_str(&name);
                s.push_str("'");
                return Err(s);
            }
            let mut call_env = BTreeMap::new();
            for (p, v) in proc.params.iter().zip(arg_vals.into_iter()) {
                call_env.insert(p.clone(), v);
            }
            match execute_ops(&proc.operations, &mut call_env, procs)? {
                ControlFlow::Return(v) => Ok(v.unwrap_or(LppoValue::Int(0))),
                ControlFlow::Break | ControlFlow::Continue => {
                    Err("break/continue outside loop".to_owned())
                }
                ControlFlow::None => Ok(LppoValue::Int(0)),
            }
        }
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
    use alloc::collections::BTreeMap;
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
                vga::vga_write_str(&parts.join(" "));
                vga::vga_write_str("\n");
                serial::write_str(&parts.join(" "));
                serial::write_str("\n");
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
                return eval_call(name, args, env, procs).map(|_| ControlFlow::None);
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
                        .get("then_ops").ok_or("If missing then_ops".to_owned())?;
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

// rust_kmain is the entry point called from the assembly bootstrap (boot.S)
// once the CPU is in 64-bit long mode with paging enabled.
#[no_mangle]
pub extern "C" fn kmain() -> ! {
    // Running in 64-bit long mode at this point (boot.S switched modes).
    vga::vga_clear();
    serial::init();
    serial::write_str("kmain: entered 64-bit long mode\n");
    vga::vga_write_str("LPPO bare metal runtime (kernel) v5.3\n");
    serial::write_str("LPPO bare metal runtime (kernel) v5.3\n");
    vga::vga_write_str("LPPO bare metal runtime (kernel) v5.3\n");
    serial::write_str("LPPO bare metal runtime (kernel) v5.3\n");

    serial::write_str("kernel: running in 64-bit long mode\n");
    vga::vga_write_str("kernel: running in 64-bit long mode\n");

    // Parse the Multiboot2 boot info (best-effort; RBX holds the boot-info ptr
    // when booted by a multiboot2 bootloader such as GRUB).
    let regions = boot_memory_regions();

    // Initialise the physical memory manager.
    pmm::init();

    // Reserve the kernel image and the early heap so the frame allocator
    // never hands them out. The kernel is loaded at 1 MiB; _end marks its end.
    extern "C" {
        static _end: usize;
    }
    let kernel_end = unsafe { _end as u64 };
    pmm::reserve(0x100000, kernel_end);
    pmm::reserve(HEAP_START as u64, HEAP_START as u64 + HEAP_SIZE as u64);

    // Reserve any non-available (reserved/ACPI/NVS/bad) regions from the map.
    for r in regions.iter() {
        if r.r#type != MULTIBOOT_MEMORY_AVAILABLE {
            pmm::reserve(r.addr, r.addr.wrapping_add(r.len));
        }
    }

    // GDT and IDT are set up by boot.S; (re)initialise them here for clarity.
    // Both are idempotent.
    gdt::init();
    idt::init();

    serial::write_str("kernel: GDT, IDT initialised\n");
    vga::vga_write_str("kernel: GDT, IDT initialised\n");

    let data: Value = match serde_json::from_slice(BYTECODE) {
        Ok(d) => d,
        Err(_) => {
            vga::vga_write_str("failed to parse LPPO bytecode (invalid JSON)\n");
            serial::write_str("failed to parse LPPO bytecode\n");
            loop {
                unsafe { core::arch::asm!("hlt"); }
            }
        }
    };

    let root = match data.as_object() {
        Some(r) => r,
        None => {
            vga::vga_write_str("bytecode must be a JSON object\n");
            serial::write_str("bytecode must be a JSON object\n");
            loop {
                unsafe { core::arch::asm!("hlt"); }
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
            vga::vga_write_str("no 'main' procedure in bytecode\n");
            serial::write_str("no 'main' procedure\n");
            loop {
                unsafe { core::arch::asm!("hlt"); }
            }
        }
    };

    serial::write_str("kernel: running LPPO main\n");
    vga::vga_write_str("kernel: running LPPO main\n");

    let mut env: BTreeMap<String, LppoValue> = BTreeMap::new();
    match execute_ops(&main_proc.operations, &mut env, &procs) {
        Ok(ControlFlow::None) => {
            vga::vga_write_str("\n[program finished]\n");
            serial::write_str("\n[program finished]\n");
        }
        Ok(ControlFlow::Return(Some(v))) => {
            vga::vga_write_str("\n[return: ");
            vga_write_value(&v);
            vga::vga_write_str("]\n");
            serial::write_str("\n[return: ");
            serial_write_value(&v);
            serial::write_str("]\n");
        }
        Ok(ControlFlow::Return(None)) => {
            vga::vga_write_str("\n[return]\n");
            serial::write_str("\n[return]\n");
        }
        Ok(ControlFlow::Break) => {
            vga::vga_write_str("\n[break]\n");
            serial::write_str("\n[break]\n");
        }
        Ok(ControlFlow::Continue) => {
            vga::vga_write_str("\n[continue]\n");
            serial::write_str("\n[continue]\n");
        }
        Err(e) => {
            vga::vga_write_str("\n[runtime error: ");
            vga::vga_write_str(&e);
            vga::vga_write_str("]\n");
            serial::write_str("\n[runtime error: ");
            serial::write_str(&e);
            serial::write_str("]\n");
        }
    }

    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}

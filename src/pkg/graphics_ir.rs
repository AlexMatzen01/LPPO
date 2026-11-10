//! Typed graphics intermediate representation (IR).
//!
//! This is an internal, typed IR that represents graphics operations lowered
//! from LPPO source. It is distinct from shader IR (e.g. SPIR-V) and from the
//! host program's control-flow IR. It documents a real lowering path:
//!
//!   LPPO AST → Graphics IR → verify → optimize → native backend
//!
//! MLIR or Rust MIR were evaluated (see Phase 9 / docs); neither is a stable
//! public embedding target for this compiler, so a typed, verifiable internal
//! IR is implemented with a documented future lowering path to MLIR/native.

use std::collections::HashMap;

////////////////////////////////////////////////////////////////////////////////
// IR types
////////////////////////////////////////////////////////////////////////////////

/// The type of a graphics IR value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GType {
    Float,
    Int,
    Vec2,
    Vec3,
    Vec4,
    Mat4,
    Color,
    Texture,
    Mesh,
    Material,
    Camera,
    Sampler,
    RenderTarget,
    Void,
}

impl GType {
    pub fn name(self) -> &'static str {
        match self {
            GType::Float => "float",
            GType::Int => "int",
            GType::Vec2 => "vec2",
            GType::Vec3 => "vec3",
            GType::Vec4 => "vec4",
            GType::Mat4 => "mat4",
            GType::Color => "color",
            GType::Texture => "texture",
            GType::Mesh => "mesh",
            GType::Material => "material",
            GType::Camera => "camera",
            GType::Sampler => "sampler",
            GType::RenderTarget => "rendertarget",
            GType::Void => "void",
        }
    }
}

/// An IR operand: either a constant or a reference to a previously-defined value.
#[derive(Debug, Clone, PartialEq)]
pub enum GOperand {
    Const(GConst),
    Ref(GId),
}

/// A constant IR value.
#[derive(Debug, Clone, PartialEq)]
pub enum GConst {
    Float(f64),
    Int(i64),
}

/// An SSA-style value identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GId(pub u32);

////////////////////////////////////////////////////////////////////////////////
// IR operations
////////////////////////////////////////////////////////////////////////////////

/// A single graphics IR operation.
#[derive(Debug, Clone, PartialEq)]
pub enum GOp {
    /// make_vec2 x y -> vec2
    MakeVec2 { result: GId, x: GOperand, y: GOperand },
    /// make_vec3 x y z -> vec3
    MakeVec3 { result: GId, x: GOperand, y: GOperand, z: GOperand },
    /// make_vec4 x y z w -> vec4
    MakeVec4 { result: GId, x: GOperand, y: GOperand, z: GOperand, w: GOperand },
    /// make_color r g b a -> color
    MakeColor { result: GId, r: GOperand, g: GOperand, b: GOperand, a: GOperand },
    /// make_mat4 ... -> mat4 (16 floats)
    MakeMat4 { result: GId, cols: [GOperand; 4] },
    /// clear r g b a
    Clear { r: GOperand, g: GOperand, b: GOperand, a: GOperand },
    /// draw_rectangle x y w h color
    DrawRectangle {
        x: GOperand,
        y: GOperand,
        w: GOperand,
        h: GOperand,
        color: GOperand,
    },
    /// draw_circle cx cy radius color
    DrawCircle {
        cx: GOperand,
        cy: GOperand,
        radius: GOperand,
        color: GOperand,
    },
    /// begin_frame window
    BeginFrame { window: GOperand },
    /// end_frame window
    EndFrame { window: GOperand },
    /// no-op marker
    Nop,
}

impl GOp {
    /// The type produced by this operation, if any.
    pub fn result_type(&self) -> Option<GType> {
        match self {
            GOp::MakeVec2 { .. } => Some(GType::Vec2),
            GOp::MakeVec3 { .. } => Some(GType::Vec3),
            GOp::MakeVec4 { .. } => Some(GType::Vec4),
            GOp::MakeColor { .. } => Some(GType::Color),
            GOp::MakeMat4 { .. } => Some(GType::Mat4),
            GOp::Clear { .. } | GOp::DrawRectangle { .. } | GOp::DrawCircle { .. }
            | GOp::BeginFrame { .. } | GOp::EndFrame { .. } | GOp::Nop => None,
        }
    }

    /// The operand type expected at each operand position (for verification).
    pub fn operand_types(&self) -> Vec<GType> {
        match self {
            GOp::MakeVec2 { .. } => vec![GType::Float, GType::Float],
            GOp::MakeVec3 { .. } => vec![GType::Float, GType::Float, GType::Float],
            GOp::MakeVec4 { .. } => {
                vec![GType::Float, GType::Float, GType::Float, GType::Float]
            }
            GOp::MakeColor { .. } => {
                vec![GType::Float, GType::Float, GType::Float, GType::Float]
            }
            GOp::MakeMat4 { .. } => {
                vec![GType::Float; 16]
            }
            GOp::Clear { .. } => vec![GType::Float, GType::Float, GType::Float, GType::Float],
            GOp::DrawRectangle { .. } => {
                vec![GType::Float, GType::Float, GType::Float, GType::Float, GType::Color]
            }
            GOp::DrawCircle { .. } => {
                vec![GType::Float, GType::Float, GType::Float, GType::Color]
            }
            GOp::BeginFrame { .. } => vec![GType::Int],
            GOp::EndFrame { .. } => vec![GType::Int],
            GOp::Nop => vec![],
        }
    }

    /// Human-readable op name.
    pub fn name(&self) -> &'static str {
        match self {
            GOp::MakeVec2 { .. } => "make_vec2",
            GOp::MakeVec3 { .. } => "make_vec3",
            GOp::MakeVec4 { .. } => "make_vec4",
            GOp::MakeColor { .. } => "make_color",
            GOp::MakeMat4 { .. } => "make_mat4",
            GOp::Clear { .. } => "clear",
            GOp::DrawRectangle { .. } => "draw_rectangle",
            GOp::DrawCircle { .. } => "draw_circle",
            GOp::BeginFrame { .. } => "begin_frame",
            GOp::EndFrame { .. } => "end_frame",
            GOp::Nop => "nop",
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// IR program
////////////////////////////////////////////////////////////////////////////////

/// A graphics IR program: a sequence of typed operations in SSA order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GProgram {
    pub ops: Vec<GOp>,
    /// result type of each defined GId, in order of definition.
    pub defined: HashMap<GId, GType>,
}

impl GProgram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, op: GOp) {
        if let Some(ty) = op.result_type() {
            self.defined.insert(op_result_id(&op), ty);
        }
        self.ops.push(op);
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// Extract the result GId from an op that produces one.
fn op_result_id(op: &GOp) -> GId {
    match op {
        GOp::MakeVec2 { result, .. } => *result,
        GOp::MakeVec3 { result, .. } => *result,
        GOp::MakeVec4 { result, .. } => *result,
        GOp::MakeColor { result, .. } => *result,
        GOp::MakeMat4 { result, .. } => *result,
        _ => GId(0),
    }
}

////////////////////////////////////////////////////////////////////////////////
// IR verification
////////////////////////////////////////////////////////////////////////////////

/// Error produced while verifying a graphics IR program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GVerifyError {
    pub op_index: usize,
    pub op: String,
    pub message: String,
}

impl std::fmt::Display for GVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "graphics IR verification error at op {} ({}): {}",
            self.op_index, self.op, self.message
        )
    }
}

/// Verify a graphics IR program.
///
/// Rules:
///   * every `Ref` must refer to a `GId` defined earlier in the program
///     (SSA dominance);
///   * each operand's type must match the operation's expected operand types;
///   * no undefined `GId` references.
pub fn verify(program: &GProgram) -> Result<(), Vec<GVerifyError>> {
    let mut errors: Vec<GVerifyError> = Vec::new();
    let mut defined: HashMap<GId, GType> = HashMap::new();

    for (i, op) in program.ops.iter().enumerate() {
        let expected = op.operand_types();
        let operands = op_operands(op);
        for (j, operand) in operands.iter().enumerate() {
            let exp_ty = expected.get(j).copied().unwrap_or(GType::Float);
            match operand {
                GOperand::Ref(id) => {
                    let def_ty = defined.get(id).ok_or_else(|| GVerifyError {
                        op_index: i,
                        op: op.name().to_string(),
                        message: format!("use of undefined id {id:?}"),
                    });
                    match def_ty {
                        Ok(ty) => {
                            if *ty != exp_ty {
                                errors.push(GVerifyError {
                                    op_index: i,
                                    op: op.name().to_string(),
                                    message: format!(
                                        "operand {} has type {:?}, expected {:?}",
                                        j, ty, exp_ty
                                    ),
                                });
                            }
                        }
                        Err(e) => errors.push(e),
                    }
                }
                GOperand::Const(c) => {
                    let const_ty = match c {
                        GConst::Float(_) => GType::Float,
                        GConst::Int(_) => GType::Int,
                    };
                    if const_ty != exp_ty {
                        errors.push(GVerifyError {
                            op_index: i,
                            op: op.name().to_string(),
                            message: format!(
                                "constant operand {} has type {:?}, expected {:?}",
                                j, const_ty, exp_ty
                            ),
                        });
                    }
                }
            }
        }
        if let Some(ty) = op.result_type() {
            defined.insert(op_result_id(op), ty);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Collect the operands of an op in order.
fn op_operands(op: &GOp) -> Vec<&GOperand> {
    match op {
        GOp::MakeVec2 { x, y, .. } => vec![x, y],
        GOp::MakeVec3 { x, y, z, .. } => vec![x, y, z],
        GOp::MakeVec4 { x, y, z, w, .. } => vec![x, y, z, w],
        GOp::MakeColor { r, g, b, a, .. } => vec![r, g, b, a],
        GOp::MakeMat4 { cols, .. } => cols.iter().collect(),
        GOp::Clear { r, g, b, a } => vec![r, g, b, a],
        GOp::DrawRectangle { x, y, w, h, color } => vec![x, y, w, h, color],
        GOp::DrawCircle { cx, cy, radius, color } => vec![cx, cy, radius, color],
        GOp::BeginFrame { window } => vec![window],
        GOp::EndFrame { window } => vec![window],
        GOp::Nop => vec![],
    }
}

////////////////////////////////////////////////////////////////////////////////
// Lowering helpers (a builder for constructing IR from higher-level constructs)
////////////////////////////////////////////////////////////////////////////////

/// A builder that constructs a `GProgram` while assigning fresh `GId`s.
pub struct GBuilder {
    program: GProgram,
    next_id: u32,
}

impl GBuilder {
    pub fn new() -> Self {
        Self { program: GProgram::new(), next_id: 1 }
    }

    pub fn id(&mut self) -> GId {
        let id = GId(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn program(self) -> GProgram {
        self.program
    }

    pub fn make_vec2(&mut self, x: GOperand, y: GOperand) -> GId {
        let r = self.id();
        self.program.push(GOp::MakeVec2 { result: r, x, y });
        r
    }

    pub fn make_color(&mut self, r: GOperand, g: GOperand, b: GOperand, a: GOperand) -> GId {
        let rid = self.id();
        self.program.push(GOp::MakeColor { result: rid, r, g, b, a });
        rid
    }

    pub fn draw_rectangle(
        &mut self,
        x: GOperand,
        y: GOperand,
        w: GOperand,
        h: GOperand,
        color: GOperand,
    ) {
        self.program.push(GOp::DrawRectangle { x, y, w, h, color });
    }

    pub fn draw_circle(
        &mut self,
        cx: GOperand,
        cy: GOperand,
        radius: GOperand,
        color: GOperand,
    ) {
        self.program.push(GOp::DrawCircle { cx, cy, radius, color });
    }

    pub fn begin_frame(&mut self, window: GOperand) {
        self.program.push(GOp::BeginFrame { window });
    }

    pub fn end_frame(&mut self, window: GOperand) {
        self.program.push(GOp::EndFrame { window });
    }

    pub fn clear(&mut self, r: GOperand, g: GOperand, b: GOperand, a: GOperand) {
        self.program.push(GOp::Clear { r, g, b, a });
    }

    pub fn const_f(v: f64) -> GOperand {
        GOperand::Const(GConst::Float(v))
    }
}

////////////////////////////////////////////////////////////////////////////////
// IR dumping
////////////////////////////////////////////////////////////////////////////////

impl std::fmt::Display for GProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, op) in self.ops.iter().enumerate() {
            writeln!(f, "  %{i}: {}", op_fmt(op))?;
        }
        Ok(())
    }
}

fn op_fmt(op: &GOp) -> String {
    match op {
        GOp::MakeVec2 { result, x, y } => {
            format!("%{} = make_vec2({}, {})", result.0, op_operand_fmt(x), op_operand_fmt(y))
        }
        GOp::MakeColor { result, r, g, b, a } => format!(
            "%{} = make_color({}, {}, {}, {})",
            result.0,
            op_operand_fmt(r),
            op_operand_fmt(g),
            op_operand_fmt(b),
            op_operand_fmt(a)
        ),
        GOp::DrawRectangle { x, y, w, h, color } => format!(
            "draw_rectangle({}, {}, {}, {}, {})",
            op_operand_fmt(x),
            op_operand_fmt(y),
            op_operand_fmt(w),
            op_operand_fmt(h),
            op_operand_fmt(color)
        ),
        GOp::DrawCircle { cx, cy, radius, color } => format!(
            "draw_circle({}, {}, {}, {})",
            op_operand_fmt(cx),
            op_operand_fmt(cy),
            op_operand_fmt(radius),
            op_operand_fmt(color)
        ),
        GOp::Clear { r, g, b, a } => {
            format!("clear({}, {}, {}, {})", op_operand_fmt(r), op_operand_fmt(g), op_operand_fmt(b), op_operand_fmt(a))
        }
        GOp::BeginFrame { window } => format!("begin_frame({})", op_operand_fmt(window)),
        GOp::EndFrame { window } => format!("end_frame({})", op_operand_fmt(window)),
        GOp::MakeVec3 { .. } => "make_vec3(...)".to_string(),
        GOp::MakeVec4 { .. } => "make_vec4(...)".to_string(),
        GOp::MakeMat4 { .. } => "make_mat4(...)".to_string(),
        GOp::Nop => "nop".to_string(),
    }
}

fn op_operand_fmt(op: &GOperand) -> String {
    match op {
        GOperand::Const(GConst::Float(v)) => format!("{v}"),
        GOperand::Const(GConst::Int(v)) => format!("{v}"),
        GOperand::Ref(id) => format!("%{}", id.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_program_verifies() {
        let mut b = GBuilder::new();
        let r = GBuilder::const_f(1.0);
        let g = GBuilder::const_f(0.0);
        let bl = GBuilder::const_f(0.0);
        let a = GBuilder::const_f(1.0);
        let color = b.make_color(r, g, bl, a);
        b.draw_rectangle(GBuilder::const_f(0.0), GBuilder::const_f(0.0), GBuilder::const_f(10.0), GBuilder::const_f(10.0), GOperand::Ref(color));
        assert!(verify(&b.program).is_ok());
    }

    #[test]
    fn undefined_ref_fails_verification() {
        let mut b = GBuilder::new();
        b.program.ops.push(GOp::DrawCircle {
            cx: GOperand::Ref(GId(999)),
            cy: GBuilder::const_f(0.0),
            radius: GBuilder::const_f(1.0),
            color: GBuilder::const_f(0.0),
        });
        let errs = verify(&b.program).unwrap_err();
        assert!(!errs.is_empty());
        assert!(errs[0].message.contains("undefined"));
    }

    #[test]
    fn type_mismatch_fails_verification() {
        // make_color expects four floats; pass an int in the red slot.
        let mut b = GBuilder::new();
        let red = GOperand::Const(GConst::Int(5));
        let green = GBuilder::const_f(0.0);
        let blue = GBuilder::const_f(0.0);
        let alpha = GBuilder::const_f(1.0);
        let result = b.id();
        b.program.ops.push(GOp::MakeColor {
            result,
            r: red,
            g: green,
            b: blue,
            a: alpha,
        });
        let errs = verify(&b.program).unwrap_err();
        assert!(!errs.is_empty());
    }

    #[test]
    fn ir_dump_is_readable() {
        let mut b = GBuilder::new();
        let r = GBuilder::const_f(1.0);
        let g = GBuilder::const_f(0.0);
        let bl = GBuilder::const_f(0.0);
        let a = GBuilder::const_f(1.0);
        let color = b.make_color(r, g, bl, a);
        b.draw_rectangle(GBuilder::const_f(1.0), GBuilder::const_f(2.0), GBuilder::const_f(3.0), GBuilder::const_f(4.0), GOperand::Ref(color));
        let dump = format!("{}", b.program);
        assert!(dump.contains("make_color"));
        assert!(dump.contains("draw_rectangle"));
    }

    #[test]
    fn result_type_tracking() {
        let mut b = GBuilder::new();
        let r = GBuilder::const_f(1.0);
        let g = GBuilder::const_f(0.0);
        let bl = GBuilder::const_f(0.0);
        let a = GBuilder::const_f(1.0);
        let color = b.make_color(r, g, bl, a);
        assert_eq!(b.program.defined.get(&color), Some(&GType::Color));
    }
}

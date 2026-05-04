//! Tensor identity, dtype, and shape — the value-plane primitives of the
//! IR.  See spec MX01 §"Core types".

use core::fmt;

/// A unique identifier for a tensor produced by an op or supplied as an
/// input/constant.  Allocated densely starting at 0 by [`GraphBuilder`].
///
/// We use a 32-bit id rather than a 64-bit one because:
/// - Real graphs have thousands, not billions, of tensors — 32 bits is
///   ~4 billion which is wildly more than any practical graph.
/// - The wire format is smaller (4 bytes vs 8), and tensor ids appear
///   many times per graph.
///
/// [`GraphBuilder`]: crate::GraphBuilder
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TensorId(pub u32);

/// A unique identifier for an op within a graph.  Currently only used
/// for diagnostics and round-trip stability — ops are ordered by their
/// position in [`Graph::ops`](crate::Graph::ops).
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OpId(pub u32);

/// The element type of a tensor.
///
/// V1 supports three dtypes:
/// - [`DType::F32`] — IEEE 754 single-precision float, 4 bytes
/// - [`DType::U8`]  — unsigned 8-bit integer, also used for booleans
///   (0 or 1) since shader languages typically don't have bool tensors
/// - [`DType::I32`] — signed 32-bit integer, 4 bytes
///
/// `f16`, `bf16`, `i64`, `complex64` are deferred to V2.  The wire
/// format reserves tags `0x03` and `0x04` for `F16` and `I64`.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum DType {
    /// IEEE 754 binary32.
    F32,
    /// Unsigned 8-bit.
    U8,
    /// Signed 32-bit.
    I32,
}

impl DType {
    /// The size of a single element in bytes.
    ///
    /// | DType | size |
    /// |-------|------|
    /// | F32   | 4    |
    /// | U8    | 1    |
    /// | I32   | 4    |
    pub const fn size_bytes(self) -> usize {
        match self {
            DType::F32 => 4,
            DType::U8 => 1,
            DType::I32 => 4,
        }
    }

    /// The wire-format tag for this dtype.  See spec MX03 §"Encoding
    /// `matrix-ir` types".
    pub const fn wire_tag(self) -> u8 {
        match self {
            DType::F32 => 0x00,
            DType::U8 => 0x01,
            DType::I32 => 0x02,
        }
    }

    /// Decode a wire-format tag back into a [`DType`].  Returns `None`
    /// for unknown or reserved tags.
    pub const fn from_wire_tag(tag: u8) -> Option<DType> {
        match tag {
            0x00 => Some(DType::F32),
            0x01 => Some(DType::U8),
            0x02 => Some(DType::I32),
            // 0x03 reserved for F16, 0x04 reserved for I64
            _ => None,
        }
    }
}

impl fmt::Display for DType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DType::F32 => f.write_str("f32"),
            DType::U8 => f.write_str("u8"),
            DType::I32 => f.write_str("i32"),
        }
    }
}

/// The shape of a tensor — its dimensions in row-major / leftmost-major
/// order.  A scalar has empty `dims`; a vector has one dim; a matrix
/// has two; and so on up to whatever rank the graph is built with
/// (validators downstream may cap rank for backend support).
///
/// V1 shapes are **fully static** — every dimension is a concrete `u32`.
/// V2 introduces symbolic dimensions for dynamic shapes; V1's `Shape`
/// is the special case where every dim is concrete.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Shape {
    /// Dimensions in leftmost-major order.  Empty `Vec` means scalar.
    pub dims: Vec<u32>,
}

impl Shape {
    /// Construct a shape from a slice.  Convenience for tests and
    /// builder usage.
    pub fn from(dims: &[u32]) -> Self {
        Shape {
            dims: dims.to_vec(),
        }
    }

    /// Construct a scalar shape (rank 0, empty dims).
    pub fn scalar() -> Self {
        Shape { dims: Vec::new() }
    }

    /// The rank of the shape — the number of dimensions.  Scalar = 0.
    pub fn rank(&self) -> usize {
        self.dims.len()
    }

    /// The total number of elements in this shape — the product of all
    /// dims, or `1` for a scalar.
    ///
    /// Returns `None` if the product overflows `u64`.  Real shapes never
    /// overflow `u64`; this primarily protects against malicious input
    /// from deserialization.
    pub fn numel(&self) -> Option<u64> {
        let mut acc: u64 = 1;
        for &d in &self.dims {
            acc = acc.checked_mul(d as u64)?;
        }
        Some(acc)
    }

    /// The total byte size of a tensor with this shape and dtype.
    /// Returns `None` on overflow.
    pub fn byte_size(&self, dtype: DType) -> Option<u64> {
        self.numel()?
            .checked_mul(dtype.size_bytes() as u64)
    }
}

impl fmt::Display for Shape {
    /// Format as `[d0, d1, d2]` (or `[]` for scalar).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for (i, d) in self.dims.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{}", d)?;
        }
        f.write_str("]")
    }
}

/// A tensor — an immutable value with identity, dtype, and shape.
///
/// Tensors are produced by ops, supplied as graph inputs, or
/// materialised from constants.  They carry no data themselves; the
/// data flows through buffers in the lower IR (`compute-ir`).
///
/// In SSA form, every tensor has exactly one defining op (or is an
/// input or constant).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Tensor {
    /// The unique identifier for this tensor within its graph.
    pub id: TensorId,
    /// The element type.
    pub dtype: DType,
    /// The static shape.
    pub shape: Shape,
}

impl Tensor {
    /// Construct a tensor.  Mostly useful for tests; production code
    /// builds tensors via [`GraphBuilder`](crate::GraphBuilder).
    pub fn new(id: TensorId, dtype: DType, shape: Shape) -> Self {
        Tensor { id, dtype, shape }
    }
}

impl fmt::Display for Tensor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{} {} {}", self.id.0, self.dtype, self.shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dtype_size_bytes_matches_spec() {
        assert_eq!(DType::F32.size_bytes(), 4);
        assert_eq!(DType::U8.size_bytes(), 1);
        assert_eq!(DType::I32.size_bytes(), 4);
    }

    #[test]
    fn dtype_wire_tag_round_trips() {
        for dt in [DType::F32, DType::U8, DType::I32] {
            let tag = dt.wire_tag();
            assert_eq!(DType::from_wire_tag(tag), Some(dt));
        }
    }

    #[test]
    fn dtype_from_unknown_tag_returns_none() {
        assert_eq!(DType::from_wire_tag(0xFF), None);
        // Reserved-for-V2 tags also return None in V1.
        assert_eq!(DType::from_wire_tag(0x03), None);
        assert_eq!(DType::from_wire_tag(0x04), None);
    }

    #[test]
    fn shape_scalar() {
        let s = Shape::scalar();
        assert_eq!(s.rank(), 0);
        assert_eq!(s.numel(), Some(1));
        assert_eq!(s.byte_size(DType::F32), Some(4));
    }

    #[test]
    fn shape_numel() {
        let s = Shape::from(&[2, 3, 4]);
        assert_eq!(s.rank(), 3);
        assert_eq!(s.numel(), Some(24));
        assert_eq!(s.byte_size(DType::F32), Some(96));
        assert_eq!(s.byte_size(DType::U8), Some(24));
    }

    #[test]
    fn shape_overflow_protection() {
        // Two huge dims that overflow u64 when multiplied.
        let s = Shape::from(&[u32::MAX, u32::MAX]);
        // u32::MAX * u32::MAX = ~1.8e19, just under u64::MAX (~1.8e19).
        // The third dim would push it over.
        let s = Shape {
            dims: vec![u32::MAX, u32::MAX, 2],
        };
        assert_eq!(s.numel(), None, "should detect overflow");
    }

    #[test]
    fn shape_display() {
        assert_eq!(format!("{}", Shape::from(&[1, 2, 3])), "[1, 2, 3]");
        assert_eq!(format!("{}", Shape::scalar()), "[]");
    }

    #[test]
    fn tensor_display() {
        let t = Tensor::new(TensorId(7), DType::F32, Shape::from(&[2, 2]));
        assert_eq!(format!("{}", t), "t7 f32 [2, 2]");
    }
}

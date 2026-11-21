//! # paint-batch-tiles
//!
//! P2D09: Retained-mode 2D sprite and tile batch lowering and zero-allocation diffing.
//!
//! Provides hardware-style sprite and tile batch structures, deterministic z-priority
//! and coordinate lowering into universal `paint-instructions` IR ([`PaintScene`]), and
//! zero-allocation retained-mode frame diffing for incremental rendering and `PaintVM.patch()` updates.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use paint_instructions::{PaintBase, PaintInstruction, PaintRect, PaintScene};
pub use pixel_container::PixelContainer;

/// Maximum number of sprites or tile cells allowed in a single batch (CWE-400 resource limit).
pub const MAX_BATCH_SPRITES: usize = 4096;

/// Sentinel index value used in [`SpriteDeltaOp`] when an index does not apply.
pub const NO_INDEX: u16 = u16::MAX;

/// A 2D textured quad representing an individual sprite on the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteQuad {
    /// Unique identifier for this sprite instance.
    pub id: u32,
    /// X coordinate of top-left corner in logical scene units.
    pub x: f64,
    /// Y coordinate of top-left corner in logical scene units.
    pub y: f64,
    /// Width of the sprite in logical scene units. Must be non-negative and finite.
    pub width: f64,
    /// Height of the sprite in logical scene units. Must be non-negative and finite.
    pub height: f64,
    /// Z-layer priority for ordering (lower numbers painted behind higher numbers).
    pub z_priority: u8,
    /// Horizontal mirroring flag.
    pub h_flip: bool,
    /// Vertical mirroring flag.
    pub v_flip: bool,
    /// RGB tint or direct fill color.
    pub color_rgb: [u8; 3],
    /// Visibility flag. Non-visible sprites are skipped during scene lowering.
    pub visible: bool,
}

impl SpriteQuad {
    /// Creates a new visible sprite quad with default priority 0 and white color `[255, 255, 255]`.
    pub const fn new(id: u32, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            z_priority: 0,
            h_flip: false,
            v_flip: false,
            color_rgb: [255, 255, 255],
            visible: true,
        }
    }

    /// Builder to set the RGB tint/fill color.
    pub const fn with_color(mut self, rgb: [u8; 3]) -> Self {
        self.color_rgb = rgb;
        self
    }

    /// Builder to set the Z-layer priority.
    pub const fn with_z_priority(mut self, z: u8) -> Self {
        self.z_priority = z;
        self
    }

    /// Builder to set horizontal and vertical flip flags.
    pub const fn with_flip(mut self, h_flip: bool, v_flip: bool) -> Self {
        self.h_flip = h_flip;
        self.v_flip = v_flip;
        self
    }

    /// Builder to set visibility.
    pub const fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

/// A single cell within a 2D tilemap grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileBatchCell {
    /// Zero-based column in the tilemap grid.
    pub grid_col: u16,
    /// Zero-based row in the tilemap grid.
    pub grid_row: u16,
    /// Index into the tileset or atlas.
    pub tile_id: u16,
    /// Hardware palette index.
    pub palette_id: u8,
    /// Horizontal mirroring flag.
    pub h_flip: bool,
    /// Vertical mirroring flag.
    pub v_flip: bool,
    /// RGB tint or direct fill color.
    pub color_rgb: [u8; 3],
}

impl TileBatchCell {
    /// Creates a new tile cell with default palette 0 and white color.
    pub const fn new(grid_col: u16, grid_row: u16, tile_id: u16) -> Self {
        Self {
            grid_col,
            grid_row,
            tile_id,
            palette_id: 0,
            h_flip: false,
            v_flip: false,
            color_rgb: [255, 255, 255],
        }
    }

    /// Builder to set the RGB tint/fill color.
    pub const fn with_color(mut self, rgb: [u8; 3]) -> Self {
        self.color_rgb = rgb;
        self
    }

    /// Builder to set the palette id.
    pub const fn with_palette(mut self, palette_id: u8) -> Self {
        self.palette_id = palette_id;
        self
    }

    /// Builder to set mirroring flags.
    pub const fn with_flip(mut self, h_flip: bool, v_flip: bool) -> Self {
        self.h_flip = h_flip;
        self.v_flip = v_flip;
        self
    }
}

/// Classification of change for an individual sprite across consecutive frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeltaKind {
    /// Sprite appeared in the next batch but was absent from the previous batch.
    Added,
    /// Sprite is present in both batches but has modified attributes.
    Modified,
    /// Sprite was present in the previous batch but removed from the next batch.
    Removed,
    /// Sprite is present in both batches with identical attributes.
    Unchanged,
}

/// A zero-allocation delta record describing a change to one sprite between frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteDeltaOp {
    /// Kind of change operation.
    pub kind: DeltaKind,
    /// Sprite ID.
    pub id: u32,
    /// Index in previous frame batch, or [`NO_INDEX`] if newly added.
    pub old_index: u16,
    /// Index in next frame batch, or [`NO_INDEX`] if removed.
    pub new_index: u16,
}

/// High-level retained-mode delta summary between two [`PaintScene`] trees by instruction ID.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ScenePatch {
    /// Instruction IDs present in next scene but absent in previous scene.
    pub added_ids: Vec<String>,
    /// Instruction IDs present in both scenes with different content.
    pub modified_ids: Vec<String>,
    /// Instruction IDs present in previous scene but absent in next scene.
    pub removed_ids: Vec<String>,
    /// Number of instructions present in both scenes with identical content.
    pub unchanged_count: usize,
}

/// Error conditions for sprite and tile batch processing.
#[derive(Clone, Debug, PartialEq)]
pub enum PaintBatchError {
    /// Batch item count exceeded the CWE-400 limit.
    BatchTooLarge {
        /// Actual item count encountered.
        count: usize,
        /// Maximum allowed item count.
        max: usize,
    },
    /// Viewport or scene dimensions are non-finite, negative, or zero (CWE-20).
    InvalidDimensions {
        /// Provided width.
        width: f64,
        /// Provided height.
        height: f64,
    },
    /// Tile dimensions are non-finite, negative, or zero (CWE-20).
    InvalidTileDimensions {
        /// Provided tile width.
        width: f64,
        /// Provided tile height.
        height: f64,
    },
    /// Sprite coordinates or dimensions are non-finite or negative (CWE-20).
    InvalidCoordinates {
        /// Sprite ID that failed validation.
        id: u32,
        /// X coordinate.
        x: f64,
        /// Y coordinate.
        y: f64,
    },
    /// Duplicate sprite ID encountered within a single batch (CWE-20).
    DuplicateSpriteId(u32),
    /// Duplicate tile grid cell coordinates encountered (CWE-20).
    DuplicateTileId {
        /// Grid column.
        col: u16,
        /// Grid row.
        row: u16,
    },
    /// The output buffer provided to `diff_sprite_batches_into` is too small.
    BufferTooSmall {
        /// Required buffer capacity.
        required: usize,
        /// Provided buffer capacity.
        provided: usize,
    },
    /// An instruction in a PaintScene lacks a stable PaintBase.id for retained-mode diffing.
    MissingInstructionId,
    /// Duplicate instruction ID encountered in a PaintScene (CWE-20).
    DuplicateInstructionId(String),
}

impl std::fmt::Display for PaintBatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BatchTooLarge { count, max } => {
                write!(f, "batch size {count} exceeds maximum allowed {max}")
            }
            Self::InvalidDimensions { width, height } => {
                write!(
                    f,
                    "invalid scene dimensions: width={width}, height={height}"
                )
            }
            Self::InvalidTileDimensions { width, height } => {
                write!(f, "invalid tile dimensions: width={width}, height={height}")
            }
            Self::InvalidCoordinates { id, x, y } => {
                write!(f, "invalid coordinates for sprite {id}: ({x}, {y})")
            }
            Self::DuplicateSpriteId(id) => {
                write!(f, "duplicate sprite ID: {id}")
            }
            Self::DuplicateTileId { col, row } => {
                write!(f, "duplicate tile grid coordinate: ({col}, {row})")
            }
            Self::BufferTooSmall { required, provided } => {
                write!(
                    f,
                    "output buffer too small: required {required}, provided {provided}"
                )
            }
            Self::MissingInstructionId => {
                write!(
                    f,
                    "paint instruction is missing a required stable PaintBase.id"
                )
            }
            Self::DuplicateInstructionId(id) => {
                write!(f, "duplicate instruction ID in scene: {id}")
            }
        }
    }
}

impl std::error::Error for PaintBatchError {}

/// Extracts a reference to the [`PaintBase`] header from any [`PaintInstruction`].
pub fn instruction_base(instr: &PaintInstruction) -> &PaintBase {
    match instr {
        PaintInstruction::Rect(r) => &r.base,
        PaintInstruction::Ellipse(e) => &e.base,
        PaintInstruction::Path(p) => &p.base,
        PaintInstruction::Text(t) => &t.base,
        PaintInstruction::GlyphRun(g) => &g.base,
        PaintInstruction::Group(g) => &g.base,
        PaintInstruction::Layer(l) => &l.base,
        PaintInstruction::Line(l) => &l.base,
        PaintInstruction::Clip(c) => &c.base,
        PaintInstruction::Gradient(g) => &g.base,
        PaintInstruction::Image(i) => &i.base,
    }
}

/// Diffs two sprite batches directly into a caller-supplied slice with zero heap allocations (VER07).
///
/// Returns the number of delta operations written to `out_ops`.
/// Rejects batches exceeding [`MAX_BATCH_SPRITES`] (CWE-400) or batches containing duplicate IDs.
pub fn diff_sprite_batches_into(
    prev: &[SpriteQuad],
    next: &[SpriteQuad],
    out_ops: &mut [SpriteDeltaOp],
) -> Result<usize, PaintBatchError> {
    if prev.len() > MAX_BATCH_SPRITES {
        return Err(PaintBatchError::BatchTooLarge {
            count: prev.len(),
            max: MAX_BATCH_SPRITES,
        });
    }
    if next.len() > MAX_BATCH_SPRITES {
        return Err(PaintBatchError::BatchTooLarge {
            count: next.len(),
            max: MAX_BATCH_SPRITES,
        });
    }

    // Check duplicate IDs in prev with zero heap allocation
    for (i, p) in prev.iter().enumerate() {
        for other in prev.iter().skip(i + 1) {
            if p.id == other.id {
                return Err(PaintBatchError::DuplicateSpriteId(p.id));
            }
        }
    }
    // Check duplicate IDs in next with zero heap allocation
    for (i, n) in next.iter().enumerate() {
        for other in next.iter().skip(i + 1) {
            if n.id == other.id {
                return Err(PaintBatchError::DuplicateSpriteId(n.id));
            }
        }
    }

    // Check if both prev and next are sorted by id (fast O(N) path)
    let prev_sorted = prev.windows(2).all(|w| w[0].id <= w[1].id);
    let next_sorted = next.windows(2).all(|w| w[0].id <= w[1].id);

    // Zero-allocation capacity pre-pass so out_ops remains untouched on BufferTooSmall.
    let total_required = if prev_sorted && next_sorted {
        let mut p = 0;
        let mut q = 0;
        let mut count = 0;
        while p < prev.len() && q < next.len() {
            if prev[p].id < next[q].id {
                p += 1;
            } else if prev[p].id > next[q].id {
                q += 1;
            } else {
                p += 1;
                q += 1;
            }
            count += 1;
        }
        count + (prev.len() - p) + (next.len() - q)
    } else {
        let mut count = prev.len();
        for n_sprite in next {
            if !prev.iter().any(|p| p.id == n_sprite.id) {
                count += 1;
            }
        }
        count
    };

    if total_required > out_ops.len() {
        return Err(PaintBatchError::BufferTooSmall {
            required: total_required,
            provided: out_ops.len(),
        });
    }

    let mut written = 0;

    if prev_sorted && next_sorted {
        let mut p = 0;
        let mut q = 0;
        while p < prev.len() && q < next.len() {
            if prev[p].id < next[q].id {
                if written >= out_ops.len() {
                    return Err(PaintBatchError::BufferTooSmall {
                        required: written + 1,
                        provided: out_ops.len(),
                    });
                }
                out_ops[written] = SpriteDeltaOp {
                    kind: DeltaKind::Removed,
                    id: prev[p].id,
                    old_index: p as u16,
                    new_index: NO_INDEX,
                };
                written += 1;
                p += 1;
            } else if prev[p].id > next[q].id {
                if written >= out_ops.len() {
                    return Err(PaintBatchError::BufferTooSmall {
                        required: written + 1,
                        provided: out_ops.len(),
                    });
                }
                out_ops[written] = SpriteDeltaOp {
                    kind: DeltaKind::Added,
                    id: next[q].id,
                    old_index: NO_INDEX,
                    new_index: q as u16,
                };
                written += 1;
                q += 1;
            } else {
                let kind = if prev[p] == next[q] {
                    DeltaKind::Unchanged
                } else {
                    DeltaKind::Modified
                };
                if written >= out_ops.len() {
                    return Err(PaintBatchError::BufferTooSmall {
                        required: written + 1,
                        provided: out_ops.len(),
                    });
                }
                out_ops[written] = SpriteDeltaOp {
                    kind,
                    id: prev[p].id,
                    old_index: p as u16,
                    new_index: q as u16,
                };
                written += 1;
                p += 1;
                q += 1;
            }
        }
        while p < prev.len() {
            if written >= out_ops.len() {
                return Err(PaintBatchError::BufferTooSmall {
                    required: written + 1,
                    provided: out_ops.len(),
                });
            }
            out_ops[written] = SpriteDeltaOp {
                kind: DeltaKind::Removed,
                id: prev[p].id,
                old_index: p as u16,
                new_index: NO_INDEX,
            };
            written += 1;
            p += 1;
        }
        while q < next.len() {
            if written >= out_ops.len() {
                return Err(PaintBatchError::BufferTooSmall {
                    required: written + 1,
                    provided: out_ops.len(),
                });
            }
            out_ops[written] = SpriteDeltaOp {
                kind: DeltaKind::Added,
                id: next[q].id,
                old_index: NO_INDEX,
                new_index: q as u16,
            };
            written += 1;
            q += 1;
        }
    } else {
        // Fallback for unsorted batches (zero heap allocation)
        for (old_idx, p_sprite) in prev.iter().enumerate() {
            let mut found = None;
            for (new_idx, n_sprite) in next.iter().enumerate() {
                if n_sprite.id == p_sprite.id {
                    found = Some(new_idx);
                    break;
                }
            }
            if let Some(new_idx) = found {
                let kind = if *p_sprite == next[new_idx] {
                    DeltaKind::Unchanged
                } else {
                    DeltaKind::Modified
                };
                if written >= out_ops.len() {
                    return Err(PaintBatchError::BufferTooSmall {
                        required: written + 1,
                        provided: out_ops.len(),
                    });
                }
                out_ops[written] = SpriteDeltaOp {
                    kind,
                    id: p_sprite.id,
                    old_index: old_idx as u16,
                    new_index: new_idx as u16,
                };
                written += 1;
            } else {
                if written >= out_ops.len() {
                    return Err(PaintBatchError::BufferTooSmall {
                        required: written + 1,
                        provided: out_ops.len(),
                    });
                }
                out_ops[written] = SpriteDeltaOp {
                    kind: DeltaKind::Removed,
                    id: p_sprite.id,
                    old_index: old_idx as u16,
                    new_index: NO_INDEX,
                };
                written += 1;
            }
        }
        for (new_idx, n_sprite) in next.iter().enumerate() {
            let found_in_prev = prev.iter().any(|p| p.id == n_sprite.id);
            if !found_in_prev {
                if written >= out_ops.len() {
                    return Err(PaintBatchError::BufferTooSmall {
                        required: written + 1,
                        provided: out_ops.len(),
                    });
                }
                out_ops[written] = SpriteDeltaOp {
                    kind: DeltaKind::Added,
                    id: n_sprite.id,
                    old_index: NO_INDEX,
                    new_index: new_idx as u16,
                };
                written += 1;
            }
        }
    }

    Ok(written)
}

/// Diffs two [`PaintScene`] graphs by stable [`PaintBase::id`] for retained-mode updates.
///
/// Returns a [`ScenePatch`] containing added, modified, and removed instruction IDs,
/// as well as the count of unchanged instructions.
/// Rejects scenes containing duplicate instruction IDs or missing IDs.
pub fn diff_paint_scenes(
    prev: &PaintScene,
    next: &PaintScene,
) -> Result<ScenePatch, PaintBatchError> {
    use std::collections::HashMap;

    if prev.instructions.len() > MAX_BATCH_SPRITES {
        return Err(PaintBatchError::BatchTooLarge {
            count: prev.instructions.len(),
            max: MAX_BATCH_SPRITES,
        });
    }
    if next.instructions.len() > MAX_BATCH_SPRITES {
        return Err(PaintBatchError::BatchTooLarge {
            count: next.instructions.len(),
            max: MAX_BATCH_SPRITES,
        });
    }

    let mut prev_map: HashMap<&str, &PaintInstruction> =
        HashMap::with_capacity(prev.instructions.len());
    for instr in &prev.instructions {
        let base = instruction_base(instr);
        let id = base
            .id
            .as_deref()
            .ok_or(PaintBatchError::MissingInstructionId)?;
        if prev_map.insert(id, instr).is_some() {
            return Err(PaintBatchError::DuplicateInstructionId(id.to_string()));
        }
    }

    let mut next_map: HashMap<&str, &PaintInstruction> =
        HashMap::with_capacity(next.instructions.len());
    let mut added_ids = Vec::new();
    let mut modified_ids = Vec::new();
    let mut unchanged_count = 0;

    for instr in &next.instructions {
        let base = instruction_base(instr);
        let id = base
            .id
            .as_deref()
            .ok_or(PaintBatchError::MissingInstructionId)?;
        if next_map.insert(id, instr).is_some() {
            return Err(PaintBatchError::DuplicateInstructionId(id.to_string()));
        }

        if let Some(prev_instr) = prev_map.get(id) {
            if *prev_instr == instr {
                unchanged_count += 1;
            } else {
                modified_ids.push(id.to_string());
            }
        } else {
            added_ids.push(id.to_string());
        }
    }

    let mut removed_ids = Vec::new();
    for instr in &prev.instructions {
        let base = instruction_base(instr);
        let id = base
            .id
            .as_deref()
            .ok_or(PaintBatchError::MissingInstructionId)?;
        if !next_map.contains_key(id) {
            removed_ids.push(id.to_string());
        }
    }

    Ok(ScenePatch {
        added_ids,
        modified_ids,
        removed_ids,
        unchanged_count,
    })
}

/// Lowers a batch of sprites into a universal [`PaintScene`].
///
/// Sprites are validated (finite positive dimensions, finite coordinates, no duplicate IDs,
/// capped at [`MAX_BATCH_SPRITES`]), filtered by visibility, and sorted
/// deterministically by `(z_priority, id)` before emitting [`PaintInstruction::Rect`] nodes.
pub fn lower_sprites_to_paint_scene(
    width: f64,
    height: f64,
    background: &str,
    sprites: &[SpriteQuad],
) -> Result<PaintScene, PaintBatchError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(PaintBatchError::InvalidDimensions { width, height });
    }
    if sprites.len() > MAX_BATCH_SPRITES {
        return Err(PaintBatchError::BatchTooLarge {
            count: sprites.len(),
            max: MAX_BATCH_SPRITES,
        });
    }

    // Check finite coordinates and duplicates
    for (i, s) in sprites.iter().enumerate() {
        if !s.x.is_finite()
            || !s.y.is_finite()
            || !s.width.is_finite()
            || !s.height.is_finite()
            || s.width < 0.0
            || s.height < 0.0
        {
            return Err(PaintBatchError::InvalidCoordinates {
                id: s.id,
                x: s.x,
                y: s.y,
            });
        }
        for other in sprites.iter().skip(i + 1) {
            if s.id == other.id {
                return Err(PaintBatchError::DuplicateSpriteId(s.id));
            }
        }
    }

    let mut visible: Vec<&SpriteQuad> = sprites.iter().filter(|s| s.visible).collect();
    visible.sort_by_key(|s| (s.z_priority, s.id));

    let mut instructions = Vec::with_capacity(visible.len());
    for s in visible {
        let metadata = if s.h_flip || s.v_flip {
            let mut map = std::collections::HashMap::new();
            if s.h_flip {
                map.insert("h_flip".to_string(), "true".to_string());
            }
            if s.v_flip {
                map.insert("v_flip".to_string(), "true".to_string());
            }
            Some(map)
        } else {
            None
        };

        let color_hex = format!(
            "#{:02x}{:02x}{:02x}",
            s.color_rgb[0], s.color_rgb[1], s.color_rgb[2]
        );

        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some(format!("sprite:{}", s.id)),
                metadata,
            },
            x: s.x,
            y: s.y,
            width: s.width,
            height: s.height,
            fill: Some(color_hex),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
    }

    Ok(PaintScene {
        width,
        height,
        background: background.to_string(),
        instructions,
        id: None,
        metadata: None,
    })
}

/// Lowers a 2D tile grid into a universal [`PaintScene`].
///
/// Grid dimensions and tile cell dimensions must be finite and positive.
/// Duplicate `(grid_col, grid_row)` coordinates are rejected.
/// The batch size is capped at [`MAX_BATCH_SPRITES`].
/// Cells are sorted deterministically by row then column, emitting
/// [`PaintInstruction::Rect`] nodes with stable `tile:<col>:<row>` IDs.
pub fn lower_tile_batch_to_paint_scene(
    scene_width: f64,
    scene_height: f64,
    tile_width: f64,
    tile_height: f64,
    background: &str,
    cells: &[TileBatchCell],
) -> Result<PaintScene, PaintBatchError> {
    if !scene_width.is_finite()
        || !scene_height.is_finite()
        || scene_width <= 0.0
        || scene_height <= 0.0
    {
        return Err(PaintBatchError::InvalidDimensions {
            width: scene_width,
            height: scene_height,
        });
    }
    if !tile_width.is_finite()
        || !tile_height.is_finite()
        || tile_width <= 0.0
        || tile_height <= 0.0
    {
        return Err(PaintBatchError::InvalidTileDimensions {
            width: tile_width,
            height: tile_height,
        });
    }
    if cells.len() > MAX_BATCH_SPRITES {
        return Err(PaintBatchError::BatchTooLarge {
            count: cells.len(),
            max: MAX_BATCH_SPRITES,
        });
    }

    // Check duplicate tile grid coordinates
    for (i, c) in cells.iter().enumerate() {
        for other in cells.iter().skip(i + 1) {
            if c.grid_col == other.grid_col && c.grid_row == other.grid_row {
                return Err(PaintBatchError::DuplicateTileId {
                    col: c.grid_col,
                    row: c.grid_row,
                });
            }
        }
    }

    let mut sorted_cells = cells.to_vec();
    sorted_cells.sort_by_key(|c| (c.grid_row, c.grid_col));

    let mut instructions = Vec::with_capacity(sorted_cells.len());
    for cell in sorted_cells {
        let x = cell.grid_col as f64 * tile_width;
        let y = cell.grid_row as f64 * tile_height;
        if !x.is_finite() || !y.is_finite() {
            return Err(PaintBatchError::InvalidTileDimensions {
                width: tile_width,
                height: tile_height,
            });
        }

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("tile_id".to_string(), cell.tile_id.to_string());
        metadata.insert("palette_id".to_string(), cell.palette_id.to_string());
        if cell.h_flip {
            metadata.insert("h_flip".to_string(), "true".to_string());
        }
        if cell.v_flip {
            metadata.insert("v_flip".to_string(), "true".to_string());
        }

        let color_hex = format!(
            "#{:02x}{:02x}{:02x}",
            cell.color_rgb[0], cell.color_rgb[1], cell.color_rgb[2]
        );

        instructions.push(PaintInstruction::Rect(PaintRect {
            base: PaintBase {
                id: Some(format!("tile:{}:{}", cell.grid_col, cell.grid_row)),
                metadata: Some(metadata),
            },
            x,
            y,
            width: tile_width,
            height: tile_height,
            fill: Some(color_hex),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }));
    }

    Ok(PaintScene {
        width: scene_width,
        height: scene_height,
        background: background.to_string(),
        instructions,
        id: None,
        metadata: None,
    })
}

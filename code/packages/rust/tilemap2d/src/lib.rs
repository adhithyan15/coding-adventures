//! # tilemap2d
//!
//! GAME21: a deterministic, zero-allocation, multi-layer scrolling background
//! engine in the spirit of the NES PPU, Game Boy BG, SNES / GBA backgrounds.
//!
//! * **Layers** ([`TileLayer`]): a borrowed grid of [`TileCell`]s with
//!   parallax scrolling (Q8 fixed point), per-axis [`WrapMode`], horizontal /
//!   vertical tile flipping, palette bank index and per-tile attribute bits.
//! * **Culling** ([`cull_layer`]): the visible tile rectangle of a layer for
//!   a [`Viewport`] and [`Camera`], computed with checked `i64` arithmetic.
//! * **Collision** (`attr_at_point`, `aabb_attrs`, `aabb_blocked`, ...):
//!   point and AABB queries against the solid / one-way / water / hazard /
//!   trigger attribute bits ([`ATTR_SOLID`], [`ATTR_ONE_WAY`], ...).
//! * **Lowering** ([`lower_layer_cells_into`], [`lower_layers_to_paint_scene`]):
//!   visible tiles become `paint-batch-tiles` (`P2D09`) cells or a
//!   `paint-instructions` (`P2D00`) scene.
//! * **Indexed scanline rendering** ([`render_layer_scanline`],
//!   [`render_layers_to_container`]): tiles are stored packed in a
//!   [`Tileset`] (`palette-indexed`, `GAME20`), unpacked one tile row at a
//!   time and expanded through [`Palette`] banks with the hardware blend
//!   modes of `paint-palette-blend` (`P2D10`).
//!
//! ## Conventions
//!
//! * Tile id `0` ([`EMPTY_TILE`]) is the blank tile: it is never drawn and
//!   never lowered (it still carries collision attributes).
//! * Parallax is Q8 fixed point: [`PARALLAX_ONE`] (256) is 1.0. The effective
//!   scroll is `floor(camera * parallax / 256) + layer.scroll`.
//! * [`WrapMode::Wrap`] repeats the map; [`WrapMode::Clamp`] clamps the
//!   scroll so the viewport never leaves the map (tiles outside are void);
//!   [`WrapMode::Void`] lets the camera roam freely over empty space.
//!
//! ## Safety and resource model
//!
//! * `#![forbid(unsafe_code)]`; no pointer arithmetic.
//! * Hot paths (cull, collision, scanline rendering) never touch the heap;
//!   the only allocation is the explicitly allocating
//!   [`lower_layers_to_paint_scene`], which builds an owned scene.
//! * Dimensions, tile sizes, layer counts, viewports, collision query areas,
//!   bank counts and tile ids are all bounded (CWE-20 / CWE-400); every size
//!   and coordinate computation is checked (CWE-190); divisors are validated
//!   non-zero (CWE-369); buffers, strides and indices are validated before
//!   any read or write (CWE-125 / CWE-787).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Generated capability verification witnesses.
pub mod capabilities;

pub use paint_batch_tiles::{PaintBase, PaintInstruction, PaintRect, PaintScene, TileBatchCell};
pub use paint_palette_blend::BlendMode;
pub use palette_indexed::{BitDepth, Palette};
pub use pixel_container::PixelContainer;

use paint_palette_blend::{blit_indexed_scanline, BlitParams};
use palette_indexed::{packed_row_len, unpack_row_into};

/// An RGBA8 colour: `[red, green, blue, alpha]`.
pub type Rgba8 = [u8; 4];

/// An RGB8 colour used as a lowering tint: `[red, green, blue]`.
pub type Rgb8 = [u8; 3];

/// Largest accepted map width or height in tiles (CWE-400).
pub const MAX_MAP_DIM: u32 = 4096;

/// Largest accepted number of cells in one map (CWE-400).
pub const MAX_MAP_CELLS: usize = 1 << 22;

/// Largest accepted tile width or height in pixels (CWE-400).
pub const MAX_TILE_SIZE: u32 = 256;

/// Largest accepted viewport / render-target width or height in pixels.
pub const MAX_VIEWPORT: u32 = 8192;

/// Largest accepted number of layers in one render or lowering call.
pub const MAX_LAYERS: usize = 8;

/// Largest accepted number of tiles inspected by one collision query.
pub const MAX_QUERY_TILES: usize = 65_536;

/// Largest accepted number of palette banks.
pub const MAX_PALETTE_BANKS: usize = 256;

/// Largest accepted number of tiles in a [`Tileset`] (the `u16` id space).
pub const MAX_TILES: u32 = 65_536;

/// The blank tile id: never drawn, never lowered.
pub const EMPTY_TILE: u16 = 0;

/// Parallax factor `1.0` in Q8 fixed point.
pub const PARALLAX_ONE: i32 = 256;

/// [`TileCell::flags`] bit: mirror the tile horizontally.
pub const FLAG_FLIP_H: u8 = 0b01;

/// [`TileCell::flags`] bit: mirror the tile vertically.
pub const FLAG_FLIP_V: u8 = 0b10;

/// All defined [`TileCell::flags`] bits.
pub const FLAG_MASK: u8 = FLAG_FLIP_H | FLAG_FLIP_V;

/// Attribute bit: blocks movement from every direction.
pub const ATTR_SOLID: u8 = 0b0_0001;

/// Attribute bit: blocks only downward motion that started above the tile.
pub const ATTR_ONE_WAY: u8 = 0b0_0010;

/// Attribute bit: water volume.
pub const ATTR_WATER: u8 = 0b0_0100;

/// Attribute bit: damaging hazard (spikes, lava).
pub const ATTR_HAZARD: u8 = 0b0_1000;

/// Attribute bit: event trigger.
pub const ATTR_TRIGGER: u8 = 0b1_0000;

/// All defined attribute bits.
pub const ATTR_MASK: u8 = ATTR_SOLID | ATTR_ONE_WAY | ATTR_WATER | ATTR_HAZARD | ATTR_TRIGGER;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failure modes of every fallible operation in this crate.
#[derive(Clone, Debug, PartialEq)]
pub enum TilemapError {
    /// Map width or height is zero or exceeds [`MAX_MAP_DIM`].
    InvalidMapSize {
        /// Map width in tiles.
        cols: u32,
        /// Map height in tiles.
        rows: u32,
    },
    /// Tile width or height is zero or exceeds [`MAX_TILE_SIZE`].
    InvalidTileSize {
        /// Tile width in pixels.
        width: u32,
        /// Tile height in pixels.
        height: u32,
    },
    /// The layer and tileset disagree on the tile size.
    TileSizeMismatch,
    /// A size computation overflowed (CWE-190).
    SizeOverflow,
    /// A buffer is shorter than the operation needs.
    BufferTooSmall {
        /// Elements (or bytes) needed.
        required: usize,
        /// Elements (or bytes) supplied.
        actual: usize,
    },
    /// Viewport or render target width / height is zero or too large.
    InvalidViewport {
        /// Viewport width in pixels.
        width: u32,
        /// Viewport height in pixels.
        height: u32,
    },
    /// A scanline index is outside the viewport.
    ScanlineOutOfRange {
        /// Requested scanline.
        y: u32,
        /// Viewport height.
        height: u32,
    },
    /// More than [`MAX_LAYERS`] layers were supplied.
    TooManyLayers {
        /// Layers supplied.
        count: usize,
    },
    /// A cell carries flag bits outside [`FLAG_MASK`].
    InvalidFlags {
        /// The offending flag byte.
        flags: u8,
    },
    /// A cell carries attribute bits outside [`ATTR_MASK`].
    InvalidAttr {
        /// The offending attribute byte.
        attr: u8,
    },
    /// A tile id is not below the tileset's tile count.
    TileIdOutOfRange {
        /// The offending tile id.
        tile_id: u16,
        /// Exclusive upper bound.
        limit: u32,
    },
    /// A palette bank index is not below the number of banks.
    PaletteBankOutOfRange {
        /// The offending bank index.
        bank: u8,
        /// Number of banks available.
        limit: usize,
    },
    /// Too many palette banks were supplied.
    TooManyBanks {
        /// Banks supplied.
        count: usize,
    },
    /// A palette bank and the tileset have different bit depths.
    DepthMismatch,
    /// A tileset row index or tile row is outside the tile.
    TileRowOutOfRange,
    /// An AABB has zero width or height.
    InvalidAabb,
    /// A collision query would inspect more than [`MAX_QUERY_TILES`] tiles.
    QueryTooLarge {
        /// Tiles the query would inspect.
        tiles: usize,
    },
    /// A lowered scene would hold more instructions than the batch limit.
    SceneTooLarge {
        /// Instructions the scene would hold.
        count: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Error reported by `palette-indexed`.
    Palette(palette_indexed::PaletteError),
    /// Error reported by `paint-palette-blend`.
    Blend(paint_palette_blend::PaletteBlendError),
    /// Error reported by `paint-batch-tiles`.
    Batch(paint_batch_tiles::PaintBatchError),
}

impl std::fmt::Display for TilemapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMapSize { cols, rows } => {
                write!(f, "invalid map size {cols}x{rows} (max {MAX_MAP_DIM})")
            }
            Self::InvalidTileSize { width, height } => {
                write!(
                    f,
                    "invalid tile size {width}x{height} (max {MAX_TILE_SIZE})"
                )
            }
            Self::TileSizeMismatch => write!(f, "layer and tileset tile sizes differ"),
            Self::SizeOverflow => write!(f, "size computation overflowed"),
            Self::BufferTooSmall { required, actual } => {
                write!(f, "buffer too small: need {required}, have {actual}")
            }
            Self::InvalidViewport { width, height } => {
                write!(f, "invalid viewport {width}x{height} (max {MAX_VIEWPORT})")
            }
            Self::ScanlineOutOfRange { y, height } => {
                write!(f, "scanline {y} outside viewport height {height}")
            }
            Self::TooManyLayers { count } => {
                write!(f, "{count} layers exceed maximum {MAX_LAYERS}")
            }
            Self::InvalidFlags { flags } => write!(f, "invalid tile flags {flags:#04x}"),
            Self::InvalidAttr { attr } => write!(f, "invalid tile attributes {attr:#04x}"),
            Self::TileIdOutOfRange { tile_id, limit } => {
                write!(f, "tile id {tile_id} out of range (limit {limit})")
            }
            Self::PaletteBankOutOfRange { bank, limit } => {
                write!(f, "palette bank {bank} out of range (limit {limit})")
            }
            Self::TooManyBanks { count } => {
                write!(
                    f,
                    "{count} palette banks exceed maximum {MAX_PALETTE_BANKS}"
                )
            }
            Self::DepthMismatch => write!(f, "palette bank and tileset bit depth differ"),
            Self::TileRowOutOfRange => write!(f, "tile row outside tile"),
            Self::InvalidAabb => write!(f, "aabb width and height must be non-zero"),
            Self::QueryTooLarge { tiles } => {
                write!(f, "query inspects {tiles} tiles (max {MAX_QUERY_TILES})")
            }
            Self::SceneTooLarge { count, max } => {
                write!(f, "scene of {count} instructions exceeds maximum {max}")
            }
            Self::Palette(e) => write!(f, "palette-indexed: {e}"),
            Self::Blend(e) => write!(f, "paint-palette-blend: {e}"),
            Self::Batch(e) => write!(f, "paint-batch-tiles: {e}"),
        }
    }
}

impl std::error::Error for TilemapError {}

// ---------------------------------------------------------------------------
// Cells and layers
// ---------------------------------------------------------------------------

/// One map cell: tile id, flip flags, palette bank and attribute bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TileCell {
    /// Index into the [`Tileset`]; [`EMPTY_TILE`] is blank.
    pub tile_id: u16,
    /// Flip bits ([`FLAG_FLIP_H`], [`FLAG_FLIP_V`]).
    pub flags: u8,
    /// Palette bank index.
    pub palette_bank: u8,
    /// Collision / attribute bits ([`ATTR_SOLID`], ...).
    pub attr: u8,
}

impl TileCell {
    /// A cell showing `tile_id` with no flips, bank 0 and no attributes.
    pub const fn tile(tile_id: u16) -> Self {
        Self {
            tile_id,
            flags: 0,
            palette_bank: 0,
            attr: 0,
        }
    }

    /// Sets the horizontal / vertical flip flags.
    pub const fn with_flip(mut self, h_flip: bool, v_flip: bool) -> Self {
        let mut flags = 0;
        if h_flip {
            flags |= FLAG_FLIP_H;
        }
        if v_flip {
            flags |= FLAG_FLIP_V;
        }
        self.flags = flags;
        self
    }

    /// Sets the palette bank.
    pub const fn with_bank(mut self, bank: u8) -> Self {
        self.palette_bank = bank;
        self
    }

    /// Sets the attribute bits.
    pub const fn with_attr(mut self, attr: u8) -> Self {
        self.attr = attr;
        self
    }

    /// True when the tile is mirrored horizontally.
    pub const fn flips_h(self) -> bool {
        self.flags & FLAG_FLIP_H != 0
    }

    /// True when the tile is mirrored vertically.
    pub const fn flips_v(self) -> bool {
        self.flags & FLAG_FLIP_V != 0
    }

    /// True when the tile is the blank tile.
    pub const fn is_blank(self) -> bool {
        self.tile_id == EMPTY_TILE
    }
}

/// Behaviour of a layer axis outside the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapMode {
    /// The map repeats forever.
    Wrap,
    /// The scroll is clamped so the viewport stays inside the map; tiles
    /// outside the map are void.
    Clamp,
    /// The scroll is unrestricted; tiles outside the map are void.
    Void,
}

/// A borrowed, validated tile grid with its scroll configuration.
///
/// Geometry (`cols`, `rows`, tile size) is private so the invariants checked
/// by [`TileLayer::attach`] always hold; scroll and presentation settings are
/// plain public fields.
#[derive(Clone, Copy, Debug)]
pub struct TileLayer<'a> {
    cells: &'a [TileCell],
    cols: u32,
    rows: u32,
    tile_w: u32,
    tile_h: u32,
    /// Static scroll offset added after parallax, in pixels.
    pub scroll_x: i32,
    /// Static scroll offset added after parallax, in pixels.
    pub scroll_y: i32,
    /// Horizontal parallax factor in Q8 ([`PARALLAX_ONE`] = 1.0).
    pub parallax_x_q8: i32,
    /// Vertical parallax factor in Q8 ([`PARALLAX_ONE`] = 1.0).
    pub parallax_y_q8: i32,
    /// Horizontal out-of-map behaviour.
    pub wrap_x: WrapMode,
    /// Vertical out-of-map behaviour.
    pub wrap_y: WrapMode,
    /// Attribute bits reported for tiles outside the map (non-wrapping axes).
    pub edge_attr: u8,
    /// Hidden layers are skipped by lowering and rendering.
    pub visible: bool,
    /// Treat palette index 0 as transparent when rendering.
    pub color0_transparent: bool,
    /// Blend mode used when compositing the layer.
    pub blend: BlendMode,
    /// Global alpha of the layer (0 = invisible, 255 = full).
    pub alpha: u8,
}

impl<'a> TileLayer<'a> {
    /// Validates and borrows a `cols x rows` grid of `cells` with
    /// `tile_w x tile_h` pixel tiles.
    ///
    /// Defaults: no scroll, parallax 1.0, [`WrapMode::Wrap`] on both axes,
    /// visible, opaque colour 0, [`BlendMode::Normal`], alpha 255.
    ///
    /// # Errors
    ///
    /// [`TilemapError::InvalidMapSize`], [`TilemapError::InvalidTileSize`],
    /// [`TilemapError::SizeOverflow`], [`TilemapError::BufferTooSmall`],
    /// [`TilemapError::InvalidFlags`] or [`TilemapError::InvalidAttr`].
    pub fn attach(
        cells: &'a [TileCell],
        cols: u32,
        rows: u32,
        tile_w: u32,
        tile_h: u32,
    ) -> Result<Self, TilemapError> {
        if cols == 0 || rows == 0 || cols > MAX_MAP_DIM || rows > MAX_MAP_DIM {
            return Err(TilemapError::InvalidMapSize { cols, rows });
        }
        check_tile_size(tile_w, tile_h)?;
        let need = (cols as usize)
            .checked_mul(rows as usize)
            .ok_or(TilemapError::SizeOverflow)?;
        if need > MAX_MAP_CELLS {
            return Err(TilemapError::InvalidMapSize { cols, rows });
        }
        let used = cells.get(..need).ok_or(TilemapError::BufferTooSmall {
            required: need,
            actual: cells.len(),
        })?;
        for cell in used {
            if cell.flags & !FLAG_MASK != 0 {
                return Err(TilemapError::InvalidFlags { flags: cell.flags });
            }
            if cell.attr & !ATTR_MASK != 0 {
                return Err(TilemapError::InvalidAttr { attr: cell.attr });
            }
        }
        Ok(Self {
            cells: used,
            cols,
            rows,
            tile_w,
            tile_h,
            scroll_x: 0,
            scroll_y: 0,
            parallax_x_q8: PARALLAX_ONE,
            parallax_y_q8: PARALLAX_ONE,
            wrap_x: WrapMode::Wrap,
            wrap_y: WrapMode::Wrap,
            edge_attr: 0,
            visible: true,
            color0_transparent: true,
            blend: BlendMode::Normal,
            alpha: 255,
        })
    }

    /// Map width in tiles.
    pub const fn layer_cols(&self) -> u32 {
        self.cols
    }

    /// Map height in tiles.
    pub const fn layer_rows(&self) -> u32 {
        self.rows
    }

    /// Tile width in pixels.
    pub const fn layer_tile_w(&self) -> u32 {
        self.tile_w
    }

    /// Tile height in pixels.
    pub const fn layer_tile_h(&self) -> u32 {
        self.tile_h
    }

    /// The `cols * rows` cells, row-major.
    pub const fn layer_cells(&self) -> &'a [TileCell] {
        self.cells
    }

    /// Map width in pixels.
    pub const fn world_px_w(&self) -> i64 {
        self.cols as i64 * self.tile_w as i64
    }

    /// Map height in pixels.
    pub const fn world_px_h(&self) -> i64 {
        self.rows as i64 * self.tile_h as i64
    }
}

fn check_tile_size(width: u32, height: u32) -> Result<(), TilemapError> {
    if width == 0 || height == 0 || width > MAX_TILE_SIZE || height > MAX_TILE_SIZE {
        Err(TilemapError::InvalidTileSize { width, height })
    } else {
        Ok(())
    }
}

/// Resolves a tile coordinate on one axis to a grid index, or `None` when it
/// lies in void space.
fn resolve_tile_axis(value: i64, count: u32, mode: WrapMode) -> Option<usize> {
    let n = i64::from(count.max(1));
    match mode {
        WrapMode::Wrap => usize::try_from(value.rem_euclid(n)).ok(),
        WrapMode::Clamp | WrapMode::Void => {
            if value >= 0 && value < n {
                usize::try_from(value).ok()
            } else {
                None
            }
        }
    }
}

/// Reads the cell at tile coordinate `(col, row)`, honouring each axis'
/// [`WrapMode`]. Returns `None` for void space.
pub fn cell_at_tile(layer: &TileLayer<'_>, col: i64, row: i64) -> Option<TileCell> {
    let c = resolve_tile_axis(col, layer.cols, layer.wrap_x)?;
    let r = resolve_tile_axis(row, layer.rows, layer.wrap_y)?;
    let index = r.checked_mul(layer.cols as usize)?.checked_add(c)?;
    layer.cells.get(index).copied()
}

// ---------------------------------------------------------------------------
// Camera, viewport, scroll
// ---------------------------------------------------------------------------

/// The camera position in world pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Camera {
    /// Horizontal camera position.
    pub x: i32,
    /// Vertical camera position.
    pub y: i32,
}

/// The visible window in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    /// Width in pixels (`1..=`[`MAX_VIEWPORT`]).
    pub width: u32,
    /// Height in pixels (`1..=`[`MAX_VIEWPORT`]).
    pub height: u32,
}

/// A pixel position or scroll offset in layer space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixelPos {
    /// Horizontal coordinate.
    pub x: i64,
    /// Vertical coordinate.
    pub y: i64,
}

/// Validates a viewport.
///
/// # Errors
///
/// [`TilemapError::InvalidViewport`] when a dimension is zero or too large.
pub fn validate_viewport(view: Viewport) -> Result<(), TilemapError> {
    if view.width == 0
        || view.height == 0
        || view.width > MAX_VIEWPORT
        || view.height > MAX_VIEWPORT
    {
        Err(TilemapError::InvalidViewport {
            width: view.width,
            height: view.height,
        })
    } else {
        Ok(())
    }
}

fn resolve_scroll_axis(raw: i64, mode: WrapMode, world: i64, view: i64) -> i64 {
    match mode {
        WrapMode::Wrap => raw.rem_euclid(world.max(1)),
        WrapMode::Clamp => raw.clamp(0, (world - view).max(0)),
        WrapMode::Void => raw,
    }
}

fn raw_scroll(camera: i32, parallax_q8: i32, offset: i32) -> Result<i64, TilemapError> {
    i64::from(camera)
        .checked_mul(i64::from(parallax_q8))
        .map(|v| v >> 8)
        .and_then(|v| v.checked_add(i64::from(offset)))
        .ok_or(TilemapError::SizeOverflow)
}

/// The effective scroll offset of `layer`: its world-space pixel position of
/// the viewport's top-left corner, after parallax, wrap and clamp rules.
///
/// # Errors
///
/// [`TilemapError::InvalidViewport`] or [`TilemapError::SizeOverflow`].
pub fn layer_scroll(
    layer: &TileLayer<'_>,
    view: Viewport,
    camera: Camera,
) -> Result<PixelPos, TilemapError> {
    validate_viewport(view)?;
    let rx = raw_scroll(camera.x, layer.parallax_x_q8, layer.scroll_x)?;
    let ry = raw_scroll(camera.y, layer.parallax_y_q8, layer.scroll_y)?;
    Ok(PixelPos {
        x: resolve_scroll_axis(rx, layer.wrap_x, layer.world_px_w(), i64::from(view.width)),
        y: resolve_scroll_axis(ry, layer.wrap_y, layer.world_px_h(), i64::from(view.height)),
    })
}

/// Converts a screen position to a layer-space pixel position (suitable for
/// the collision queries).
///
/// # Errors
///
/// [`TilemapError::InvalidViewport`] or [`TilemapError::SizeOverflow`].
pub fn screen_to_layer(
    layer: &TileLayer<'_>,
    view: Viewport,
    camera: Camera,
    screen_x: i32,
    screen_y: i32,
) -> Result<PixelPos, TilemapError> {
    let scroll = layer_scroll(layer, view, camera)?;
    Ok(PixelPos {
        x: scroll
            .x
            .checked_add(i64::from(screen_x))
            .ok_or(TilemapError::SizeOverflow)?,
        y: scroll
            .y
            .checked_add(i64::from(screen_y))
            .ok_or(TilemapError::SizeOverflow)?,
    })
}

// ---------------------------------------------------------------------------
// Culling
// ---------------------------------------------------------------------------

/// The tile rectangle of a layer that intersects the viewport.
///
/// Tile `(first_col + i, first_row + j)` for `i < cols`, `j < rows` is drawn
/// with its top-left corner at screen position
/// `(offset_x + i * tile_w, offset_y + j * tile_h)`. For wrapping axes the
/// tile coordinates may lie outside the map and are resolved modulo its size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisibleRect {
    /// First visible tile column (unwrapped).
    pub first_col: i64,
    /// First visible tile row (unwrapped).
    pub first_row: i64,
    /// Number of visible tile columns (0 when nothing is visible).
    pub cols: u32,
    /// Number of visible tile rows (0 when nothing is visible).
    pub rows: u32,
    /// Screen x of the first visible column's left edge.
    pub offset_x: i64,
    /// Screen y of the first visible row's top edge.
    pub offset_y: i64,
}

struct AxisCull {
    first: i64,
    count: u32,
    offset: i64,
}

fn cull_axis(
    scroll: i64,
    view: i64,
    tile: i64,
    tiles: u32,
    mode: WrapMode,
) -> Result<AxisCull, TilemapError> {
    let tile = tile.max(1);
    let mut first = scroll.div_euclid(tile);
    let last_px = scroll
        .checked_add(view - 1)
        .ok_or(TilemapError::SizeOverflow)?;
    let mut last = last_px.div_euclid(tile);
    if mode != WrapMode::Wrap {
        first = first.max(0);
        last = last.min(i64::from(tiles) - 1);
        if first > last {
            return Ok(AxisCull {
                first: 0,
                count: 0,
                offset: 0,
            });
        }
    }
    let count = u32::try_from(last - first + 1).map_err(|_| TilemapError::SizeOverflow)?;
    let offset = first
        .checked_mul(tile)
        .and_then(|v| v.checked_sub(scroll))
        .ok_or(TilemapError::SizeOverflow)?;
    Ok(AxisCull {
        first,
        count,
        offset,
    })
}

fn cull_with_scroll(
    layer: &TileLayer<'_>,
    view: Viewport,
    scroll: PixelPos,
) -> Result<VisibleRect, TilemapError> {
    let ax = cull_axis(
        scroll.x,
        i64::from(view.width),
        i64::from(layer.tile_w),
        layer.cols,
        layer.wrap_x,
    )?;
    let ay = cull_axis(
        scroll.y,
        i64::from(view.height),
        i64::from(layer.tile_h),
        layer.rows,
        layer.wrap_y,
    )?;
    if ax.count == 0 || ay.count == 0 {
        return Ok(VisibleRect::default());
    }
    Ok(VisibleRect {
        first_col: ax.first,
        first_row: ay.first,
        cols: ax.count,
        rows: ay.count,
        offset_x: ax.offset,
        offset_y: ay.offset,
    })
}

/// Computes the visible tile rectangle of `layer` for `view` and `camera`.
///
/// Pure integer arithmetic on `i64`; zero heap allocation.
///
/// # Errors
///
/// [`TilemapError::InvalidViewport`] or [`TilemapError::SizeOverflow`].
pub fn cull_layer(
    layer: &TileLayer<'_>,
    view: Viewport,
    camera: Camera,
) -> Result<VisibleRect, TilemapError> {
    let scroll = layer_scroll(layer, view, camera)?;
    cull_with_scroll(layer, view, scroll)
}

// ---------------------------------------------------------------------------
// Collision queries
// ---------------------------------------------------------------------------

/// An axis-aligned box in layer-space pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aabb {
    /// Left edge.
    pub x: i64,
    /// Top edge.
    pub y: i64,
    /// Width in pixels (non-zero).
    pub w: u32,
    /// Height in pixels (non-zero).
    pub h: u32,
}

/// Motion context for one-way platforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionHint {
    /// True when the box is moving downward this step.
    pub moving_down: bool,
    /// The box's bottom edge (exclusive) before the step.
    pub prev_bottom: i64,
}

/// The first tile found by [`aabb_first_hit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileHit {
    /// Tile column (unwrapped).
    pub col: i64,
    /// Tile row (unwrapped).
    pub row: i64,
    /// The tile's attribute bits.
    pub attr: u8,
}

/// Attribute bits of tile `(col, row)`; void tiles report
/// [`TileLayer::edge_attr`].
pub fn attr_at_tile(layer: &TileLayer<'_>, col: i64, row: i64) -> u8 {
    match cell_at_tile(layer, col, row) {
        Some(cell) => cell.attr & ATTR_MASK,
        None => layer.edge_attr & ATTR_MASK,
    }
}

/// Attribute bits at layer-space pixel `(x, y)`.
pub fn attr_at_point(layer: &TileLayer<'_>, x: i64, y: i64) -> u8 {
    let col = x.div_euclid(i64::from(layer.tile_w.max(1)));
    let row = y.div_euclid(i64::from(layer.tile_h.max(1)));
    attr_at_tile(layer, col, row)
}

/// True when the tile under pixel `(x, y)` has any bit of `mask` set.
pub fn point_has_attr(layer: &TileLayer<'_>, x: i64, y: i64, mask: u8) -> bool {
    attr_at_point(layer, x, y) & mask != 0
}

struct TileRange {
    c0: i64,
    c1: i64,
    r0: i64,
    r1: i64,
}

fn aabb_tile_range(layer: &TileLayer<'_>, aabb: Aabb) -> Result<TileRange, TilemapError> {
    if aabb.w == 0 || aabb.h == 0 {
        return Err(TilemapError::InvalidAabb);
    }
    let right = aabb
        .x
        .checked_add(i64::from(aabb.w) - 1)
        .ok_or(TilemapError::SizeOverflow)?;
    let bottom = aabb
        .y
        .checked_add(i64::from(aabb.h) - 1)
        .ok_or(TilemapError::SizeOverflow)?;
    let tw = i64::from(layer.tile_w.max(1));
    let th = i64::from(layer.tile_h.max(1));
    let range = TileRange {
        c0: aabb.x.div_euclid(tw),
        c1: right.div_euclid(tw),
        r0: aabb.y.div_euclid(th),
        r1: bottom.div_euclid(th),
    };
    let ncols = u64::try_from(range.c1 - range.c0 + 1).map_err(|_| TilemapError::SizeOverflow)?;
    let nrows = u64::try_from(range.r1 - range.r0 + 1).map_err(|_| TilemapError::SizeOverflow)?;
    let tiles = ncols
        .checked_mul(nrows)
        .ok_or(TilemapError::QueryTooLarge { tiles: usize::MAX })?;
    if tiles > MAX_QUERY_TILES as u64 {
        return Err(TilemapError::QueryTooLarge {
            tiles: usize::try_from(tiles).unwrap_or(usize::MAX),
        });
    }
    Ok(range)
}

/// Visits every tile overlapped by `aabb` in row-major order until `visit`
/// returns `true`.
fn scan_aabb<F: FnMut(i64, i64, u8) -> Result<bool, TilemapError>>(
    layer: &TileLayer<'_>,
    aabb: Aabb,
    mut visit: F,
) -> Result<(), TilemapError> {
    let range = aabb_tile_range(layer, aabb)?;
    for row in range.r0..=range.r1 {
        for col in range.c0..=range.c1 {
            if visit(col, row, attr_at_tile(layer, col, row))? {
                return Ok(());
            }
        }
    }
    Ok(())
}

/// The union of the attribute bits of every tile overlapped by `aabb`.
///
/// # Errors
///
/// [`TilemapError::InvalidAabb`], [`TilemapError::QueryTooLarge`] or
/// [`TilemapError::SizeOverflow`].
pub fn aabb_attrs(layer: &TileLayer<'_>, aabb: Aabb) -> Result<u8, TilemapError> {
    let mut union = 0u8;
    scan_aabb(layer, aabb, |_, _, attr| {
        union |= attr;
        Ok(false)
    })?;
    Ok(union)
}

/// True when any tile overlapped by `aabb` has a bit of `mask` set.
///
/// # Errors
///
/// Same as [`aabb_attrs`].
pub fn aabb_has_attr(layer: &TileLayer<'_>, aabb: Aabb, mask: u8) -> Result<bool, TilemapError> {
    Ok(aabb_first_hit(layer, aabb, mask)?.is_some())
}

/// The first tile (row-major) overlapped by `aabb` with a bit of `mask` set.
///
/// # Errors
///
/// Same as [`aabb_attrs`].
pub fn aabb_first_hit(
    layer: &TileLayer<'_>,
    aabb: Aabb,
    mask: u8,
) -> Result<Option<TileHit>, TilemapError> {
    let mut hit = None;
    scan_aabb(layer, aabb, |col, row, attr| {
        if attr & mask != 0 {
            hit = Some(TileHit { col, row, attr });
            Ok(true)
        } else {
            Ok(false)
        }
    })?;
    Ok(hit)
}

/// True when `aabb` is blocked: it overlaps an [`ATTR_SOLID`] tile, or an
/// [`ATTR_ONE_WAY`] tile while `motion.moving_down` and the box was entirely
/// above that tile's top edge (`prev_bottom <= tile_top`) before the step.
///
/// # Errors
///
/// Same as [`aabb_attrs`].
pub fn aabb_blocked(
    layer: &TileLayer<'_>,
    aabb: Aabb,
    motion: MotionHint,
) -> Result<bool, TilemapError> {
    let th = i64::from(layer.tile_h.max(1));
    let mut blocked = false;
    scan_aabb(layer, aabb, |_, row, attr| {
        if attr & ATTR_SOLID != 0 {
            blocked = true;
        } else if attr & ATTR_ONE_WAY != 0 && motion.moving_down {
            let top = row.checked_mul(th).ok_or(TilemapError::SizeOverflow)?;
            if motion.prev_bottom <= top {
                blocked = true;
            }
        }
        Ok(blocked)
    })?;
    Ok(blocked)
}

// ---------------------------------------------------------------------------
// Lowering into paint-batch-tiles / paint-instructions
// ---------------------------------------------------------------------------

fn bank_tint(bank_colors: &[Rgb8], bank: u8) -> Result<Rgb8, TilemapError> {
    bank_colors
        .get(usize::from(bank))
        .copied()
        .ok_or(TilemapError::PaletteBankOutOfRange {
            bank,
            limit: bank_colors.len(),
        })
}

/// Rejects hand-built rectangles larger than any real viewport could cull
/// (a culled rectangle has at most `MAX_VIEWPORT + 1` tiles per axis), so a
/// hostile `VisibleRect` cannot force an unbounded loop (CWE-400).
fn check_rect_extent(rect: &VisibleRect) -> Result<(), TilemapError> {
    let limit = MAX_VIEWPORT + 1;
    if rect.cols > limit || rect.rows > limit {
        Err(TilemapError::InvalidViewport {
            width: rect.cols,
            height: rect.rows,
        })
    } else {
        Ok(())
    }
}

/// Visits every non-blank visible cell of `rect` with its rect-relative
/// `(i, j)` position.
fn for_each_visible<F: FnMut(u32, u32, TileCell) -> Result<(), TilemapError>>(
    layer: &TileLayer<'_>,
    rect: &VisibleRect,
    mut visit: F,
) -> Result<(), TilemapError> {
    check_rect_extent(rect)?;
    for j in 0..rect.rows {
        let row = rect
            .first_row
            .checked_add(i64::from(j))
            .ok_or(TilemapError::SizeOverflow)?;
        for i in 0..rect.cols {
            let col = rect
                .first_col
                .checked_add(i64::from(i))
                .ok_or(TilemapError::SizeOverflow)?;
            if let Some(cell) = cell_at_tile(layer, col, row) {
                if !cell.is_blank() {
                    visit(i, j, cell)?;
                }
            }
        }
    }
    Ok(())
}

/// Lowers the visible non-blank cells of `rect` into `out` as
/// `paint-batch-tiles` cells (`grid_col` / `grid_row` are rect-relative).
/// Cell tints come from `bank_colors[palette_bank]`. Returns the number of
/// cells written. Zero heap allocation.
///
/// # Errors
///
/// [`TilemapError::BufferTooSmall`] (nothing is lost: `required` reports the
/// exact count needed), [`TilemapError::PaletteBankOutOfRange`] or
/// [`TilemapError::SizeOverflow`].
pub fn lower_layer_cells_into(
    layer: &TileLayer<'_>,
    rect: &VisibleRect,
    bank_colors: &[Rgb8],
    out: &mut [TileBatchCell],
) -> Result<usize, TilemapError> {
    let mut needed = 0usize;
    for_each_visible(layer, rect, |i, j, cell| {
        let tint = bank_tint(bank_colors, cell.palette_bank)?;
        let grid_col = u16::try_from(i).map_err(|_| TilemapError::SizeOverflow)?;
        let grid_row = u16::try_from(j).map_err(|_| TilemapError::SizeOverflow)?;
        if let Some(slot) = out.get_mut(needed) {
            *slot = TileBatchCell::new(grid_col, grid_row, cell.tile_id)
                .with_palette(cell.palette_bank)
                .with_flip(cell.flips_h(), cell.flips_v())
                .with_color(tint);
        }
        needed += 1;
        Ok(())
    })?;
    if needed > out.len() {
        return Err(TilemapError::BufferTooSmall {
            required: needed,
            actual: out.len(),
        });
    }
    Ok(needed)
}

/// Counts the visible non-blank cells of `rect`.
///
/// # Errors
///
/// [`TilemapError::SizeOverflow`].
pub fn count_visible_cells(
    layer: &TileLayer<'_>,
    rect: &VisibleRect,
) -> Result<usize, TilemapError> {
    let mut n = 0usize;
    for_each_visible(layer, rect, |_, _, _| {
        n += 1;
        Ok(())
    })?;
    Ok(n)
}

/// Lowers every visible layer into one [`PaintScene`] (`P2D00`), back to
/// front in layer order. Tiles become `Rect` instructions with stable ids
/// `layer<N>/tile:<col>:<row>` positioned with the sub-tile scroll offset
/// applied. This function allocates; it is not a hot path.
///
/// The total instruction count is capped at
/// [`paint_batch_tiles::MAX_BATCH_SPRITES`].
///
/// # Errors
///
/// Layer / viewport validation errors, [`TilemapError::SceneTooLarge`],
/// [`TilemapError::PaletteBankOutOfRange`] or [`TilemapError::Batch`].
pub fn lower_layers_to_paint_scene(
    layers: &[TileLayer<'_>],
    view: Viewport,
    camera: Camera,
    bank_colors: &[Rgb8],
    background: &str,
) -> Result<PaintScene, TilemapError> {
    validate_viewport(view)?;
    if layers.len() > MAX_LAYERS {
        return Err(TilemapError::TooManyLayers {
            count: layers.len(),
        });
    }
    let max = paint_batch_tiles::MAX_BATCH_SPRITES;
    let mut instructions: Vec<PaintInstruction> = Vec::new();
    for (index, layer) in layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        let rect = cull_layer(layer, view, camera)?;
        let count = count_visible_cells(layer, &rect)?;
        let total = instructions.len().saturating_add(count);
        if total > max {
            return Err(TilemapError::SceneTooLarge { count: total, max });
        }
        let mut cells = vec![TileBatchCell::new(0, 0, 0); count];
        let written = lower_layer_cells_into(layer, &rect, bank_colors, &mut cells)?;
        cells.truncate(written);
        let scene = paint_batch_tiles::lower_tile_batch_to_paint_scene(
            f64::from(view.width),
            f64::from(view.height),
            f64::from(layer.tile_w),
            f64::from(layer.tile_h),
            background,
            &cells,
        )
        .map_err(TilemapError::Batch)?;
        for instruction in scene.instructions {
            if let PaintInstruction::Rect(mut rect_instr) = instruction {
                rect_instr.x += rect.offset_x as f64;
                rect_instr.y += rect.offset_y as f64;
                let old = rect_instr.base.id.take().unwrap_or_default();
                rect_instr.base.id = Some(format!("layer{index}/{old}"));
                if let Some(meta) = rect_instr.base.metadata.as_mut() {
                    meta.insert("layer".to_string(), index.to_string());
                }
                instructions.push(PaintInstruction::Rect(rect_instr));
            }
        }
    }
    Ok(PaintScene {
        width: f64::from(view.width),
        height: f64::from(view.height),
        background: background.to_string(),
        instructions,
        id: None,
        metadata: None,
    })
}

// ---------------------------------------------------------------------------
// Tilesets
// ---------------------------------------------------------------------------

/// A borrowed set of packed indexed tiles (`palette-indexed` row packing).
///
/// Tile `n` occupies `tile_h * row_len` bytes starting at
/// `n * tile_h * row_len`; each tile row starts on a byte boundary.
#[derive(Clone, Copy, Debug)]
pub struct Tileset<'a> {
    depth: BitDepth,
    data: &'a [u8],
    tile_w: u32,
    tile_h: u32,
    tile_count: u32,
    row_len: usize,
    tile_bytes: usize,
}

impl<'a> Tileset<'a> {
    /// Validates and borrows `tile_count` packed tiles of `tile_w x tile_h`
    /// pixels at `depth`.
    ///
    /// # Errors
    ///
    /// [`TilemapError::InvalidTileSize`], [`TilemapError::TileIdOutOfRange`]
    /// (zero or more than [`MAX_TILES`] tiles), [`TilemapError::SizeOverflow`]
    /// or [`TilemapError::BufferTooSmall`].
    pub fn attach_tiles(
        depth: BitDepth,
        data: &'a [u8],
        tile_w: u32,
        tile_h: u32,
        tile_count: u32,
    ) -> Result<Self, TilemapError> {
        check_tile_size(tile_w, tile_h)?;
        if tile_count == 0 || tile_count > MAX_TILES {
            return Err(TilemapError::TileIdOutOfRange {
                tile_id: 0,
                limit: tile_count,
            });
        }
        let row_len = packed_row_len(depth, tile_w).map_err(TilemapError::Palette)?;
        let tile_bytes = row_len
            .checked_mul(tile_h as usize)
            .ok_or(TilemapError::SizeOverflow)?;
        let total = tile_bytes
            .checked_mul(tile_count as usize)
            .ok_or(TilemapError::SizeOverflow)?;
        if data.len() < total {
            return Err(TilemapError::BufferTooSmall {
                required: total,
                actual: data.len(),
            });
        }
        Ok(Self {
            depth,
            data,
            tile_w,
            tile_h,
            tile_count,
            row_len,
            tile_bytes,
        })
    }

    /// Number of tiles.
    pub const fn tiles_count(&self) -> u32 {
        self.tile_count
    }

    /// Pixel bit depth of the packed tiles.
    pub const fn tiles_depth(&self) -> BitDepth {
        self.depth
    }

    /// Unpacks row `y` of tile `tile_id` into `out[..tile_w]`, applying the
    /// vertical and horizontal flips. Zero heap allocation.
    ///
    /// # Errors
    ///
    /// [`TilemapError::TileIdOutOfRange`], [`TilemapError::TileRowOutOfRange`],
    /// [`TilemapError::BufferTooSmall`] or [`TilemapError::Palette`].
    pub fn unpack_tile_row(
        &self,
        tile_id: u16,
        y: u32,
        h_flip: bool,
        v_flip: bool,
        out: &mut [u8],
    ) -> Result<(), TilemapError> {
        if u32::from(tile_id) >= self.tile_count {
            return Err(TilemapError::TileIdOutOfRange {
                tile_id,
                limit: self.tile_count,
            });
        }
        if y >= self.tile_h {
            return Err(TilemapError::TileRowOutOfRange);
        }
        let ry = if v_flip { self.tile_h - 1 - y } else { y };
        let start = usize::from(tile_id)
            .checked_mul(self.tile_bytes)
            .and_then(|v| v.checked_add((ry as usize).checked_mul(self.row_len)?))
            .ok_or(TilemapError::SizeOverflow)?;
        let end = start
            .checked_add(self.row_len)
            .ok_or(TilemapError::SizeOverflow)?;
        let packed = self
            .data
            .get(start..end)
            .ok_or(TilemapError::BufferTooSmall {
                required: end,
                actual: self.data.len(),
            })?;
        unpack_row_into(self.depth, packed, self.tile_w, out).map_err(TilemapError::Palette)?;
        if h_flip {
            if let Some(row) = out.get_mut(..self.tile_w as usize) {
                row.reverse();
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Indexed scanline rendering
// ---------------------------------------------------------------------------

fn check_bank<'b>(
    banks: &'b [Palette],
    tileset: &Tileset<'_>,
    bank: u8,
) -> Result<&'b Palette, TilemapError> {
    let palette = banks
        .get(usize::from(bank))
        .ok_or(TilemapError::PaletteBankOutOfRange {
            bank,
            limit: banks.len(),
        })?;
    if palette.palette_depth() != tileset.depth {
        return Err(TilemapError::DepthMismatch);
    }
    Ok(palette)
}

fn check_render_inputs(
    layer: &TileLayer<'_>,
    tileset: &Tileset<'_>,
    banks: &[Palette],
) -> Result<(), TilemapError> {
    if layer.tile_w != tileset.tile_w || layer.tile_h != tileset.tile_h {
        return Err(TilemapError::TileSizeMismatch);
    }
    if banks.len() > MAX_PALETTE_BANKS {
        return Err(TilemapError::TooManyBanks { count: banks.len() });
    }
    Ok(())
}

/// Validates every visible cell of `rect` against the tileset and banks.
fn validate_visible(
    layer: &TileLayer<'_>,
    rect: &VisibleRect,
    tileset: &Tileset<'_>,
    banks: &[Palette],
) -> Result<(), TilemapError> {
    for_each_visible(layer, rect, |_, _, cell| {
        if u32::from(cell.tile_id) >= tileset.tile_count {
            return Err(TilemapError::TileIdOutOfRange {
                tile_id: cell.tile_id,
                limit: tileset.tile_count,
            });
        }
        check_bank(banks, tileset, cell.palette_bank)?;
        Ok(())
    })
}

/// Visits the non-blank cells of one pixel row of the visible rectangle.
///
/// `visit(cell, screen_x)` is called per tile with its left screen edge.
fn for_each_row_cell<F: FnMut(TileCell, i64) -> Result<(), TilemapError>>(
    layer: &TileLayer<'_>,
    rect: &VisibleRect,
    tile_row: i64,
    mut visit: F,
) -> Result<(), TilemapError> {
    check_rect_extent(rect)?;
    let tw = i64::from(layer.tile_w);
    for i in 0..rect.cols {
        let col = rect
            .first_col
            .checked_add(i64::from(i))
            .ok_or(TilemapError::SizeOverflow)?;
        let Some(cell) = cell_at_tile(layer, col, tile_row) else {
            continue;
        };
        if cell.is_blank() {
            continue;
        }
        let x = i64::from(i)
            .checked_mul(tw)
            .and_then(|v| v.checked_add(rect.offset_x))
            .ok_or(TilemapError::SizeOverflow)?;
        visit(cell, x)?;
    }
    Ok(())
}

/// Everything needed to draw the rows of one layer.
struct RowFrame<'f, 'a> {
    layer: &'f TileLayer<'a>,
    tileset: &'f Tileset<'a>,
    banks: &'f [Palette],
    scroll: PixelPos,
    rect: VisibleRect,
    view_w: u32,
}

/// Draws pixel row `y` (screen space) of one layer into `line`.
fn draw_row(frame: &RowFrame<'_, '_>, y: u32, line: &mut [u8]) -> Result<usize, TilemapError> {
    let RowFrame {
        layer,
        tileset,
        banks,
        scroll,
        rect,
        view_w,
    } = frame;
    let (layer, tileset, banks, scroll, view_w) = (*layer, *tileset, *banks, *scroll, *view_w);
    let world_y = scroll
        .y
        .checked_add(i64::from(y))
        .ok_or(TilemapError::SizeOverflow)?;
    let th = i64::from(layer.tile_h.max(1));
    let tile_row = world_y.div_euclid(th);
    if resolve_tile_axis(tile_row, layer.rows, layer.wrap_y).is_none() {
        return Ok(0);
    }
    let pixel_row =
        u32::try_from(world_y.rem_euclid(th)).map_err(|_| TilemapError::SizeOverflow)?;
    let tw = i64::from(layer.tile_w);
    let view_w = i64::from(view_w);

    // Pass 1: validate every tile of the row before any write.
    for_each_row_cell(layer, rect, tile_row, |cell, _| {
        if u32::from(cell.tile_id) >= tileset.tile_count {
            return Err(TilemapError::TileIdOutOfRange {
                tile_id: cell.tile_id,
                limit: tileset.tile_count,
            });
        }
        check_bank(banks, tileset, cell.palette_bank)?;
        Ok(())
    })?;

    // Pass 2: draw.
    let params = BlitParams {
        mode: layer.blend,
        alpha: layer.alpha,
        transparent_index: if layer.color0_transparent {
            Some(0)
        } else {
            None
        },
        h_flip: false,
        v_flip: false,
    };
    let mut scratch = [0u8; MAX_TILE_SIZE as usize];
    let mut drawn = 0usize;
    for_each_row_cell(layer, rect, tile_row, |cell, x| {
        let seg0 = x.max(0);
        let seg1 = x.saturating_add(tw).min(view_w);
        if seg0 >= seg1 {
            return Ok(());
        }
        let palette = check_bank(banks, tileset, cell.palette_bank)?;
        tileset.unpack_tile_row(
            cell.tile_id,
            pixel_row,
            cell.flips_h(),
            cell.flips_v(),
            &mut scratch,
        )?;
        let sx0 = usize::try_from(seg0 - x).map_err(|_| TilemapError::SizeOverflow)?;
        let sx1 = usize::try_from(seg1 - x).map_err(|_| TilemapError::SizeOverflow)?;
        let dx0 = usize::try_from(seg0).map_err(|_| TilemapError::SizeOverflow)?;
        let dx1 = usize::try_from(seg1).map_err(|_| TilemapError::SizeOverflow)?;
        let indices = scratch.get(sx0..sx1).ok_or(TilemapError::BufferTooSmall {
            required: sx1,
            actual: scratch.len(),
        })?;
        let required = dx1.checked_mul(4).ok_or(TilemapError::SizeOverflow)?;
        let dst = line
            .get_mut(dx0 * 4..required)
            .ok_or(TilemapError::BufferTooSmall {
                required,
                actual: 0,
            })?;
        drawn += blit_indexed_scanline(dst, indices, palette.active_entries(), &params)
            .map_err(TilemapError::Blend)?;
        Ok(())
    })?;
    Ok(drawn)
}

/// Renders scanline `y` of one layer into `line` (RGBA8, at least
/// `view.width * 4` bytes), compositing over the existing contents with the
/// layer's blend mode. Calling this per scanline with a different `camera`
/// gives raster (HBlank) scroll effects. Returns the number of pixels drawn.
/// Hidden layers draw nothing. Zero heap allocation.
///
/// # Errors
///
/// Viewport / tile-size / tile-id / bank / depth validation errors,
/// [`TilemapError::ScanlineOutOfRange`] or [`TilemapError::BufferTooSmall`].
/// On error no pixel of `line` has been modified.
pub fn render_layer_scanline(
    layer: &TileLayer<'_>,
    tileset: &Tileset<'_>,
    banks: &[Palette],
    view: Viewport,
    camera: Camera,
    y: u32,
    line: &mut [u8],
) -> Result<usize, TilemapError> {
    validate_viewport(view)?;
    check_render_inputs(layer, tileset, banks)?;
    if y >= view.height {
        return Err(TilemapError::ScanlineOutOfRange {
            y,
            height: view.height,
        });
    }
    let required = (view.width as usize)
        .checked_mul(4)
        .ok_or(TilemapError::SizeOverflow)?;
    if line.len() < required {
        return Err(TilemapError::BufferTooSmall {
            required,
            actual: line.len(),
        });
    }
    if !layer.visible {
        return Ok(0);
    }
    let scroll = layer_scroll(layer, view, camera)?;
    let rect = cull_with_scroll(layer, view, scroll)?;
    if rect.cols == 0 || rect.rows == 0 {
        return Ok(0);
    }
    let frame = RowFrame {
        layer,
        tileset,
        banks,
        scroll,
        rect,
        view_w: view.width,
    };
    draw_row(&frame, y, line)
}

/// Renders every visible layer, back to front, into `dst` (RGBA8). The
/// viewport is the container size. When `clear` is set the container is
/// first filled with that colour. Returns the total pixels drawn.
///
/// All layers are validated before the first byte is written, so a
/// validation error leaves `dst` untouched. Zero heap allocation.
///
/// # Errors
///
/// [`TilemapError::InvalidViewport`], [`TilemapError::BufferTooSmall`]
/// (container length), [`TilemapError::TooManyLayers`] and the per-layer
/// validation errors of [`render_layer_scanline`].
pub fn render_layers_to_container(
    layers: &[TileLayer<'_>],
    tileset: &Tileset<'_>,
    banks: &[Palette],
    camera: Camera,
    clear: Option<Rgba8>,
    dst: &mut PixelContainer,
) -> Result<usize, TilemapError> {
    let view = Viewport {
        width: dst.width,
        height: dst.height,
    };
    validate_viewport(view)?;
    if layers.len() > MAX_LAYERS {
        return Err(TilemapError::TooManyLayers {
            count: layers.len(),
        });
    }
    let stride = (view.width as usize)
        .checked_mul(4)
        .ok_or(TilemapError::SizeOverflow)?;
    let expected = stride
        .checked_mul(view.height as usize)
        .ok_or(TilemapError::SizeOverflow)?;
    if dst.data.len() != expected {
        return Err(TilemapError::BufferTooSmall {
            required: expected,
            actual: dst.data.len(),
        });
    }
    for layer in layers {
        check_render_inputs(layer, tileset, banks)?;
        if layer.visible {
            let scroll = layer_scroll(layer, view, camera)?;
            let rect = cull_with_scroll(layer, view, scroll)?;
            validate_visible(layer, &rect, tileset, banks)?;
        }
    }
    if let Some(color) = clear {
        for px in dst.data.as_mut_slice().as_chunks_mut::<4>().0.iter_mut() {
            *px = color;
        }
    }
    let mut drawn = 0usize;
    for layer in layers {
        if !layer.visible {
            continue;
        }
        let scroll = layer_scroll(layer, view, camera)?;
        let rect = cull_with_scroll(layer, view, scroll)?;
        if rect.cols == 0 || rect.rows == 0 {
            continue;
        }
        let frame = RowFrame {
            layer,
            tileset,
            banks,
            scroll,
            rect,
            view_w: view.width,
        };
        for y in 0..view.height {
            let start = (y as usize)
                .checked_mul(stride)
                .ok_or(TilemapError::SizeOverflow)?;
            let end = start
                .checked_add(stride)
                .ok_or(TilemapError::SizeOverflow)?;
            let actual = dst.data.len();
            let line = dst
                .data
                .get_mut(start..end)
                .ok_or(TilemapError::BufferTooSmall {
                    required: end,
                    actual,
                })?;
            drawn += draw_row(&frame, y, line)?;
        }
    }
    Ok(drawn)
}

//! # Conformance Tests for `tilemap2d` (GAME21)
//!
//! Covers:
//! - Cell builders, layer / tileset attachment and validation
//! - Parallax (Q8), wrap / clamp / void scroll resolution
//! - Viewport culling with safe `i64` arithmetic
//! - Point / AABB tile-attribute collision queries (solid, one-way, water, hazard, trigger)
//! - Lowering into `paint-batch-tiles` cells and `paint-instructions` scenes
//! - Indexed scanline rendering (flips, banks, colour-0, blend, wrap) against a brute-force reference
//! - CWE-20 / 125 / 190 / 369 / 400 / 787 guards

use coding_adventures_sha256::sha256_hex;
use paint_batch_tiles::{PaintInstruction, TileBatchCell};
use palette_indexed::{pack_row_into, BitDepth, Palette};
use tilemap2d::{
    aabb_attrs, aabb_blocked, aabb_first_hit, aabb_has_attr, attr_at_point, attr_at_tile,
    cell_at_tile, count_visible_cells, cull_layer, layer_scroll, lower_layer_cells_into,
    lower_layers_to_paint_scene, point_has_attr, render_layer_scanline, render_layers_to_container,
    screen_to_layer, validate_viewport, Aabb, BlendMode, Camera, MotionHint, PixelContainer,
    TileCell, TileLayer, TilemapError, Tileset, Viewport, WrapMode, ATTR_HAZARD, ATTR_MASK,
    ATTR_ONE_WAY, ATTR_SOLID, ATTR_TRIGGER, ATTR_WATER, EMPTY_TILE, FLAG_FLIP_H, FLAG_FLIP_V,
    MAX_LAYERS, MAX_MAP_DIM, MAX_PALETTE_BANKS, MAX_QUERY_TILES, MAX_TILES, MAX_TILE_SIZE,
    MAX_VIEWPORT, PARALLAX_ONE,
};

// --- helpers ---------------------------------------------------------------

/// Pixel value of tile `tile` at `(x, y)`: unique-ish, never 0 except for a
/// designated hole at `(0, 0)` of odd tiles.
fn tile_pixel(tile: u16, x: u32, y: u32) -> u8 {
    if tile % 2 == 1 && x == 0 && y == 0 {
        0
    } else {
        ((u32::from(tile) * 3 + x * 5 + y * 7) % 15 + 1) as u8
    }
}

fn make_tileset_data(depth: BitDepth, tw: u32, th: u32, count: u32) -> Vec<u8> {
    let row_len = palette_indexed::packed_row_len(depth, tw).unwrap();
    let mut data = vec![0u8; row_len * th as usize * count as usize];
    for tile in 0..count {
        for y in 0..th {
            let row: Vec<u8> = (0..tw).map(|x| tile_pixel(tile as u16, x, y)).collect();
            let off = (tile * th + y) as usize * row_len;
            pack_row_into(depth, &row, &mut data[off..off + row_len]).unwrap();
        }
    }
    data
}

fn bank(shift: u8) -> Palette {
    let mut p = Palette::blank(BitDepth::Bpp4);
    for i in 0..16u8 {
        p.set_entry(i, [i * 16, 255 - i * 16, shift, 255]).unwrap();
    }
    p
}

fn banks() -> [Palette; 2] {
    [bank(10), bank(200)]
}

fn grid(cols: u32, rows: u32, mut f: impl FnMut(u32, u32) -> TileCell) -> Vec<TileCell> {
    let mut v = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            v.push(f(c, r));
        }
    }
    v
}

fn px(c: &PixelContainer, x: u32, y: u32) -> [u8; 4] {
    let o = ((y * c.width + x) * 4) as usize;
    [c.data[o], c.data[o + 1], c.data[o + 2], c.data[o + 3]]
}

// --- cells -----------------------------------------------------------------

#[test]
fn cell_builders_and_flags() {
    let c = TileCell::tile(7)
        .with_flip(true, false)
        .with_bank(3)
        .with_attr(ATTR_SOLID | ATTR_WATER);
    assert_eq!(c.tile_id, 7);
    assert!(c.flips_h() && !c.flips_v());
    assert_eq!(c.flags, FLAG_FLIP_H);
    assert_eq!(c.palette_bank, 3);
    assert_eq!(c.attr, ATTR_SOLID | ATTR_WATER);
    assert!(!c.is_blank());
    assert!(TileCell::tile(EMPTY_TILE).is_blank());
    assert_eq!(
        TileCell::tile(1).with_flip(true, true).flags,
        FLAG_FLIP_H | FLAG_FLIP_V
    );
    assert_eq!(ATTR_MASK, 0b1_1111);
}

// --- attach ----------------------------------------------------------------

#[test]
fn layer_attach_validation() {
    let cells = vec![TileCell::tile(1); 6];
    let l = TileLayer::attach(&cells, 3, 2, 8, 8).unwrap();
    assert_eq!((l.layer_cols(), l.layer_rows()), (3, 2));
    assert_eq!((l.layer_tile_w(), l.layer_tile_h()), (8, 8));
    assert_eq!((l.world_px_w(), l.world_px_h()), (24, 16));
    assert_eq!(l.layer_cells().len(), 6);

    assert!(matches!(
        TileLayer::attach(&cells, 0, 2, 8, 8),
        Err(TilemapError::InvalidMapSize { .. })
    ));
    assert!(matches!(
        TileLayer::attach(&cells, 3, 0, 8, 8),
        Err(TilemapError::InvalidMapSize { .. })
    ));
    assert!(matches!(
        TileLayer::attach(&cells, MAX_MAP_DIM + 1, 1, 8, 8),
        Err(TilemapError::InvalidMapSize { .. })
    ));
    assert!(matches!(
        TileLayer::attach(&cells, MAX_MAP_DIM, MAX_MAP_DIM, 8, 8),
        Err(TilemapError::InvalidMapSize { .. })
    ));
    assert!(matches!(
        TileLayer::attach(&cells, u32::MAX, u32::MAX, 8, 8),
        Err(TilemapError::InvalidMapSize { .. })
    ));
    assert!(matches!(
        TileLayer::attach(&cells, 3, 2, 0, 8),
        Err(TilemapError::InvalidTileSize { .. })
    ));
    assert!(matches!(
        TileLayer::attach(&cells, 3, 2, 8, MAX_TILE_SIZE + 1),
        Err(TilemapError::InvalidTileSize { .. })
    ));
    assert_eq!(
        TileLayer::attach(&cells, 3, 3, 8, 8).unwrap_err(),
        TilemapError::BufferTooSmall {
            required: 9,
            actual: 6
        }
    );
}

#[test]
fn layer_attach_rejects_bad_cell_bits() {
    let mut cells = vec![TileCell::tile(1); 4];
    cells[2].flags = 0b100;
    assert_eq!(
        TileLayer::attach(&cells, 2, 2, 8, 8).unwrap_err(),
        TilemapError::InvalidFlags { flags: 0b100 }
    );
    cells[2].flags = 0;
    cells[3].attr = 0x80;
    assert_eq!(
        TileLayer::attach(&cells, 2, 2, 8, 8).unwrap_err(),
        TilemapError::InvalidAttr { attr: 0x80 }
    );
}

#[test]
fn layer_attach_ignores_trailing_cells() {
    let mut cells = vec![TileCell::tile(1); 5];
    cells[4].attr = 0xFF; // beyond cols*rows: never validated or read
    let l = TileLayer::attach(&cells, 2, 2, 4, 4).unwrap();
    assert_eq!(l.layer_cells().len(), 4);
}

#[test]
fn tileset_attach_validation() {
    let data = make_tileset_data(BitDepth::Bpp4, 4, 4, 3);
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 3).unwrap();
    assert_eq!(ts.tiles_count(), 3);
    assert_eq!(ts.tiles_depth(), BitDepth::Bpp4);
    assert!(matches!(
        Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 4),
        Err(TilemapError::BufferTooSmall { .. })
    ));
    assert!(matches!(
        Tileset::attach_tiles(BitDepth::Bpp4, &data, 0, 4, 3),
        Err(TilemapError::InvalidTileSize { .. })
    ));
    assert!(matches!(
        Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 0),
        Err(TilemapError::TileIdOutOfRange { .. })
    ));
    assert!(matches!(
        Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, MAX_TILES + 1),
        Err(TilemapError::TileIdOutOfRange { .. })
    ));
}

#[test]
fn tileset_unpack_rows_and_flips() {
    for depth in [BitDepth::Bpp4, BitDepth::Bpp8] {
        let data = make_tileset_data(depth, 5, 3, 4);
        let ts = Tileset::attach_tiles(depth, &data, 5, 3, 4).unwrap();
        let mut out = [0u8; 8];
        for tile in 0..4u16 {
            for y in 0..3 {
                for (h, v) in [(false, false), (true, false), (false, true), (true, true)] {
                    ts.unpack_tile_row(tile, y, h, v, &mut out).unwrap();
                    let sy = if v { 2 - y } else { y };
                    for x in 0..5 {
                        let sx = if h { 4 - x } else { x };
                        assert_eq!(out[x as usize], tile_pixel(tile, sx, sy));
                    }
                }
            }
        }
    }
}

#[test]
fn tileset_unpack_rejects_bad_arguments() {
    let data = make_tileset_data(BitDepth::Bpp4, 4, 4, 2);
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 2).unwrap();
    let mut out = [0u8; 4];
    assert!(matches!(
        ts.unpack_tile_row(2, 0, false, false, &mut out),
        Err(TilemapError::TileIdOutOfRange {
            tile_id: 2,
            limit: 2
        })
    ));
    assert!(matches!(
        ts.unpack_tile_row(0, 4, false, false, &mut out),
        Err(TilemapError::TileRowOutOfRange)
    ));
    let mut small = [0u8; 2];
    assert!(matches!(
        ts.unpack_tile_row(0, 0, false, false, &mut small),
        Err(TilemapError::Palette(_))
    ));
}

// --- scroll ----------------------------------------------------------------

fn plain_layer(cells: &[TileCell], cols: u32, rows: u32) -> TileLayer<'_> {
    TileLayer::attach(cells, cols, rows, 8, 8).unwrap()
}

#[test]
fn viewport_validation() {
    assert!(validate_viewport(Viewport {
        width: 1,
        height: 1
    })
    .is_ok());
    assert!(validate_viewport(Viewport {
        width: MAX_VIEWPORT,
        height: MAX_VIEWPORT
    })
    .is_ok());
    for (w, h) in [(0, 1), (1, 0), (MAX_VIEWPORT + 1, 1), (1, MAX_VIEWPORT + 1)] {
        assert_eq!(
            validate_viewport(Viewport {
                width: w,
                height: h
            }),
            Err(TilemapError::InvalidViewport {
                width: w,
                height: h
            })
        );
    }
}

#[test]
fn scroll_parallax_q8() {
    let cells = vec![TileCell::tile(1); 64 * 64];
    let mut l = plain_layer(&cells, 64, 64);
    l.wrap_x = WrapMode::Void;
    l.wrap_y = WrapMode::Void;
    let v = Viewport {
        width: 32,
        height: 32,
    };
    let cam = Camera { x: 100, y: -100 };
    let s = layer_scroll(&l, v, cam).unwrap();
    assert_eq!((s.x, s.y), (100, -100));
    l.parallax_x_q8 = PARALLAX_ONE / 2;
    l.parallax_y_q8 = 0;
    l.scroll_x = 3;
    l.scroll_y = -4;
    let s = layer_scroll(&l, v, cam).unwrap();
    assert_eq!((s.x, s.y), (53, -4));
    // Q8 shifts are floor, not truncation: -101 * 128 >> 8 = -51.
    l.parallax_x_q8 = 128;
    l.scroll_x = 0;
    let s = layer_scroll(&l, v, Camera { x: -101, y: 0 }).unwrap();
    assert_eq!(s.x, -51);
}

#[test]
fn scroll_wrap_normalises_negative() {
    let cells = vec![TileCell::tile(1); 4 * 4];
    let l = plain_layer(&cells, 4, 4); // 32 px world
    let v = Viewport {
        width: 16,
        height: 16,
    };
    let s = layer_scroll(&l, v, Camera { x: -1, y: 33 }).unwrap();
    assert_eq!((s.x, s.y), (31, 1));
    let s = layer_scroll(
        &l,
        v,
        Camera {
            x: i32::MIN,
            y: i32::MAX,
        },
    )
    .unwrap();
    assert!(s.x >= 0 && s.x < 32 && s.y >= 0 && s.y < 32);
}

#[test]
fn scroll_clamp_keeps_viewport_inside() {
    let cells = vec![TileCell::tile(1); 4 * 4];
    let mut l = plain_layer(&cells, 4, 4); // 32 px world
    l.wrap_x = WrapMode::Clamp;
    l.wrap_y = WrapMode::Clamp;
    let v = Viewport {
        width: 20,
        height: 10,
    };
    let s = layer_scroll(&l, v, Camera { x: -50, y: 500 }).unwrap();
    assert_eq!((s.x, s.y), (0, 22));
    let s = layer_scroll(&l, v, Camera { x: 500, y: -5 }).unwrap();
    assert_eq!((s.x, s.y), (12, 0));
    // A viewport larger than the map pins the scroll at 0.
    let big = Viewport {
        width: 64,
        height: 64,
    };
    let s = layer_scroll(&l, big, Camera { x: 9, y: 9 }).unwrap();
    assert_eq!((s.x, s.y), (0, 0));
}

#[test]
fn scroll_extremes_do_not_overflow() {
    let cells = vec![TileCell::tile(1); 4];
    let mut l = plain_layer(&cells, 2, 2);
    l.parallax_x_q8 = i32::MAX;
    l.parallax_y_q8 = i32::MIN;
    l.scroll_x = i32::MAX;
    l.scroll_y = i32::MIN;
    l.wrap_x = WrapMode::Void;
    l.wrap_y = WrapMode::Void;
    let v = Viewport {
        width: 8,
        height: 8,
    };
    for cam in [
        Camera {
            x: i32::MAX,
            y: i32::MAX,
        },
        Camera {
            x: i32::MIN,
            y: i32::MIN,
        },
        Camera {
            x: i32::MAX,
            y: i32::MIN,
        },
    ] {
        assert!(layer_scroll(&l, v, cam).is_ok());
        assert!(cull_layer(&l, v, cam).is_ok());
    }
}

#[test]
fn screen_to_layer_adds_scroll() {
    let cells = vec![TileCell::tile(1); 16];
    let l = plain_layer(&cells, 4, 4);
    let v = Viewport {
        width: 16,
        height: 16,
    };
    let p = screen_to_layer(&l, v, Camera { x: 5, y: 6 }, 3, -2).unwrap();
    assert_eq!((p.x, p.y), (8, 4));
}

// --- culling ---------------------------------------------------------------

#[test]
fn cull_aligned_and_subtile() {
    let cells = vec![TileCell::tile(1); 8 * 8];
    let l = plain_layer(&cells, 8, 8);
    let v = Viewport {
        width: 32,
        height: 16,
    };
    let r = cull_layer(&l, v, Camera { x: 0, y: 0 }).unwrap();
    assert_eq!((r.first_col, r.first_row, r.cols, r.rows), (0, 0, 4, 2));
    assert_eq!((r.offset_x, r.offset_y), (0, 0));
    let r = cull_layer(&l, v, Camera { x: 3, y: 5 }).unwrap();
    assert_eq!((r.first_col, r.first_row, r.cols, r.rows), (0, 0, 5, 3));
    assert_eq!((r.offset_x, r.offset_y), (-3, -5));
    let r = cull_layer(&l, v, Camera { x: 8, y: 8 }).unwrap();
    assert_eq!((r.first_col, r.first_row, r.cols, r.rows), (1, 1, 4, 2));
    assert_eq!((r.offset_x, r.offset_y), (0, 0));
}

#[test]
fn cull_wrap_extends_past_map() {
    let cells = vec![TileCell::tile(1); 2 * 2];
    let l = plain_layer(&cells, 2, 2); // 16 px world
    let v = Viewport {
        width: 40,
        height: 8,
    };
    let r = cull_layer(&l, v, Camera { x: 12, y: 0 }).unwrap();
    assert_eq!((r.first_col, r.cols), (1, 6));
    assert_eq!(r.offset_x, -4);
    assert_eq!(r.rows, 1);
    // Wrapped lookups resolve into the map.
    assert!(cell_at_tile(&l, r.first_col + 5, 0).is_some());
}

#[test]
fn cull_clamp_and_void_clip_to_map() {
    let cells = vec![TileCell::tile(1); 4 * 4];
    let mut l = plain_layer(&cells, 4, 4); // 32 px world
    l.wrap_x = WrapMode::Void;
    l.wrap_y = WrapMode::Void;
    let v = Viewport {
        width: 16,
        height: 16,
    };
    let r = cull_layer(&l, v, Camera { x: -8, y: -8 }).unwrap();
    assert_eq!((r.first_col, r.first_row, r.cols, r.rows), (0, 0, 1, 1));
    assert_eq!((r.offset_x, r.offset_y), (8, 8));
    let r = cull_layer(&l, v, Camera { x: 28, y: 0 }).unwrap();
    assert_eq!((r.first_col, r.cols), (3, 1));
    assert_eq!(r.offset_x, -4);
    // Entirely outside the map: nothing visible.
    let r = cull_layer(&l, v, Camera { x: 1000, y: 0 }).unwrap();
    assert_eq!((r.cols, r.rows), (0, 0));
    assert_eq!(count_visible_cells(&l, &r).unwrap(), 0);
    let r = cull_layer(&l, v, Camera { x: -1000, y: -1000 }).unwrap();
    assert_eq!((r.cols, r.rows), (0, 0));
}

#[test]
fn cull_rejects_bad_viewport() {
    let cells = vec![TileCell::tile(1); 4];
    let l = plain_layer(&cells, 2, 2);
    assert!(matches!(
        cull_layer(
            &l,
            Viewport {
                width: 0,
                height: 4
            },
            Camera::default()
        ),
        Err(TilemapError::InvalidViewport { .. })
    ));
}

#[test]
fn cull_covers_every_visible_pixel() {
    // Every screen pixel must fall inside the culled rectangle, for many cameras.
    let cells = vec![TileCell::tile(1); 5 * 3];
    for wrap in [WrapMode::Wrap, WrapMode::Void, WrapMode::Clamp] {
        let mut l = TileLayer::attach(&cells, 5, 3, 7, 6).unwrap();
        l.wrap_x = wrap;
        l.wrap_y = wrap;
        let v = Viewport {
            width: 29,
            height: 21,
        };
        for cx in (-80..80).step_by(7) {
            for cy in (-60..60).step_by(5) {
                let cam = Camera { x: cx, y: cy };
                let r = cull_layer(&l, v, cam).unwrap();
                let s = layer_scroll(&l, v, cam).unwrap();
                let in_x = |sx: i64| {
                    let col = (s.x + sx).div_euclid(7);
                    (wrap == WrapMode::Wrap || (0..5).contains(&col)).then_some(col)
                };
                let in_y = |sy: i64| {
                    let row = (s.y + sy).div_euclid(6);
                    (wrap == WrapMode::Wrap || (0..3).contains(&row)).then_some(row)
                };
                for sy in 0..i64::from(v.height) {
                    for sx in 0..i64::from(v.width) {
                        if let (Some(col), Some(row)) = (in_x(sx), in_y(sy)) {
                            let i = col - r.first_col;
                            let j = row - r.first_row;
                            assert!(
                                i >= 0 && i < i64::from(r.cols) && j >= 0 && j < i64::from(r.rows),
                                "({sx},{sy}) cam ({cx},{cy}) {wrap:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}

// --- collision -------------------------------------------------------------

fn collision_map() -> Vec<TileCell> {
    // 4x3 map, 8px tiles:
    //  . S W H
    //  O T . .
    //  S S S S
    let a = |attr| TileCell::tile(1).with_attr(attr);
    vec![
        a(0),
        a(ATTR_SOLID),
        a(ATTR_WATER),
        a(ATTR_HAZARD),
        a(ATTR_ONE_WAY),
        a(ATTR_TRIGGER),
        a(0),
        a(0),
        a(ATTR_SOLID),
        a(ATTR_SOLID),
        a(ATTR_SOLID),
        a(ATTR_SOLID),
    ]
}

#[test]
fn point_attribute_queries() {
    let cells = collision_map();
    let l = plain_layer(&cells, 4, 3);
    assert_eq!(attr_at_tile(&l, 1, 0), ATTR_SOLID);
    assert_eq!(attr_at_point(&l, 8, 0), ATTR_SOLID);
    assert_eq!(attr_at_point(&l, 15, 7), ATTR_SOLID);
    assert_eq!(attr_at_point(&l, 16, 7), ATTR_WATER);
    assert_eq!(attr_at_point(&l, 24, 0), ATTR_HAZARD);
    assert_eq!(attr_at_point(&l, 0, 8), ATTR_ONE_WAY);
    assert_eq!(attr_at_point(&l, 8, 8), ATTR_TRIGGER);
    assert!(point_has_attr(&l, 8, 8, ATTR_TRIGGER | ATTR_HAZARD));
    assert!(!point_has_attr(&l, 0, 0, ATTR_MASK));
}

#[test]
fn point_queries_wrap_and_edge() {
    let cells = collision_map();
    let mut l = plain_layer(&cells, 4, 3); // 32 x 24 px
                                           // Wrap: (-1, 0) is the last column of row 0 = hazard.
    assert_eq!(attr_at_point(&l, -1, 0), ATTR_HAZARD);
    assert_eq!(attr_at_point(&l, 32 + 8, 0), ATTR_SOLID);
    assert_eq!(attr_at_point(&l, 0, 24 + 8), ATTR_ONE_WAY);
    // Void with edge attr: outside the map reads edge_attr.
    l.wrap_x = WrapMode::Void;
    l.wrap_y = WrapMode::Clamp;
    l.edge_attr = ATTR_SOLID | 0x80; // undefined bits are masked
    assert_eq!(attr_at_point(&l, -1, 0), ATTR_SOLID);
    assert_eq!(attr_at_point(&l, 0, -1), ATTR_SOLID);
    assert_eq!(attr_at_point(&l, 32, 0), ATTR_SOLID);
    assert_eq!(attr_at_point(&l, i64::MAX, i64::MIN), ATTR_SOLID);
    l.edge_attr = 0;
    assert_eq!(attr_at_point(&l, -1, 0), 0);
}

#[test]
fn aabb_union_and_first_hit() {
    let cells = collision_map();
    let l = plain_layer(&cells, 4, 3);
    let b = Aabb {
        x: 4,
        y: 4,
        w: 8,
        h: 8,
    }; // overlaps tiles (0,0)(1,0)(0,1)(1,1)
    assert_eq!(
        aabb_attrs(&l, b).unwrap(),
        ATTR_SOLID | ATTR_ONE_WAY | ATTR_TRIGGER
    );
    assert!(aabb_has_attr(&l, b, ATTR_TRIGGER).unwrap());
    assert!(!aabb_has_attr(&l, b, ATTR_WATER).unwrap());
    let hit = aabb_first_hit(&l, b, ATTR_ONE_WAY | ATTR_TRIGGER)
        .unwrap()
        .unwrap();
    assert_eq!((hit.col, hit.row, hit.attr), (0, 1, ATTR_ONE_WAY));
    assert_eq!(aabb_first_hit(&l, b, ATTR_HAZARD).unwrap(), None);
    // 1x1 box on a tile boundary touches exactly one tile.
    let one = Aabb {
        x: 8,
        y: 0,
        w: 1,
        h: 1,
    };
    assert_eq!(aabb_attrs(&l, one).unwrap(), ATTR_SOLID);
    // A box ending exactly on a tile edge does not touch the next tile.
    let edge = Aabb {
        x: 0,
        y: 0,
        w: 8,
        h: 8,
    };
    assert_eq!(aabb_attrs(&l, edge).unwrap(), 0);
}

#[test]
fn aabb_negative_coordinates() {
    let cells = collision_map();
    let l = plain_layer(&cells, 4, 3);
    let b = Aabb {
        x: -4,
        y: -4,
        w: 8,
        h: 8,
    }; // wraps onto the opposite corner tiles
    assert_eq!(aabb_attrs(&l, b).unwrap(), ATTR_HAZARD | ATTR_SOLID);
}

#[test]
fn aabb_rejects_bad_boxes() {
    let cells = collision_map();
    let l = plain_layer(&cells, 4, 3);
    assert_eq!(
        aabb_attrs(
            &l,
            Aabb {
                x: 0,
                y: 0,
                w: 0,
                h: 4
            }
        ),
        Err(TilemapError::InvalidAabb)
    );
    assert_eq!(
        aabb_attrs(
            &l,
            Aabb {
                x: 0,
                y: 0,
                w: 4,
                h: 0
            }
        ),
        Err(TilemapError::InvalidAabb)
    );
    assert_eq!(
        aabb_attrs(
            &l,
            Aabb {
                x: i64::MAX,
                y: 0,
                w: 8,
                h: 8
            }
        ),
        Err(TilemapError::SizeOverflow)
    );
    assert_eq!(
        aabb_attrs(
            &l,
            Aabb {
                x: 0,
                y: i64::MAX - 1,
                w: 8,
                h: 8
            }
        ),
        Err(TilemapError::SizeOverflow)
    );
}

#[test]
fn aabb_query_size_is_capped() {
    let cells = collision_map();
    let l = plain_layer(&cells, 4, 3);
    // 8px tiles: a 2048 x 2048 box inspects 256 x 256 = 65536 tiles (allowed).
    assert_eq!(MAX_QUERY_TILES, 65_536);
    assert!(aabb_attrs(
        &l,
        Aabb {
            x: 0,
            y: 0,
            w: 2048,
            h: 2048
        }
    )
    .is_ok());
    // One more tile column is rejected.
    assert!(matches!(
        aabb_attrs(
            &l,
            Aabb {
                x: 0,
                y: 0,
                w: 2049,
                h: 2048
            }
        ),
        Err(TilemapError::QueryTooLarge { .. })
    ));
    assert!(matches!(
        aabb_attrs(
            &l,
            Aabb {
                x: 0,
                y: 0,
                w: u32::MAX,
                h: u32::MAX
            }
        ),
        Err(TilemapError::QueryTooLarge { .. })
    ));
}

#[test]
fn one_way_platforms_block_only_from_above() {
    let cells = collision_map();
    let l = plain_layer(&cells, 4, 3); // one-way tile at (0,1): top = 8
    let falling_from_above = MotionHint {
        moving_down: true,
        prev_bottom: 8,
    };
    let box_in_tile = Aabb {
        x: 0,
        y: 4,
        w: 8,
        h: 8,
    }; // overlaps row 0 and 1 of col 0
    assert!(aabb_blocked(&l, box_in_tile, falling_from_above).unwrap());
    // Started inside the platform: passes through.
    let inside = MotionHint {
        moving_down: true,
        prev_bottom: 12,
    };
    assert!(!aabb_blocked(&l, box_in_tile, inside).unwrap());
    // Moving up: passes through.
    let up = MotionHint {
        moving_down: false,
        prev_bottom: 8,
    };
    assert!(!aabb_blocked(&l, box_in_tile, up).unwrap());
    // Solid tiles always block regardless of motion.
    let solid_box = Aabb {
        x: 8,
        y: 0,
        w: 4,
        h: 4,
    };
    assert!(aabb_blocked(&l, solid_box, up).unwrap());
    // Water / hazard / trigger never block.
    let water = Aabb {
        x: 16,
        y: 0,
        w: 16,
        h: 8,
    };
    assert!(!aabb_blocked(&l, water, falling_from_above).unwrap());
}

// --- lowering --------------------------------------------------------------

fn tints() -> [[u8; 3]; 2] {
    [[10, 20, 30], [200, 100, 50]]
}

#[test]
fn lower_cells_skip_blank_and_carry_attributes() {
    let cells = grid(4, 4, |c, r| {
        if (c + r) % 2 == 0 {
            TileCell::tile(EMPTY_TILE)
        } else {
            TileCell::tile(5)
                .with_flip(c % 2 == 0, r % 2 == 0)
                .with_bank((c % 2) as u8)
        }
    });
    let l = plain_layer(&cells, 4, 4);
    let v = Viewport {
        width: 32,
        height: 32,
    };
    let r = cull_layer(&l, v, Camera::default()).unwrap();
    assert_eq!(count_visible_cells(&l, &r).unwrap(), 8);
    let mut out = [TileBatchCell::new(0, 0, 0); 8];
    let n = lower_layer_cells_into(&l, &r, &tints(), &mut out).unwrap();
    assert_eq!(n, 8);
    assert_eq!((out[0].grid_col, out[0].grid_row), (1, 0));
    assert_eq!(out[0].tile_id, 5);
    assert_eq!(out[0].palette_id, 1);
    assert_eq!(out[0].color_rgb, [200, 100, 50]);
    assert!(!out[0].h_flip && out[0].v_flip);
    assert_eq!((out[7].grid_col, out[7].grid_row), (2, 3));
}

#[test]
fn cwe_400_hostile_visible_rect_is_rejected() {
    let cells = grid(2, 2, |_, _| TileCell::tile(1));
    let l = plain_layer(&cells, 2, 2);
    let hostile = tilemap2d::VisibleRect {
        first_col: 0,
        first_row: 0,
        cols: u32::MAX,
        rows: u32::MAX,
        offset_x: 0,
        offset_y: 0,
    };
    assert!(matches!(
        count_visible_cells(&l, &hostile),
        Err(TilemapError::InvalidViewport { .. })
    ));
    let mut out = [TileBatchCell::new(0, 0, 0); 4];
    assert!(matches!(
        lower_layer_cells_into(&l, &hostile, &tints(), &mut out),
        Err(TilemapError::InvalidViewport { .. })
    ));
    // The largest real rectangle (1px tiles across an 8192px viewport) is accepted.
    let cells1 = vec![TileCell::tile(1); 4];
    let l1 = TileLayer::attach(&cells1, 2, 2, 1, 1).unwrap();
    let r = cull_layer(
        &l1,
        Viewport {
            width: MAX_VIEWPORT,
            height: 1,
        },
        Camera { x: 1, y: 0 },
    )
    .unwrap();
    assert!(r.cols <= MAX_VIEWPORT + 1);
}

#[test]
fn lower_cells_error_paths() {
    let cells = grid(2, 2, |_, _| TileCell::tile(1).with_bank(1));
    let l = plain_layer(&cells, 2, 2);
    let v = Viewport {
        width: 16,
        height: 16,
    };
    let r = cull_layer(&l, v, Camera::default()).unwrap();
    let mut small = [TileBatchCell::new(0, 0, 0); 3];
    assert_eq!(
        lower_layer_cells_into(&l, &r, &tints(), &mut small),
        Err(TilemapError::BufferTooSmall {
            required: 4,
            actual: 3
        })
    );
    let mut out = [TileBatchCell::new(0, 0, 0); 4];
    assert_eq!(
        lower_layer_cells_into(&l, &r, &[[0, 0, 0]], &mut out),
        Err(TilemapError::PaletteBankOutOfRange { bank: 1, limit: 1 })
    );
}

#[test]
fn lower_scene_positions_ids_and_order() {
    let back = grid(2, 2, |_, _| TileCell::tile(1));
    let front = grid(2, 2, |c, r| {
        if c == 0 && r == 0 {
            TileCell::tile(2).with_bank(1)
        } else {
            TileCell::tile(EMPTY_TILE)
        }
    });
    let mut l0 = plain_layer(&back, 2, 2);
    l0.parallax_x_q8 = 0;
    l0.parallax_y_q8 = 0;
    let l1 = plain_layer(&front, 2, 2);
    let v = Viewport {
        width: 16,
        height: 16,
    };
    let scene =
        lower_layers_to_paint_scene(&[l0, l1], v, Camera { x: 3, y: 2 }, &tints(), "#000000")
            .unwrap();
    assert_eq!((scene.width, scene.height), (16.0, 16.0));
    assert_eq!(scene.background, "#000000");
    // Layer 0: parallax 0 => scroll 0 => 4 aligned tiles; layer 1: scroll (3,2).
    let rects: Vec<_> = scene
        .instructions
        .iter()
        .map(|i| match i {
            PaintInstruction::Rect(r) => r,
            _ => panic!("expected rect"),
        })
        .collect();
    assert!(rects.len() >= 5);
    let first = rects[0];
    assert_eq!(first.base.id.as_deref(), Some("layer0/tile:0:0"));
    assert_eq!((first.x, first.y), (0.0, 0.0));
    let last = *rects
        .iter()
        .find(|r| r.base.id.as_deref() == Some("layer1/tile:0:0"))
        .expect("layer1 first tile");
    assert_eq!(rects.len(), 8);
    assert_eq!((last.x, last.y), (-3.0, -2.0));
    assert_eq!(last.fill.as_deref(), Some("#c86432"));
    let meta = last.base.metadata.as_ref().unwrap();
    assert_eq!(meta.get("layer").map(String::as_str), Some("1"));
    assert_eq!(meta.get("palette_id").map(String::as_str), Some("1"));
    // Ids are unique.
    let mut ids: Vec<_> = rects.iter().map(|r| r.base.id.clone().unwrap()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), rects.len());
}

#[test]
fn lower_scene_skips_hidden_layers_and_validates() {
    let cells = grid(2, 2, |_, _| TileCell::tile(1));
    let mut l = plain_layer(&cells, 2, 2);
    l.visible = false;
    let v = Viewport {
        width: 16,
        height: 16,
    };
    let scene = lower_layers_to_paint_scene(&[l], v, Camera::default(), &tints(), "#fff").unwrap();
    assert!(scene.instructions.is_empty());

    let layers = vec![plain_layer(&cells, 2, 2); MAX_LAYERS + 1];
    assert_eq!(
        lower_layers_to_paint_scene(&layers, v, Camera::default(), &tints(), "#fff"),
        Err(TilemapError::TooManyLayers {
            count: MAX_LAYERS + 1
        })
    );
    assert!(matches!(
        lower_layers_to_paint_scene(
            &[plain_layer(&cells, 2, 2)],
            Viewport {
                width: 0,
                height: 1
            },
            Camera::default(),
            &tints(),
            "#fff"
        ),
        Err(TilemapError::InvalidViewport { .. })
    ));
}

#[test]
fn lower_scene_enforces_instruction_cap() {
    let cells = grid(80, 64, |_, _| TileCell::tile(1));
    let l = TileLayer::attach(&cells, 80, 64, 8, 8).unwrap();
    // 80 x 64 = 5120 visible tiles > 4096.
    let v = Viewport {
        width: 640,
        height: 512,
    };
    assert!(matches!(
        lower_layers_to_paint_scene(&[l], v, Camera::default(), &tints(), "#000"),
        Err(TilemapError::SceneTooLarge {
            count: 5120,
            max: 4096
        })
    ));
}

// --- rendering -------------------------------------------------------------

fn render_setup() -> (Vec<u8>, [Palette; 2]) {
    (make_tileset_data(BitDepth::Bpp4, 4, 4, 8), banks())
}

#[test]
fn render_single_tile_exact_pixels_and_flips() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    for (h, v) in [(false, false), (true, false), (false, true), (true, true)] {
        let cells = vec![TileCell::tile(2).with_flip(h, v)];
        let mut l = TileLayer::attach(&cells, 1, 1, 4, 4).unwrap();
        l.color0_transparent = false;
        let mut dst = PixelContainer::new(4, 4);
        let drawn =
            render_layers_to_container(&[l], &ts, &bk, Camera::default(), None, &mut dst).unwrap();
        assert_eq!(drawn, 16);
        for y in 0..4 {
            for x in 0..4 {
                let sx = if h { 3 - x } else { x };
                let sy = if v { 3 - y } else { y };
                let idx = tile_pixel(2, sx, sy);
                assert_eq!(
                    px(&dst, x, y),
                    bk[0].entry(idx).unwrap(),
                    "({x},{y}) {h} {v}"
                );
            }
        }
    }
}

#[test]
fn render_color0_transparency_and_clear() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    // Tile 1 has index 0 at (0,0).
    let cells = vec![TileCell::tile(1)];
    let mut l = TileLayer::attach(&cells, 1, 1, 4, 4).unwrap();
    let mut dst = PixelContainer::new(4, 4);
    let bg = [1, 2, 3, 255];
    let drawn =
        render_layers_to_container(&[l], &ts, &bk, Camera::default(), Some(bg), &mut dst).unwrap();
    assert_eq!(drawn, 15);
    assert_eq!(px(&dst, 0, 0), bg);
    assert_ne!(px(&dst, 1, 0), bg);
    // Opaque colour 0 draws the hole too.
    l.color0_transparent = false;
    let drawn =
        render_layers_to_container(&[l], &ts, &bk, Camera::default(), Some(bg), &mut dst).unwrap();
    assert_eq!(drawn, 16);
    assert_eq!(px(&dst, 0, 0), bk[0].entry(0).unwrap());
}

#[test]
fn render_matches_reference_for_random_maps() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let mut seed = 0x1234_5678_9abc_def0u64;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as u32
    };
    for case in 0..24 {
        let cols = 3 + next() % 5;
        let rows = 2 + next() % 5;
        let cells = grid(cols, rows, |_, _| {
            TileCell::tile((next() % 8) as u16)
                .with_flip(next() % 2 == 0, next() % 2 == 0)
                .with_bank((next() % 2) as u8)
        });
        let mut l = TileLayer::attach(&cells, cols, rows, 4, 4).unwrap();
        let modes = [WrapMode::Wrap, WrapMode::Clamp, WrapMode::Void];
        l.wrap_x = modes[(next() % 3) as usize];
        l.wrap_y = modes[(next() % 3) as usize];
        l.parallax_x_q8 = (next() % 400) as i32;
        l.parallax_y_q8 = (next() % 400) as i32;
        l.scroll_x = (next() % 21) as i32 - 10;
        l.scroll_y = (next() % 21) as i32 - 10;
        l.color0_transparent = next() % 2 == 0;
        let cam = Camera {
            x: (next() % 201) as i32 - 100,
            y: (next() % 201) as i32 - 100,
        };
        let (w, h) = (1 + next() % 40, 1 + next() % 30);
        let v = Viewport {
            width: w,
            height: h,
        };
        let mut dst = PixelContainer::new(w, h);
        let bg = [9, 9, 9, 255];
        render_layers_to_container(&[l], &ts, &bk, cam, Some(bg), &mut dst).unwrap();

        let s = layer_scroll(&l, v, cam).unwrap();
        for y in 0..h {
            for x in 0..w {
                let wx = s.x + i64::from(x);
                let wy = s.y + i64::from(y);
                let mut expect = bg;
                if let Some(cell) = cell_at_tile(&l, wx.div_euclid(4), wy.div_euclid(4)) {
                    if cell.tile_id != EMPTY_TILE {
                        let mut tx = wx.rem_euclid(4) as u32;
                        let mut ty = wy.rem_euclid(4) as u32;
                        if cell.flips_h() {
                            tx = 3 - tx;
                        }
                        if cell.flips_v() {
                            ty = 3 - ty;
                        }
                        let idx = tile_pixel(cell.tile_id, tx, ty);
                        if !(idx == 0 && l.color0_transparent) {
                            expect = bk[usize::from(cell.palette_bank)].entry(idx).unwrap();
                        }
                    }
                }
                assert_eq!(px(&dst, x, y), expect, "case {case} at ({x},{y})");
            }
        }
    }
}

#[test]
fn render_layers_composite_in_order_with_blend() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let back = vec![TileCell::tile(2)];
    let front = vec![TileCell::tile(2).with_bank(1)];
    let mut l0 = TileLayer::attach(&back, 1, 1, 4, 4).unwrap();
    let mut l1 = TileLayer::attach(&front, 1, 1, 4, 4).unwrap();
    l0.color0_transparent = false;
    l1.color0_transparent = false;
    let mut dst = PixelContainer::new(4, 4);
    render_layers_to_container(&[l0, l1], &ts, &bk, Camera::default(), None, &mut dst).unwrap();
    assert_eq!(px(&dst, 2, 1), bk[1].entry(tile_pixel(2, 2, 1)).unwrap());

    // Additive front layer saturates onto the back layer.
    l1.blend = BlendMode::Add;
    l1.alpha = 255;
    render_layers_to_container(
        &[l0, l1],
        &ts,
        &bk,
        Camera::default(),
        Some([0, 0, 0, 255]),
        &mut dst,
    )
    .unwrap();
    let a = bk[0].entry(tile_pixel(2, 2, 1)).unwrap();
    let b = bk[1].entry(tile_pixel(2, 2, 1)).unwrap();
    let got = px(&dst, 2, 1);
    for ch in 0..3 {
        assert_eq!(got[ch], a[ch].saturating_add(b[ch]), "channel {ch}");
    }

    // Hidden layers draw nothing.
    l0.visible = false;
    l1.visible = false;
    let drawn = render_layers_to_container(
        &[l0, l1],
        &ts,
        &bk,
        Camera::default(),
        Some([5, 5, 5, 255]),
        &mut dst,
    )
    .unwrap();
    assert_eq!(drawn, 0);
    assert_eq!(px(&dst, 0, 0), [5, 5, 5, 255]);
}

#[test]
fn render_scanline_supports_raster_scroll() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let cells = grid(4, 1, |c, _| TileCell::tile(c as u16 + 2));
    let mut l = TileLayer::attach(&cells, 4, 1, 4, 4).unwrap();
    l.color0_transparent = false;
    let v = Viewport {
        width: 8,
        height: 4,
    };
    let mut line = [0u8; 32];
    let n = render_layer_scanline(&l, &ts, &bk, v, Camera { x: 0, y: 0 }, 1, &mut line).unwrap();
    assert_eq!(n, 8);
    assert_eq!(&line[0..4], &bk[0].entry(tile_pixel(2, 0, 1)).unwrap());
    // A different camera for the next scanline shifts it by 4 pixels (one tile).
    let n = render_layer_scanline(&l, &ts, &bk, v, Camera { x: 4, y: 0 }, 2, &mut line).unwrap();
    assert_eq!(n, 8);
    assert_eq!(&line[0..4], &bk[0].entry(tile_pixel(3, 0, 2)).unwrap());
}

#[test]
fn render_void_rows_and_hidden_draw_nothing() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let cells = vec![TileCell::tile(2); 4];
    let mut l = TileLayer::attach(&cells, 2, 2, 4, 4).unwrap();
    l.color0_transparent = false;
    l.wrap_x = WrapMode::Void;
    l.wrap_y = WrapMode::Void;
    let v = Viewport {
        width: 8,
        height: 4,
    };
    let mut line = [7u8; 32];
    let n = render_layer_scanline(&l, &ts, &bk, v, Camera { x: 0, y: 100 }, 0, &mut line).unwrap();
    assert_eq!(n, 0);
    assert!(line.iter().all(|&b| b == 7));
    l.visible = false;
    let n = render_layer_scanline(&l, &ts, &bk, v, Camera::default(), 0, &mut line).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn render_partial_edge_tiles_clip() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let cells = vec![TileCell::tile(3); 4];
    let mut l = TileLayer::attach(&cells, 2, 2, 4, 4).unwrap();
    l.color0_transparent = false;
    let mut dst = PixelContainer::new(5, 3);
    // Camera shifted by (3,2): a 5x3 window starting mid-tile; every pixel is covered.
    let drawn =
        render_layers_to_container(&[l], &ts, &bk, Camera { x: 3, y: 2 }, None, &mut dst).unwrap();
    assert_eq!(drawn, 15);
    assert_eq!(px(&dst, 0, 0), bk[0].entry(tile_pixel(3, 3, 2)).unwrap());
    assert_eq!(px(&dst, 1, 0), bk[0].entry(tile_pixel(3, 0, 2)).unwrap());
}

#[test]
fn render_error_paths_leave_target_untouched() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let sentinel = [9u8, 8, 7, 6];
    let fresh = || {
        let mut c = PixelContainer::new(4, 4);
        for chunk in c.data.chunks_mut(4) {
            chunk.copy_from_slice(&sentinel);
        }
        c
    };

    // Tile id out of range.
    let bad_tile = vec![TileCell::tile(8)];
    let l = TileLayer::attach(&bad_tile, 1, 1, 4, 4).unwrap();
    let mut dst = fresh();
    assert!(matches!(
        render_layers_to_container(
            &[l],
            &ts,
            &bk,
            Camera::default(),
            Some([0, 0, 0, 0]),
            &mut dst
        ),
        Err(TilemapError::TileIdOutOfRange {
            tile_id: 8,
            limit: 8
        })
    ));
    assert!(dst.data.chunks(4).all(|c| c == sentinel));

    // Palette bank out of range.
    let bad_bank = vec![TileCell::tile(1).with_bank(2)];
    let l = TileLayer::attach(&bad_bank, 1, 1, 4, 4).unwrap();
    let mut dst = fresh();
    assert!(matches!(
        render_layers_to_container(&[l], &ts, &bk, Camera::default(), None, &mut dst),
        Err(TilemapError::PaletteBankOutOfRange { bank: 2, limit: 2 })
    ));
    assert!(dst.data.chunks(4).all(|c| c == sentinel));

    // Bit depth mismatch between tileset and palette bank.
    let ok_cells = vec![TileCell::tile(1)];
    let l = TileLayer::attach(&ok_cells, 1, 1, 4, 4).unwrap();
    let wrong = [Palette::blank(BitDepth::Bpp8)];
    assert_eq!(
        render_layers_to_container(&[l], &ts, &wrong, Camera::default(), None, &mut fresh()),
        Err(TilemapError::DepthMismatch)
    );

    // Tile size mismatch.
    let l8 = TileLayer::attach(&ok_cells, 1, 1, 8, 8).unwrap();
    assert_eq!(
        render_layers_to_container(&[l8], &ts, &bk, Camera::default(), None, &mut fresh()),
        Err(TilemapError::TileSizeMismatch)
    );

    // Too many layers / banks.
    let many = vec![l; MAX_LAYERS + 1];
    assert_eq!(
        render_layers_to_container(&many, &ts, &bk, Camera::default(), None, &mut fresh()),
        Err(TilemapError::TooManyLayers {
            count: MAX_LAYERS + 1
        })
    );
    let many_banks = vec![bank(0); MAX_PALETTE_BANKS + 1];
    assert_eq!(
        render_layers_to_container(
            &[l],
            &ts,
            &many_banks,
            Camera::default(),
            None,
            &mut fresh()
        ),
        Err(TilemapError::TooManyBanks {
            count: MAX_PALETTE_BANKS + 1
        })
    );

    // Container whose data length disagrees with its dimensions.
    let mut broken = PixelContainer::new(4, 4);
    broken.data.pop();
    assert!(matches!(
        render_layers_to_container(&[l], &ts, &bk, Camera::default(), None, &mut broken),
        Err(TilemapError::BufferTooSmall {
            required: 64,
            actual: 63
        })
    ));
    // Degenerate container.
    let mut empty = PixelContainer::new(0, 0);
    assert!(matches!(
        render_layers_to_container(&[l], &ts, &bk, Camera::default(), None, &mut empty),
        Err(TilemapError::InvalidViewport { .. })
    ));
}

#[test]
fn render_scanline_error_paths() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let cells = vec![TileCell::tile(2)];
    let l = TileLayer::attach(&cells, 1, 1, 4, 4).unwrap();
    let v = Viewport {
        width: 4,
        height: 4,
    };
    let mut line = [0u8; 16];
    assert_eq!(
        render_layer_scanline(&l, &ts, &bk, v, Camera::default(), 4, &mut line),
        Err(TilemapError::ScanlineOutOfRange { y: 4, height: 4 })
    );
    let mut short = [0u8; 15];
    assert_eq!(
        render_layer_scanline(&l, &ts, &bk, v, Camera::default(), 0, &mut short),
        Err(TilemapError::BufferTooSmall {
            required: 16,
            actual: 15
        })
    );
    assert!(matches!(
        render_layer_scanline(
            &l,
            &ts,
            &bk,
            Viewport {
                width: MAX_VIEWPORT + 1,
                height: 1
            },
            Camera::default(),
            0,
            &mut line
        ),
        Err(TilemapError::InvalidViewport { .. })
    ));
    // A bad tile in the row leaves the line untouched.
    let bad = vec![TileCell::tile(2), TileCell::tile(99)];
    let l = TileLayer::attach(&bad, 2, 1, 4, 4).unwrap();
    let mut line = [3u8; 32];
    assert!(render_layer_scanline(
        &l,
        &ts,
        &bk,
        Viewport {
            width: 8,
            height: 4
        },
        Camera::default(),
        0,
        &mut line
    )
    .is_err());
    assert!(line.iter().all(|&b| b == 3));
}

#[test]
fn render_eight_bit_tileset_and_two_bit_palette_depths() {
    // Bpp8 tiles with a Bpp8 palette bank render through the same path.
    let data = make_tileset_data(BitDepth::Bpp8, 4, 4, 4);
    let ts = Tileset::attach_tiles(BitDepth::Bpp8, &data, 4, 4, 4).unwrap();
    let mut p = Palette::blank(BitDepth::Bpp8);
    for i in 0..=255u8 {
        p.set_entry(i, [i, i, i, 255]).unwrap();
    }
    let cells = vec![TileCell::tile(2)];
    let mut l = TileLayer::attach(&cells, 1, 1, 4, 4).unwrap();
    l.color0_transparent = false;
    let mut dst = PixelContainer::new(4, 4);
    render_layers_to_container(&[l], &ts, &[p], Camera::default(), None, &mut dst).unwrap();
    let idx = tile_pixel(2, 1, 2);
    assert_eq!(px(&dst, 1, 2), [idx, idx, idx, 255]);
}

#[test]
fn render_golden_frame_digest() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let cells = grid(6, 5, |c, r| {
        TileCell::tile(((c * 3 + r * 5) % 8) as u16)
            .with_flip(c % 2 == 0, r % 3 == 0)
            .with_bank(((c + r) % 2) as u8)
    });
    let mut l = TileLayer::attach(&cells, 6, 5, 4, 4).unwrap();
    l.parallax_x_q8 = 192;
    let mut dst = PixelContainer::new(37, 23);
    render_layers_to_container(
        &[l],
        &ts,
        &bk,
        Camera { x: 13, y: 7 },
        Some([0, 0, 0, 255]),
        &mut dst,
    )
    .unwrap();
    let digest = sha256_hex(&dst.data);
    assert_eq!(
        digest, "86fb404ca354ccef80e9b01f7bbe8ee10571f0ee4a857bcfb921c8a8f374b013",
        "golden digest changed: update only for an intentional behaviour change"
    );
}

#[test]
fn determinism_same_inputs_same_output() {
    let (data, bk) = render_setup();
    let ts = Tileset::attach_tiles(BitDepth::Bpp4, &data, 4, 4, 8).unwrap();
    let cells = grid(5, 5, |c, r| {
        TileCell::tile(((c + r * 2) % 8) as u16).with_bank((c % 2) as u8)
    });
    let l = TileLayer::attach(&cells, 5, 5, 4, 4).unwrap();
    let mut a = PixelContainer::new(21, 17);
    let mut b = PixelContainer::new(21, 17);
    render_layers_to_container(&[l], &ts, &bk, Camera { x: -9, y: 31 }, None, &mut a).unwrap();
    render_layers_to_container(&[l], &ts, &bk, Camera { x: -9, y: 31 }, None, &mut b).unwrap();
    assert_eq!(a.data, b.data);
}

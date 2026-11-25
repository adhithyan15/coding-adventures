//! Hostile-input and deterministic fuzz test suite for `tilemap2d`.

#![forbid(unsafe_code)]

use palette_indexed::{BitDepth, Palette};
use pixel_container::PixelContainer;
use tilemap2d::{
    aabb_attrs, aabb_blocked, aabb_first_hit, attr_at_point, attr_at_tile, cell_at_tile,
    count_visible_cells, cull_layer, layer_scroll, lower_layer_cells_into,
    lower_layers_to_paint_scene, render_layer_scanline, render_layers_to_container,
    screen_to_layer, Aabb, Camera, MotionHint, TileBatchCell, TileCell, TileLayer, TilemapError,
    Tileset, Viewport, VisibleRect, WrapMode, ATTR_MASK, ATTR_SOLID, FLAG_MASK, MAX_LAYERS,
    MAX_MAP_DIM, MAX_TILE_SIZE, MAX_VIEWPORT,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    fn next_i64(&mut self) -> i64 {
        self.next_u64() as i64
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
}

#[test]
fn hostile_dims_extreme_coords_and_invalid_cells() {
    let cells = [TileCell::tile(1); 16];

    // Layer dimension and tile-size extremes.
    for &(cols, rows) in &[
        (0, 4),
        (4, 0),
        (MAX_MAP_DIM + 1, 1),
        (1, MAX_MAP_DIM + 1),
        (u32::MAX, u32::MAX),
    ] {
        assert!(matches!(
            TileLayer::attach(&cells, cols, rows, 8, 8),
            Err(TilemapError::InvalidMapSize { .. })
        ));
    }
    for &(tw, th) in &[
        (0, 8),
        (8, 0),
        (MAX_TILE_SIZE + 1, 8),
        (8, MAX_TILE_SIZE + 1),
        (u32::MAX, u32::MAX),
    ] {
        assert!(matches!(
            TileLayer::attach(&cells, 4, 4, tw, th),
            Err(TilemapError::InvalidTileSize { .. })
        ));
    }

    // Invalid flags / attrs rejected on attach.
    let bad_flag = [TileCell {
        tile_id: 1,
        flags: !FLAG_MASK,
        palette_bank: 0,
        attr: 0,
    }];
    assert!(matches!(
        TileLayer::attach(&bad_flag, 1, 1, 8, 8),
        Err(TilemapError::InvalidFlags { .. })
    ));
    let bad_attr = [TileCell {
        tile_id: 1,
        flags: 0,
        palette_bank: 0,
        attr: !ATTR_MASK,
    }];
    assert!(matches!(
        TileLayer::attach(&bad_attr, 1, 1, 8, 8),
        Err(TilemapError::InvalidAttr { .. })
    ));

    // Viewport extremes.
    let layer = TileLayer::attach(&cells, 4, 4, 8, 8).unwrap();
    for view in [
        Viewport {
            width: 0,
            height: 16,
        },
        Viewport {
            width: 16,
            height: 0,
        },
        Viewport {
            width: MAX_VIEWPORT + 1,
            height: 16,
        },
        Viewport {
            width: u32::MAX,
            height: u32::MAX,
        },
    ] {
        assert!(matches!(
            cull_layer(&layer, view, Camera::default()),
            Err(TilemapError::InvalidViewport { .. })
        ));
    }

    // Extreme camera coordinates (i32::MIN / i32::MAX) and i64::MIN / i64::MAX lookups.
    let ok_view = Viewport {
        width: 64,
        height: 64,
    };
    for &cam in &[
        Camera {
            x: i32::MIN,
            y: i32::MIN,
        },
        Camera {
            x: i32::MAX,
            y: i32::MAX,
        },
    ] {
        let _ = layer_scroll(&layer, ok_view, cam);
        let _ = cull_layer(&layer, ok_view, cam);
        let _ = screen_to_layer(&layer, ok_view, cam, i32::MIN, i32::MAX);
    }
    let _ = cell_at_tile(&layer, i64::MIN, i64::MAX);
    let _ = attr_at_tile(&layer, i64::MAX, i64::MIN);
    let _ = attr_at_point(&layer, i64::MIN, i64::MAX);

    // Oversized AABB and zero-size AABB.
    assert_eq!(
        aabb_attrs(
            &layer,
            Aabb {
                x: 0,
                y: 0,
                w: 0,
                h: 8
            }
        ),
        Err(TilemapError::InvalidAabb)
    );
    assert!(matches!(
        aabb_attrs(
            &layer,
            Aabb {
                x: 0,
                y: 0,
                w: u32::MAX,
                h: u32::MAX
            }
        ),
        Err(TilemapError::QueryTooLarge { .. })
    ));

    // Hand-built hostile VisibleRect rejected before looping (CWE-400).
    let hostile_rect = VisibleRect {
        first_col: i64::MAX,
        first_row: i64::MAX,
        cols: u32::MAX,
        rows: u32::MAX,
        offset_x: 0,
        offset_y: 0,
    };
    assert!(matches!(
        count_visible_cells(&layer, &hostile_rect),
        Err(TilemapError::InvalidViewport { .. })
    ));

    // Too many layers rejected.
    let many_layers = [layer; MAX_LAYERS + 1];
    assert!(matches!(
        lower_layers_to_paint_scene(
            &many_layers,
            ok_view,
            Camera::default(),
            &[[255, 255, 255]],
            "#000000"
        ),
        Err(TilemapError::TooManyLayers { .. })
    ));

    // Tileset bounds & scanline atomic failure preservation.
    let tile_data = [0x11u8; 64];
    assert!(Tileset::attach_tiles(BitDepth::Bpp4, &tile_data, 8, 8, 0).is_err());
    assert!(Tileset::attach_tiles(BitDepth::Bpp4, &tile_data[..1], 8, 8, 2).is_err());

    let tileset = Tileset::attach_tiles(BitDepth::Bpp4, &tile_data, 8, 8, 2).unwrap();
    let bad_cell = [TileCell::tile(99); 4];
    let bad_layer = TileLayer::attach(&bad_cell, 2, 2, 8, 8).unwrap();
    let mut dst = PixelContainer {
        width: 16,
        height: 16,
        data: vec![0x5Au8; 16 * 16 * 4],
    };
    let pal = Palette::blank(BitDepth::Bpp4);
    assert!(matches!(
        render_layers_to_container(
            &[bad_layer],
            &tileset,
            &[pal],
            Camera::default(),
            Some([0, 0, 0, 255]),
            &mut dst
        ),
        Err(TilemapError::TileIdOutOfRange { tile_id: 99, .. })
    ));
    assert!(dst.data.iter().all(|&b| b == 0x5A));
}

#[test]
fn hostile_fuzz_1000_cases() {
    let mut rng = SplitMix64(0x2026_0021_ABCD_1234);
    let wraps = [WrapMode::Wrap, WrapMode::Clamp, WrapMode::Void];
    let mut cells = [TileCell::tile(0); 64];
    for c in &mut cells {
        c.tile_id = (rng.next_u8() % 4) as u16;
        c.flags = rng.next_u8() & FLAG_MASK;
        c.attr = rng.next_u8() & ATTR_MASK;
    }

    let tile_data = [0x12u8; 4 * 8 * 4];
    let tileset = Tileset::attach_tiles(BitDepth::Bpp4, &tile_data, 8, 8, 4).unwrap();
    let pal = Palette::blank(BitDepth::Bpp4);
    let bank_colors = [[200u8, 100, 50]];

    for _ in 0..1000 {
        let mut layer = TileLayer::attach(&cells, 8, 8, 8, 8).unwrap();
        layer.scroll_x = rng.next_i32();
        layer.scroll_y = rng.next_i32();
        layer.parallax_x_q8 = rng.next_i32();
        layer.parallax_y_q8 = rng.next_i32();
        layer.wrap_x = wraps[(rng.next_u8() as usize) % wraps.len()];
        layer.wrap_y = wraps[(rng.next_u8() as usize) % wraps.len()];
        layer.edge_attr = if rng.next_u8() & 1 != 0 {
            ATTR_SOLID
        } else {
            0
        };
        layer.visible = !rng.next_u8().is_multiple_of(8);
        layer.alpha = rng.next_u8();

        let view = Viewport {
            width: match rng.next_u8() % 6 {
                0 => 0,
                1 => MAX_VIEWPORT + 1,
                _ => (rng.next_u32() % 64) + 1,
            },
            height: match rng.next_u8() % 6 {
                0 => 0,
                1 => MAX_VIEWPORT + 1,
                _ => (rng.next_u32() % 64) + 1,
            },
        };
        let cam = Camera {
            x: rng.next_i32(),
            y: rng.next_i32(),
        };

        let _ = cell_at_tile(&layer, rng.next_i64(), rng.next_i64());
        let _ = attr_at_point(&layer, rng.next_i64(), rng.next_i64());
        let _ = cull_layer(&layer, view, cam);
        let _ = screen_to_layer(&layer, view, cam, rng.next_i32(), rng.next_i32());

        let aabb = Aabb {
            x: rng.next_i64(),
            y: rng.next_i64(),
            w: rng.next_u32() % 128,
            h: rng.next_u32() % 128,
        };
        let _ = aabb_attrs(&layer, aabb);
        let _ = aabb_first_hit(&layer, aabb, rng.next_u8());
        let _ = aabb_blocked(
            &layer,
            aabb,
            MotionHint {
                moving_down: rng.next_u8() & 1 != 0,
                prev_bottom: rng.next_i64(),
            },
        );

        let _ = lower_layers_to_paint_scene(&[layer], view, cam, &bank_colors, "#102030");
        if let Ok(rect) = cull_layer(&layer, view, cam) {
            let mut batch = [TileBatchCell::new(0, 0, 0); 16];
            let _ = lower_layer_cells_into(&layer, &rect, &bank_colors, &mut batch);
        }

        let mut line = [0u8; 64 * 4];
        let _ = render_layer_scanline(
            &layer,
            &tileset,
            &[pal],
            view,
            cam,
            rng.next_u32() % 80,
            &mut line,
        );
        let mut pc = PixelContainer {
            width: 32,
            height: 32,
            data: vec![0u8; 32 * 32 * 4],
        };
        let _ = render_layers_to_container(&[layer], &tileset, &[pal], cam, None, &mut pc);
    }
}

//! Procedural tile sprite generator using the `image` crate.
//!
//! Generates isometric tile sprites (64x32 px each) in a horizontal sprite strip,
//! returning raw RGBA bytes suitable for `Platform::load_sprite_from_bytes`.

use image::{ImageBuffer, Rgba, RgbaImage};

/// Tile dimensions (isometric 2:1 projection)
const TILE_W: u32 = 64;
const TILE_H: u32 = 32;

/// All tile types to generate, in left-to-right sprite sheet order.
const TILE_TYPES: &[(&str, TileKind)] = &[
    ("grass_passable", TileKind::GrassPassable),
    ("grass_solid",    TileKind::GrassSolid),
    ("stone_solid",    TileKind::StoneSolid),
    ("lava_trap",      TileKind::LavaTrap),
    ("ice_passable",   TileKind::IcePassable),
    ("bridge_passable",TileKind::BridgePassable),
    ("snow_solid",     TileKind::SnowSolid),
    ("ice_trap",       TileKind::IceTrap),
    ("checkpoint",     TileKind::Checkpoint),
    ("goal",           TileKind::Goal),
];

#[derive(Clone, Copy)]
enum TileKind {
    GrassPassable,
    GrassSolid,
    StoneSolid,
    LavaTrap,
    IcePassable,
    BridgePassable,
    SnowSolid,
    IceTrap,
    Checkpoint,
    Goal,
}

/// Generate the complete tile sprite sheet as raw RGBA bytes.
///
/// Returns `(width, height, rgba_data)` where width = 64 * num_tiles, height = 32.
pub fn generate_tile_sprites() -> (u32, u32, Vec<u8>) {
    let num_tiles = TILE_TYPES.len() as u32;
    let sheet_w = TILE_W * num_tiles;
    let sheet_h = TILE_H;

    let mut img: RgbaImage = ImageBuffer::new(sheet_w, sheet_h);

    for (i, (_, kind)) in TILE_TYPES.iter().enumerate() {
        let offset_x = i as u32 * TILE_W;
        draw_tile(&mut img, offset_x, kind);
    }

    // Flatten to raw RGBA bytes
    let raw: Vec<u8> = img.into_raw();
    (sheet_w, sheet_h, raw)
}

/// Draw a single tile at (offset_x, 0) in the sprite sheet.
fn draw_tile(img: &mut RgbaImage, offset_x: u32, kind: &TileKind) {
    match kind {
        TileKind::GrassPassable => draw_grass_passable(img, offset_x),
        TileKind::GrassSolid    => draw_grass_solid(img, offset_x),
        TileKind::StoneSolid    => draw_stone_solid(img, offset_x),
        TileKind::LavaTrap      => draw_lava_trap(img, offset_x),
        TileKind::IcePassable   => draw_ice_passable(img, offset_x),
        TileKind::BridgePassable=> draw_bridge_passable(img, offset_x),
        TileKind::SnowSolid     => draw_snow_solid(img, offset_x),
        TileKind::IceTrap       => draw_ice_trap(img, offset_x),
        TileKind::Checkpoint    => draw_checkpoint(img, offset_x),
        TileKind::Goal          => draw_goal(img, offset_x),
    }
}

// -----------------------------------------------------------------------
// Tile drawing functions
// -----------------------------------------------------------------------

fn draw_grass_passable(img: &mut RgbaImage, offset_x: u32) {
    // Green diamond with light shade pattern
    let bg = rgba(30, 80, 30, 255);
    let fg = rgba(50, 120, 50, 255);
    let highlight = rgba(60, 140, 60, 255);

    fill_diamond(img, offset_x, bg);

    // Draw light shade pattern pixels
    let pattern: &[(u32, u32)] = &[
        (8, 4), (16, 6), (24, 4), (40, 4), (48, 6), (56, 4),
        (12, 10), (20, 12), (28, 10), (36, 12), (44, 10), (52, 12),
        (8, 18), (16, 16), (24, 18), (32, 14), (40, 18), (48, 16), (56, 18),
    ];
    for &(px, py) in pattern {
        let x = offset_x + px;
        if x < offset_x + TILE_W && py < TILE_H {
            img.put_pixel(x, py, fg);
        }
    }

    // Edge highlight
    for y in 0u32..16 {
        let half_w = y + 1;
        let left_x = offset_x + (TILE_W / 2) - half_w;
        let right_x = offset_x + (TILE_W / 2) + half_w - 1;
        if left_x < offset_x + TILE_W {
            img.put_pixel(left_x, y, highlight);
        }
        if right_x < offset_x + TILE_W && right_x >= offset_x {
            img.put_pixel(right_x, y, highlight);
        }
    }
}

fn draw_grass_solid(img: &mut RgbaImage, offset_x: u32) {
    // Solid dark green
    let base = rgba(20, 60, 20, 255);
    let mid = rgba(30, 80, 30, 255);
    let top = rgba(40, 100, 40, 255);

    fill_diamond(img, offset_x, base);

    // Dense pattern in top half (every 2 pixels)
    for y in 0u32..16 {
        let half_w = y + 1;
        let start_x = offset_x + (TILE_W / 2) - half_w;
        let end_x = start_x + half_w * 2;
        let mut x = start_x;
        while x < end_x {
            if x < offset_x + TILE_W {
                img.put_pixel(x, y, mid);
            }
            x += 2;
        }
    }

    // Top edge highlight
    for y in 0u32..16 {
        let half_w = y + 1;
        let left_x = offset_x + (TILE_W / 2) - half_w;
        let right_x = offset_x + (TILE_W / 2) + half_w - 1;
        if left_x < offset_x + TILE_W {
            img.put_pixel(left_x, y, top);
        }
        if right_x < offset_x + TILE_W && right_x >= offset_x {
            img.put_pixel(right_x, y, top);
        }
    }
}

fn draw_stone_solid(img: &mut RgbaImage, offset_x: u32) {
    // Gray stone
    let base = rgba(80, 80, 85, 255);
    let mid = rgba(100, 100, 105, 255);
    let highlight = rgba(130, 130, 135, 255);

    fill_diamond(img, offset_x, base);

    // Stone block pattern (every 3 pixels)
    for y in 0u32..TILE_H {
        let half_w = if y < 16 { y + 1 } else { 32 - y };
        let start_x = offset_x + (TILE_W / 2) - half_w;
        let end_x = start_x + half_w * 2;
        let mut x = start_x;
        while x < end_x {
            if x < offset_x + TILE_W {
                img.put_pixel(x, y, mid);
            }
            x += 3;
        }
    }

    // Top edge highlight
    for y in 0u32..16 {
        let half_w = y + 1;
        let left_x = offset_x + (TILE_W / 2) - half_w;
        if left_x < offset_x + TILE_W {
            img.put_pixel(left_x, y, highlight);
        }
    }
}

fn draw_lava_trap(img: &mut RgbaImage, offset_x: u32) {
    // Red/orange lava
    let base = rgba(180, 50, 0, 255);
    let mid = rgba(220, 80, 0, 255);
    let bright = rgba(255, 120, 0, 255);
    let yellow = rgba(255, 200, 0, 255);

    fill_diamond(img, offset_x, base);

    // Lava flow pattern - static texture
    let pattern: &[(u32, u32)] = &[
        (20, 6), (36, 8), (52, 6), (28, 12), (44, 14), (16, 18), (40, 20), (56, 18),
        (24, 24), (48, 26), (12, 8), (32, 10), (60, 12), (20, 16), (44, 22), (52, 28),
    ];
    for &(px, py) in pattern {
        let x = offset_x + px;
        if x < offset_x + TILE_W && py < TILE_H {
            img.put_pixel(x, py, mid);
        }
    }

    // Bright spots
    let bright_spots: &[(u32, u32)] = &[
        (28, 8), (44, 10), (24, 16), (48, 18), (36, 24), (20, 12), (52, 20),
    ];
    for &(px, py) in bright_spots {
        let x = offset_x + px;
        if x < offset_x + TILE_W && py < TILE_H {
            img.put_pixel(x, py, bright);
        }
    }

    // Yellow hotspots
    let yellow_spots: &[(u32, u32)] = &[(32, 10), (40, 16), (28, 20), (48, 24)];
    for &(px, py) in yellow_spots {
        let x = offset_x + px;
        if x < offset_x + TILE_W && py < TILE_H {
            img.put_pixel(x, py, yellow);
        }
    }
}

fn draw_ice_passable(img: &mut RgbaImage, offset_x: u32) {
    // Light blue diamond with shade pattern
    let bg = rgba(150, 200, 220, 255);
    let fg = rgba(180, 220, 240, 255);
    let highlight = rgba(220, 240, 250, 255);

    fill_diamond(img, offset_x, bg);

    // Ice crystal pattern
    let pattern: &[(u32, u32)] = &[
        (16, 4), (32, 6), (48, 4), (24, 10), (40, 12), (56, 10),
        (12, 16), (28, 14), (44, 16), (20, 20), (36, 22), (52, 20),
    ];
    for &(px, py) in pattern {
        let x = offset_x + px;
        if x < offset_x + TILE_W && py < TILE_H {
            img.put_pixel(x, py, fg);
        }
    }

    // Edge highlight
    for y in 0u32..16 {
        let half_w = y + 1;
        let left_x = offset_x + (TILE_W / 2) - half_w;
        let right_x = offset_x + (TILE_W / 2) + half_w - 1;
        if left_x < offset_x + TILE_W {
            img.put_pixel(left_x, y, highlight);
        }
        if right_x < offset_x + TILE_W && right_x >= offset_x {
            img.put_pixel(right_x, y, highlight);
        }
    }
}

fn draw_bridge_passable(img: &mut RgbaImage, offset_x: u32) {
    // Brown wooden bridge - horizontal planks
    let bg = rgba(100, 70, 40, 255);
    let plank = rgba(120, 85, 50, 255);
    let highlight = rgba(140, 100, 60, 255);

    fill_diamond(img, offset_x, bg);

    // Draw horizontal plank lines
    for y in 4u32..TILE_H {
        let half_w = if y < 16 { y + 1 } else { 32 - y };
        let start_x = offset_x + (TILE_W / 2) - half_w;
        let end_x = start_x + half_w * 2;
        let in_plank_row = y % 4 == 0 || y % 4 == 1;
        let mut x = start_x;
        while x < end_x {
            if x < offset_x + TILE_W {
                img.put_pixel(x, y, if in_plank_row { plank } else { bg });
            }
            x += 1;
        }
    }

    // Edge highlight on top
    for y in 0u32..16 {
        let half_w = y + 1;
        let left_x = offset_x + (TILE_W / 2) - half_w;
        let right_x = offset_x + (TILE_W / 2) + half_w - 1;
        if left_x < offset_x + TILE_W {
            img.put_pixel(left_x, y, highlight);
        }
        if right_x < offset_x + TILE_W && right_x >= offset_x {
            img.put_pixel(right_x, y, highlight);
        }
    }
}

fn draw_snow_solid(img: &mut RgbaImage, offset_x: u32) {
    // White/light gray snow
    let base = rgba(200, 210, 220, 255);
    let mid = rgba(220, 230, 240, 255);
    let highlight = rgba(255, 255, 255, 255);

    fill_diamond(img, offset_x, base);

    // Snow sparkle pattern (every 4 pixels)
    for y in 0u32..TILE_H {
        let half_w = if y < 16 { y + 1 } else { 32 - y };
        let start_x = offset_x + (TILE_W / 2) - half_w;
        let end_x = start_x + half_w * 2;
        let mut x = start_x;
        while x < end_x {
            if x < offset_x + TILE_W {
                img.put_pixel(x, y, mid);
            }
            x += 4;
        }
    }

    // Top edge bright highlight
    for y in 0u32..16 {
        let half_w = y + 1;
        let left_x = offset_x + (TILE_W / 2) - half_w;
        if left_x < offset_x + TILE_W {
            img.put_pixel(left_x, y, highlight);
        }
    }
}

fn draw_ice_trap(img: &mut RgbaImage, offset_x: u32) {
    // Cyan ice trap
    let bg = rgba(0, 150, 180, 255);
    let fg = rgba(0, 200, 220, 255);
    let bright = rgba(100, 240, 255, 255);

    fill_diamond(img, offset_x, bg);

    // Ice crystal shards pattern
    let pattern: &[(u32, u32)] = &[
        (20, 4), (36, 8), (52, 4), (16, 12), (32, 14), (48, 12),
        (24, 18), (40, 20), (56, 18), (12, 22), (28, 24), (44, 22),
    ];
    for &(px, py) in pattern {
        let x = offset_x + px;
        if x < offset_x + TILE_W && py < TILE_H {
            img.put_pixel(x, py, fg);
        }
    }

    // Bright crystal tips
    let bright_spots: &[(u32, u32)] = &[(28, 8), (44, 12), (24, 16), (40, 20), (52, 24)];
    for &(px, py) in bright_spots {
        let x = offset_x + px;
        if x < offset_x + TILE_W && py < TILE_H {
            img.put_pixel(x, py, bright);
        }
    }

    // Edge highlight
    for y in 0u32..16 {
        let half_w = y + 1;
        let left_x = offset_x + (TILE_W / 2) - half_w;
        let right_x = offset_x + (TILE_W / 2) + half_w - 1;
        if left_x < offset_x + TILE_W {
            img.put_pixel(left_x, y, bright);
        }
        if right_x < offset_x + TILE_W && right_x >= offset_x {
            img.put_pixel(right_x, y, bright);
        }
    }
}

fn draw_checkpoint(img: &mut RgbaImage, offset_x: u32) {
    // Golden diamond checkpoint marker on green base
    let bg = rgba(30, 80, 30, 255);
    let gold = rgba(255, 215, 0, 255);
    let bright = rgba(255, 240, 100, 255);
    let dark_gold = rgba(180, 150, 0, 255);

    // First draw green passable base
    fill_diamond(img, offset_x, bg);

    // Draw golden diamond (◆) in center - a rotated square shape
    let cx = offset_x + 32;
    let cy = 16u32;

    // Fill the diamond with gold
    for y in (cy - 10)..(cy + 10) {
        let dist = if y < cy { cy - y } else { y - cy };
        for x in (cx - dist)..(cx + dist) {
            if x >= offset_x && x < offset_x + TILE_W && y < TILE_H {
                img.put_pixel(x, y, gold);
            }
        }
    }

    // Bright center highlight
    for y in (cy - 5)..(cy + 5) {
        let dist = if y < cy { cy - y } else { y - cy };
        for x in (cx - dist)..(cx + dist) {
            if x >= offset_x && x < offset_x + TILE_W && y < TILE_H {
                img.put_pixel(x, y, bright);
            }
        }
    }

    // Draw diamond outline using bresenham
    bresenham_line(img, cx, cy - 10, cx + 10, cy, dark_gold);
    bresenham_line(img, cx + 10, cy, cx, cy + 10, dark_gold);
    bresenham_line(img, cx, cy + 10, cx - 10, cy, dark_gold);
    bresenham_line(img, cx - 10, cy, cx, cy - 10, dark_gold);
}

fn draw_goal(img: &mut RgbaImage, offset_x: u32) {
    // Bright star goal marker (★) on green base
    let bg = rgba(30, 80, 30, 255);
    let star_color = rgba(255, 255, 100, 255);
    let bright_star = rgba(255, 255, 200, 255);
    let dark_star = rgba(200, 180, 0, 255);

    // First draw green passable base
    fill_diamond(img, offset_x, bg);

    // Draw star (★) in center
    let cx = offset_x + 32;
    let cy = 16u32;

    // Simple 5-pointed star approximation using distance from center
    for y in (cy - 12)..(cy + 12) {
        for x in (cx - 12)..(cx + 12) {
            if x >= offset_x && x < offset_x + TILE_W && y < TILE_H {
                if is_point_in_star(x, y, cx, cy, 12, 5) {
                    img.put_pixel(x, y, star_color);
                }
            }
        }
    }

    // Bright center
    for y in (cy - 4)..(cy + 4) {
        for x in (cx - 4)..(cx + 4) {
            if x >= offset_x && x < offset_x + TILE_W && y < TILE_H {
                if is_point_in_star(x, y, cx, cy, 12, 5) {
                    img.put_pixel(x, y, bright_star);
                }
            }
        }
    }

    // Star outline using simple cross and X
    for r in 8..=11 {
        // Approximate star outline with distance-based check
        for angle in 0..360 {
            let rad = (angle as f64) * std::f64::consts::PI / 180.0;
            let px = cx as f64 + (r as f64 * (rad.cos()));
            let py = cy as f64 + (r as f64 * (rad.sin()));
            if px >= offset_x as f64 && px < (offset_x + TILE_W) as f64 && py >= 0.0 && py < TILE_H as f64 {
                if is_point_in_star(px as u32, py as u32, cx, cy, 12, 5) {
                    img.put_pixel(px as u32, py as u32, dark_star);
                }
            }
        }
    }
}

/// Check if a point is inside a star shape.
fn is_point_in_star(px: u32, py: u32, cx: u32, cy: u32, outer_r: u32, _inner_r: u32) -> bool {
    // Simple diamond/balloon shape approximation
    let dx = (px as i32 - cx as i32).abs() as u32;
    let dy = (py as i32 - cy as i32).abs() as u32;
    let max_dist = outer_r;
    dx + dy <= max_dist
}

/// Fill the isometric diamond shape in the 64x32 tile at offset_x.
fn fill_diamond(img: &mut RgbaImage, offset_x: u32, color: Rgba<u8>) {
    // Top half: rows 0..=15 (16 rows), diamond narrows from full width to center
    for y in 0u32..16 {
        let half_w = y + 1;
        let start_x = offset_x + (TILE_W / 2) - half_w;
        for x in start_x..(start_x + half_w * 2) {
            img.put_pixel(x, y, color);
        }
    }
    // Bottom half: rows 16..=31, diamond widens back out
    for y in 16u32..32 {
        let half_w = 32 - y;
        let start_x = offset_x + (TILE_W / 2) - half_w;
        for x in start_x..(start_x + half_w * 2) {
            img.put_pixel(x, y, color);
        }
    }
}

/// Bresenham's line algorithm for drawing outlines.
fn bresenham_line(img: &mut RgbaImage, x0: u32, y0: u32, x1: u32, y1: u32, color: Rgba<u8>) {
    let mut x0 = x0 as i32;
    let mut y0 = y0 as i32;
    let x1 = x1 as i32;
    let y1 = y1 as i32;

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    loop {
        let w = img.width() as i32;
        let h = img.height() as i32;
        if x0 >= 0 && x0 < w && y0 >= 0 && y0 < h {
            img.put_pixel(x0 as u32, y0 as u32, color);
        }

        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x0 += sx;
        }
        if e2 < dx {
            err += dx;
            y0 += sy;
        }
    }
}

// -----------------------------------------------------------------------
// RGBA color helper
// -----------------------------------------------------------------------
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba<u8> {
    Rgba([r, g, b, a])
}
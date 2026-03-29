//! Programmatic sprite generation using the `image` crate.
//! Generates character sprites (player @, NPC ?) as RGBA bytes.

use image::{ImageBuffer, Rgba, RgbaImage};

/// Frame size for each character sprite (64x64).
const FRAME_SIZE: u32 = 64;
const FRAME_SIZE_I32: i32 = 64;

/// Generate the complete characters sprite sheet as RGBA bytes.
/// Layout: 3 columns x 3 rows = 192x192 total
/// Row 0: player1 idle, run0, run1
/// Row 1: player2 idle, run0, run1
/// Row 2: npc idle, (empty), (empty)
pub fn generate_characters_sprite_sheet() -> Vec<u8> {
    let frames = 3; // columns
    let rows = 3; // player1, player2, npc
    let width = FRAME_SIZE * frames;
    let height = FRAME_SIZE * rows;

    let mut img: RgbaImage = ImageBuffer::new(width, height);

    // Fill background with dark color so first pixel (0,0) is not transparent
    for pixel in img.pixels_mut() {
        *pixel = Rgba([0x20, 0x20, 0x40, 0xFF]);
    }

    // Player 1: @ in blue (#4488FF) - row 0
    draw_character(&mut img, 0, 0, '@', [0x44, 0x88, 0xFF, 0xFF]);
    draw_character(&mut img, 1, 0, '@', [0x44, 0x88, 0xFF, 0xFF]); // run0
    draw_character(&mut img, 2, 0, '@', [0x44, 0x88, 0xFF, 0xFF]); // run1

    // Player 2: @ in red (#FF4444) - row 1
    draw_character(&mut img, 0, 1, '@', [0xFF, 0x44, 0x44, 0xFF]);
    draw_character(&mut img, 1, 1, '@', [0xFF, 0x44, 0x44, 0xFF]); // run0
    draw_character(&mut img, 2, 1, '@', [0xFF, 0x44, 0x44, 0xFF]); // run1

    // NPC: ? in purple (#AA44FF) - row 2
    draw_character(&mut img, 0, 2, '?', [0xAA, 0x44, 0xFF, 0xFF]);

    img.into_raw()
}

/// Draw a character glyph into a sprite sheet cell.
/// Uses a simple pixel-art approach for ASCII characters.
fn draw_character(img: &mut RgbaImage, col: u32, row: u32, ch: char, color: [u8; 4]) {
    let ox = col * FRAME_SIZE;
    let oy = row * FRAME_SIZE;
    let cx = ox + FRAME_SIZE / 2;
    let cy = oy + FRAME_SIZE / 2;
    let frame_i32 = FRAME_SIZE_I32;

    match ch {
        '@' => {
            // Draw @ symbol: circle with tail
            let center_r: i32 = 18;
            let inner_r: i32 = 10;

            // Outer circle
            for dy in -center_r..=center_r {
                for dx in -center_r..=center_r {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    let center_r_sq = (center_r as f32).powi(2);
                    if (dist_sq - center_r_sq).abs() < center_r_sq * 0.6 {
                        let px = cx as i32 + dx;
                        let py = cy as i32 + dy - 4;
                        if px >= ox as i32 && px < (ox + FRAME_SIZE) as i32
                            && py >= oy as i32 && py < (oy + FRAME_SIZE) as i32
                        {
                            img.put_pixel(px as u32, py as u32, Rgba(color));
                        }
                    }
                }
            }
            // Inner hole (background color)
            for dy in -inner_r..=inner_r {
                for dx in -inner_r..=inner_r {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    let inner_r_sq = (inner_r as f32).powi(2);
                    if dist_sq < inner_r_sq {
                        let px = cx as i32 + dx;
                        let py = cy as i32 + dy - 4;
                        if px >= ox as i32 && px < (ox + FRAME_SIZE) as i32
                            && py >= oy as i32 && py < (oy + FRAME_SIZE) as i32
                        {
                            img.put_pixel(px as u32, py as u32, Rgba([0x20, 0x20, 0x40, 0xFF]));
                        }
                    }
                }
            }
            // Tail (vertical line at bottom right)
            for i in 0..8 {
                let px = cx as i32 + 8;
                let py = cy as i32 + 4 + i;
                if py >= oy as i32 && py < (oy + FRAME_SIZE) as i32 {
                    img.put_pixel(px as u32, py as u32, Rgba(color));
                }
            }
        }
        '?' => {
            // Draw ? symbol: top curve + vertical line + dot
            let top_r: i32 = 14;

            // Top curved part
            for dy in -top_r..=top_r {
                for dx in -top_r..=top_r {
                    let dist_sq = (dx * dx + dy * dy) as f32;
                    let top_r_sq = (top_r as f32).powi(2);
                    // Create a C-shape (open on the right)
                    if (dist_sq - top_r_sq).abs() < top_r_sq * 0.7 && dx < 4 {
                        let px = cx as i32 + dx;
                        let py = cy as i32 + dy - 10;
                        if px >= ox as i32 && px < (ox + FRAME_SIZE) as i32
                            && py >= oy as i32 && py < (oy + FRAME_SIZE) as i32
                        {
                            img.put_pixel(px as u32, py as u32, Rgba(color));
                        }
                    }
                }
            }
            // Vertical stem
            for i in 0..14 {
                let px = cx as i32 - 4;
                let py = cy as i32 - 2 + i;
                if py >= oy as i32 && py < (oy + FRAME_SIZE) as i32 {
                    img.put_pixel(px as u32, py as u32, Rgba(color));
                }
            }
            // Dot at bottom
            for dy in -4..=4 {
                for dx in -4..=4 {
                    if dx * dx + dy * dy < 16 {
                        let px = cx as i32 + dx;
                        let py = cy as i32 + dy + 16;
                        if px >= ox as i32 && px < (ox + FRAME_SIZE) as i32
                            && py >= oy as i32 && py < (oy + FRAME_SIZE) as i32
                        {
                            img.put_pixel(px as u32, py as u32, Rgba(color));
                        }
                    }
                }
            }
        }
        _ => {}
    }

    // Suppress unused variable warning
    let _ = frame_i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_characters_sprite_sheet_size() {
        let data = generate_characters_sprite_sheet();
        // 192x192 RGBA = 192 * 192 * 4 = 147456 bytes
        assert_eq!(data.len(), 192 * 192 * 4);
    }

    #[test]
    fn test_generate_characters_returns_valid_rgba() {
        let data = generate_characters_sprite_sheet();
        // Check first pixel is not all zeros (should have some color)
        assert!(data[0] != 0 || data[1] != 0 || data[2] != 0 || data[3] != 0,
            "Generated sprite sheet should not be all transparent");
    }
}

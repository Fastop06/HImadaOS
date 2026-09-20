use crate::FbInfo;
use core::cmp::Ord;

#[derive(Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Copy, Clone)]
pub struct Border {
    pub width: usize,
    pub color_tl: Color,
    pub color_br: Color,
}

#[inline(always)]
pub unsafe fn clean_dcache_range(start: usize, len: usize) {
    let line_size = 64; // Apple Silicon cache line size
    let end = start + len;
    let mut addr = start & !(line_size - 1);
    while addr < end {
        core::arch::asm!(
            "dc cvac, {0}",
            in(reg) addr,
            options(nostack, preserves_flags)
        );
        addr += line_size;
    }
    core::arch::asm!("dsb sy");
}

pub fn draw_background(fb: &FbInfo, buffer: *mut u8, color_top: Color, color_bottom: Color) {
    let width = fb.width as usize;
    let height = fb.height as usize;
    let pitch = fb.pitch as usize;
    let bpp = (fb.bpp / 8) as usize;

    for y in 0..height {
        let blend = (y * 255) / height;
        let inv = 255 - blend;
        let r = ((color_top.r as usize * inv + color_bottom.r as usize * blend) / 255) as u8;
        let g = ((color_top.g as usize * inv + color_bottom.g as usize * blend) / 255) as u8;
        let b = ((color_top.b as usize * inv + color_bottom.b as usize * blend) / 255) as u8;
        
        for x in 0..width {
            let offset = y * pitch + x * bpp;
            unsafe {
                core::ptr::write_volatile(buffer.add(offset), b);
                core::ptr::write_volatile(buffer.add(offset + 1), g);
                core::ptr::write_volatile(buffer.add(offset + 2), r);
                if bpp == 4 {
                    core::ptr::write_volatile(buffer.add(offset + 3), 255);
                }
            }
        }
    }
}

fn isqrt(n: usize) -> usize {
    let mut x = n;
    let mut y = 1;
    while x > y {
        x = (x + y) / 2;
        y = n / x;
    }
    x
}

pub fn draw_hyprland_window(
    fb: &FbInfo, buffer: *mut u8,
    x: usize, y: usize, w: usize, h: usize,
    radius: usize,
    bg_color: Color,
    border: Option<Border>
) {
    let pitch = fb.pitch as usize;
    let bpp = (fb.bpp / 8) as usize;

    // Boundary checks to avoid overflow
    let start_y = y;
    let end_y = (y + h).min(fb.height as usize);
    let start_x = x;
    let end_x = (x + w).min(fb.width as usize);

    for py in start_y..end_y {
        for px in start_x..end_x {
            let dx = if px < x + radius { x + radius - px }
                     else if px > x + w - radius { px - (x + w - radius) }
                     else { 0 };
                     
            let dy = if py < y + radius { y + radius - py }
                     else if py > y + h - radius { py - (y + h - radius) }
                     else { 0 };

            let dist_to_corner_center = isqrt(dx * dx + dy * dy);

            // Clip outside rounded corners
            if dist_to_corner_center > radius { continue; }
            
            // Calculate distance to the nearest edge
            let dist_to_edge = if dx > 0 || dy > 0 {
                radius - dist_to_corner_center
            } else {
                let d_left = px - x;
                let d_right = (x + w - 1) - px;
                let d_top = py - y;
                let d_bottom = (y + h - 1) - py;
                d_left.min(d_right).min(d_top).min(d_bottom)
            };

            let is_border = match &border {
                Some(b) => dist_to_edge < b.width,
                None => false
            };

            let offset = py * pitch + px * bpp;
            
            let (final_r, final_g, final_b, alpha) = if is_border {
                let b = border.as_ref().unwrap();
                let blend_x = (px - x) * 255 / w;
                let blend_y = (py - y) * 255 / h;
                let blend = (blend_x + blend_y) / 2;
                let inv = 255 - blend;
                
                let r = ((b.color_tl.r as usize * inv + b.color_br.r as usize * blend) / 255) as u8;
                let g = ((b.color_tl.g as usize * inv + b.color_br.g as usize * blend) / 255) as u8;
                let blue = ((b.color_tl.b as usize * inv + b.color_br.b as usize * blend) / 255) as u8;
                (r, g, blue, 255) // Solid border
            } else {
                // Apply a slight fade at the very edge of the background for anti-aliasing feel
                let mut a = bg_color.a as usize;
                if dist_to_corner_center == radius { a /= 2; }
                (bg_color.r, bg_color.g, bg_color.b, a)
            };

            unsafe {
                if alpha == 255 {
                    core::ptr::write_volatile(buffer.add(offset), final_b);
                    core::ptr::write_volatile(buffer.add(offset + 1), final_g);
                    core::ptr::write_volatile(buffer.add(offset + 2), final_r);
                } else {
                    let bg_b = core::ptr::read_volatile(buffer.add(offset)) as usize;
                    let bg_g = core::ptr::read_volatile(buffer.add(offset + 1)) as usize;
                    let bg_r = core::ptr::read_volatile(buffer.add(offset + 2)) as usize;
                    
                    let inv_alpha = 255 - alpha;
                    let out_b = ((final_b as usize * alpha) + (bg_b * inv_alpha)) / 255;
                    let out_g = ((final_g as usize * alpha) + (bg_g * inv_alpha)) / 255;
                    let out_r = ((final_r as usize * alpha) + (bg_r * inv_alpha)) / 255;
                    
                    core::ptr::write_volatile(buffer.add(offset), out_b.min(255) as u8);
                    core::ptr::write_volatile(buffer.add(offset + 1), out_g.min(255) as u8);
                    core::ptr::write_volatile(buffer.add(offset + 2), out_r.min(255) as u8);
                }
                if bpp == 4 {
                    core::ptr::write_volatile(buffer.add(offset + 3), 255);
                }
            }
        }
    }
}

pub fn draw_circle(fb: &FbInfo, buffer: *mut u8, cx: usize, cy: usize, r: usize, color: Color) {
    let pitch = fb.pitch as usize;
    let bpp = (fb.bpp / 8) as usize;
    for y in cy.saturating_sub(r)..=(cy + r) {
        for x in cx.saturating_sub(r)..=(cx + r) {
            let dx = if x > cx { x - cx } else { cx - x };
            let dy = if y > cy { y - cy } else { cy - y };
            // Simple anti-aliasing feel at the edge
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= r * r {
                let offset = y * pitch + x * bpp;
                unsafe {
                    core::ptr::write_volatile(buffer.add(offset), color.b);
                    core::ptr::write_volatile(buffer.add(offset + 1), color.g);
                    core::ptr::write_volatile(buffer.add(offset + 2), color.r);
                }
            }
        }
    }
}

pub fn draw_char(fb: &FbInfo, buffer: *mut u8, x: usize, y: usize, c: char, color: Color, scale: usize) {
    if c as usize >= 128 { return; }
    let glyph = crate::font::FONT8X8[c as usize];
    let pitch = fb.pitch as usize;
    let bpp = (fb.bpp / 8) as usize;

    for (row, byte) in glyph.iter().enumerate() {
        for col in 0..8 {
            if (*byte & (1 << col)) != 0 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = x + col * scale + dx;
                        let py = y + row * scale + dy;
                        if px < fb.width as usize && py < fb.height as usize {
                            let offset = py * pitch + px * bpp;
                            unsafe {
                                core::ptr::write_volatile(buffer.add(offset), color.b);
                                core::ptr::write_volatile(buffer.add(offset + 1), color.g);
                                core::ptr::write_volatile(buffer.add(offset + 2), color.r);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn draw_string(fb: &FbInfo, buffer: *mut u8, x: usize, y: usize, s: &str, color: Color, scale: usize) {
    let mut curr_x = x;
    for c in s.chars() {
        draw_char(fb, buffer, curr_x, y, c, color, scale);
        curr_x += 8 * scale;
    }
}

pub fn draw_hyprland_border_only(
    fb: &FbInfo, buffer: *mut u8,
    x: usize, y: usize, w: usize, h: usize,
    radius: usize,
    border: &Border,
    tick: usize
) {
    let pitch = fb.pitch as usize;
    let bpp = (fb.bpp / 8) as usize;

    let start_y = y;
    let end_y = (y + h).min(fb.height as usize);
    let start_x = x;
    let end_x = (x + w).min(fb.width as usize);

    for py in start_y..end_y {
        for px in start_x..end_x {
            let dx = if px < x + radius { x + radius - px }
                     else if px > x + w - radius { px - (x + w - radius) }
                     else { 0 };
                     
            let dy = if py < y + radius { y + radius - py }
                     else if py > y + h - radius { py - (y + h - radius) }
                     else { 0 };

            let dist_to_corner_center = isqrt(dx * dx + dy * dy);
            if dist_to_corner_center > radius { continue; }
            
            let dist_to_edge = if dx > 0 || dy > 0 {
                radius - dist_to_corner_center
            } else {
                let d_left = px - x;
                let d_right = (x + w - 1) - px;
                let d_top = py - y;
                let d_bottom = (y + h - 1) - py;
                d_left.min(d_right).min(d_top).min(d_bottom)
            };

            if dist_to_edge < border.width {
                let offset = py * pitch + px * bpp;
                // Animated gradient calculation
                let total_dist = (px - x) + (py - y) + tick;
                let blend_raw = (total_dist * 255 / (w + h)) % 512;
                let blend = if blend_raw > 255 { 512 - blend_raw } else { blend_raw };
                let inv = 255 - blend;
                
                let r = ((border.color_tl.r as usize * inv + border.color_br.r as usize * blend) / 255) as u8;
                let g = ((border.color_tl.g as usize * inv + border.color_br.g as usize * blend) / 255) as u8;
                let blue = ((border.color_tl.b as usize * inv + border.color_br.b as usize * blend) / 255) as u8;

                unsafe {
                    core::ptr::write_volatile(buffer.add(offset), blue);
                    core::ptr::write_volatile(buffer.add(offset + 1), g);
                    core::ptr::write_volatile(buffer.add(offset + 2), r);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// True Hardware-Accelerated Terminal Console
// ─────────────────────────────────────────────────────────────

pub struct ConsoleState {
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub fg_color: Color,
    pub bg_color: Color,
    pub scale: usize,
}

pub static CONSOLE: spin::Mutex<ConsoleState> = spin::Mutex::new(ConsoleState {
    cursor_x: 10,
    cursor_y: 10,
    fg_color: Color { r: 205, g: 214, b: 244, a: 255 }, // Crisp White / Platinum
    bg_color: Color { r: 0, g: 0, b: 0, a: 255 },         // Pure Solid Black Background
    scale: 1,                                            // 1x Native Sharp Console Font
});

pub fn draw_char_with_bg(fb: &FbInfo, buffer: *mut u8, x: usize, y: usize, c: char, fg: Color, bg: Color, scale: usize) {
    let glyph = if (c as usize) < 128 { crate::font::FONT8X8[c as usize] } else { crate::font::FONT8X8['?' as usize] };
    let pitch = fb.pitch as usize;
    let bpp = (fb.bpp / 8) as usize;

    for (row, byte) in glyph.iter().enumerate() {
        for col in 0..8 {
            let color = if (*byte & (1 << col)) != 0 { fg } else { bg };
            for dy in 0..scale {
                for dx in 0..scale {
                    let px = x + col * scale + dx;
                    let py = y + row * scale + dy;
                    if px < fb.width as usize && py < fb.height as usize {
                        let offset = py * pitch + px * bpp;
                        unsafe {
                            core::ptr::write_volatile(buffer.add(offset), color.b);
                            core::ptr::write_volatile(buffer.add(offset + 1), color.g);
                            core::ptr::write_volatile(buffer.add(offset + 2), color.r);
                        }
                    }
                }
            }
        }
    }
}

pub fn console_print_str(fb: &FbInfo, buffer: *mut u8, s: &str) {
    let mut con = CONSOLE.lock();
    let char_w = 8 * con.scale;
    let char_h = 8 * con.scale;
    let line_h = char_h + 8; // Comfortable vertical line spacing
    let max_x = fb.width as usize - 10;
    let max_y = fb.height as usize - line_h;
    let pitch = fb.pitch as usize;
    let bpp = (fb.bpp / 8) as usize;

    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                let mut seq_buf = [0u8; 16];
                let mut seq_len = 0;
                while let Some(&next_c) = chars.peek() {
                    if (next_c >= 'a' && next_c <= 'z') || (next_c >= 'A' && next_c <= 'Z') {
                        let terminator = chars.next().unwrap();
                        let seq_bytes = &seq_buf[..seq_len];
                        if terminator == 'm' {
                            match seq_bytes {
                                b"0" => con.fg_color = Color { r: 205, g: 214, b: 244, a: 255 },
                                b"1;32" | b"32" => con.fg_color = Color { r: 166, g: 227, b: 161, a: 255 }, // Green
                                b"1;34" | b"34" => con.fg_color = Color { r: 137, g: 180, b: 250, a: 255 }, // Blue
                                b"1;36" | b"36" => con.fg_color = Color { r: 148, g: 226, b: 213, a: 255 }, // Cyan
                                b"1;31" | b"31" => con.fg_color = Color { r: 243, g: 139, b: 168, a: 255 }, // Red
                                b"1;33" | b"33" => con.fg_color = Color { r: 249, g: 226, b: 175, a: 255 }, // Yellow
                                b"1;35" | b"35" => con.fg_color = Color { r: 203, g: 166, b: 247, a: 255 }, // Purple
                                b"1;37" | b"37" | b"1" => con.fg_color = Color { r: 255, g: 255, b: 255, a: 255 },
                                _ => {}
                            }
                        } else if terminator == 'J' && seq_bytes == b"2" {
                            con.cursor_x = 10;
                            con.cursor_y = 10;
                            unsafe {
                                core::ptr::write_bytes(buffer, 0, pitch * (fb.height as usize));
                            }
                        }
                        break;
                    } else {
                        let c = chars.next().unwrap();
                        if seq_len < seq_buf.len() {
                            seq_buf[seq_len] = c as u8;
                            seq_len += 1;
                        }
                    }
                }
                continue;
            }
        }

        if c == '\x08' {
            if con.cursor_x >= 10 + char_w {
                con.cursor_x -= char_w;
            } else {
                con.cursor_x = 10;
            }
        } else if c == '\n' {
            con.cursor_x = 10;
            con.cursor_y += line_h;
        } else if c == '\r' {
            con.cursor_x = 10;
        } else if c == '\t' {
            con.cursor_x += char_w * 4;
        } else {
            if con.cursor_x + char_w > max_x {
                con.cursor_x = 10;
                con.cursor_y += line_h;
            }
            if con.cursor_y > max_y {
                let scroll_bytes = line_h * pitch;
                let total_bytes = (fb.height as usize) * pitch;
                unsafe {
                    core::ptr::copy(buffer.add(scroll_bytes), buffer, total_bytes - scroll_bytes);
                    for y in (fb.height as usize - line_h)..fb.height as usize {
                        for x in 0..fb.width as usize {
                            let offset = y * pitch + x * bpp;
                            core::ptr::write_volatile(buffer.add(offset), con.bg_color.b);
                            core::ptr::write_volatile(buffer.add(offset + 1), con.bg_color.g);
                            core::ptr::write_volatile(buffer.add(offset + 2), con.bg_color.r);
                        }
                    }
                }
                con.cursor_y -= line_h;
            }

            draw_char_with_bg(fb, buffer, con.cursor_x, con.cursor_y, c, con.fg_color, con.bg_color, con.scale);
            con.cursor_x += char_w;
        }
    }

    unsafe {
        clean_dcache_range(buffer as usize, pitch * (fb.height as usize));
    }
}

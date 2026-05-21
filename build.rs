use image::{ColorType, ImageFormat};
use std::{env, fs, path::PathBuf};

#[cfg(target_os = "linux")]
use std::process::Command;

const BADGE_ICON_SIZE: usize = 16;
const BADGE_RED: [u8; 4] = [0xED, 0x42, 0x45, 0xFF];
const BADGE_WHITE: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

fn badge_label(count: i32) -> Vec<char> {
  if count < 0 {
    Vec::new()
  } else if count > 9 {
    vec!['9', '+']
  } else {
    count.to_string().chars().collect()
  }
}

fn badge_glyph(ch: char) -> Option<[&'static str; 5]> {
  match ch {
    '0' => Some(["111", "101", "101", "101", "111"]),
    '1' => Some(["010", "110", "010", "010", "111"]),
    '2' => Some(["111", "001", "111", "100", "111"]),
    '3' => Some(["111", "001", "111", "001", "111"]),
    '4' => Some(["101", "101", "111", "001", "001"]),
    '5' => Some(["111", "100", "111", "001", "111"]),
    '6' => Some(["111", "100", "111", "101", "111"]),
    '7' => Some(["111", "001", "001", "010", "010"]),
    '8' => Some(["111", "101", "111", "101", "111"]),
    '9' => Some(["111", "101", "111", "001", "111"]),
    '+' => Some(["000", "010", "111", "010", "000"]),
    _ => None,
  }
}

fn in_rounded_rect(x: i32, y: i32, rect_x: i32, rect_y: i32, w: i32, h: i32, r: i32) -> bool {
  if r == 0 {
    return true;
  }
  let (lx, ly) = (rect_x + r, rect_y + r);
  let (rx, ry) = (rect_x + w - r - 1, rect_y + h - r - 1);
  if x < lx && y < ly {
    let dx = x - lx;
    let dy = y - ly;
    dx * dx + dy * dy <= r * r
  } else if x > rx && y < ly {
    let dx = x - rx;
    let dy = y - ly;
    dx * dx + dy * dy <= r * r
  } else if x < lx && y > ry {
    let dx = x - lx;
    let dy = y - ry;
    dx * dx + dy * dy <= r * r
  } else if x > rx && y > ry {
    let dx = x - rx;
    let dy = y - ry;
    dx * dx + dy * dy <= r * r
  } else {
    true
  }
}

fn blend_pixel(rgba: &mut [u8], px: i32, py: i32, color: [u8; 4]) {
  if px < 0 || py >= BADGE_ICON_SIZE as i32 || py < 0 {
    return;
  }
  let ux = px as usize;
  let uy = py as usize;
  if ux >= BADGE_ICON_SIZE {
    return;
  }

  let index = (uy * BADGE_ICON_SIZE + ux) * 4;
  let alpha = color[3] as f32 / 255.0;
  let inverse_alpha = 1.0 - alpha;

  rgba[index] = (color[0] as f32 * alpha + rgba[index] as f32 * inverse_alpha).round() as u8;
  rgba[index + 1] =
    (color[1] as f32 * alpha + rgba[index + 1] as f32 * inverse_alpha).round() as u8;
  rgba[index + 2] =
    (color[2] as f32 * alpha + rgba[index + 2] as f32 * inverse_alpha).round() as u8;
  rgba[index + 3] = ((color[3] as f32) + rgba[index + 3] as f32 * inverse_alpha)
    .round()
    .clamp(0.0, 255.0) as u8;
}

fn draw_rect(rgba: &mut [u8], x: i32, y: i32, w: i32, h: i32, r: i32, color: [u8; 4]) {
  let radius = r.max(0).min(w / 2).min(h / 2);
  for py in y..(y + h) {
    for px in x..(x + w) {
      if in_rounded_rect(px, py, x, y, w, h, radius) {
        blend_pixel(rgba, px, py, color);
      }
    }
  }
}

fn draw_glyph(rgba: &mut [u8], ox: i32, oy: i32, ch: char, scale: i32, color: [u8; 4]) {
  let Some(glyph) = badge_glyph(ch) else {
    return;
  };

  for (row_index, row) in glyph.iter().enumerate() {
    for (col_index, bit) in row.chars().enumerate() {
      if bit != '1' {
        continue;
      }
      for sy in 0..scale {
        for sx in 0..scale {
          blend_pixel(
            rgba,
            ox + (col_index as i32 * scale) + sx,
            oy + (row_index as i32 * scale) + sy,
            color,
          );
        }
      }
    }
  }
}

fn render_badge_png(count: i32) -> Vec<u8> {
  let mut rgba = vec![0_u8; BADGE_ICON_SIZE * BADGE_ICON_SIZE * 4];
  let label = badge_label(count);

  if count < 0 {
    draw_rect(&mut rgba, 1, 1, 14, 14, 7, BADGE_RED);
  } else {
    let scale = 2;
    let spacing = if label.len() > 1 { 1 } else { 0 };
    let glyph_w = 3 * scale;
    let glyph_h = 5 * scale;
    let label_w = (label.len() as i32 * glyph_w) + ((label.len().saturating_sub(1)) as i32 * spacing);
    let badge_h = 14;
    let badge_w = (label_w + 4).max(badge_h);
    let badge_x = ((BADGE_ICON_SIZE as i32 - badge_w) / 2).max(0);
    let badge_y = ((BADGE_ICON_SIZE as i32 - badge_h) / 2).max(0);

    draw_rect(&mut rgba, badge_x, badge_y, badge_w, badge_h, badge_h / 2, BADGE_RED);

    let text_x = badge_x + ((badge_w - label_w) / 2);
    let text_y = badge_y + ((badge_h - glyph_h) / 2);

    for (index, ch) in label.iter().enumerate() {
      draw_glyph(&mut rgba, text_x + index as i32 * (glyph_w + spacing), text_y, *ch, scale, BADGE_WHITE);
    }
  }

  let mut png_bytes = Vec::new();
  {
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    image::write_buffer_with_format(
      &mut cursor,
      &rgba,
      BADGE_ICON_SIZE as u32,
      BADGE_ICON_SIZE as u32,
      ColorType::Rgba8,
      ImageFormat::Png,
    )
    .expect("failed to encode badge png");
  }

  png_bytes
}

fn check_webrtc_support() {
  #[cfg(target_os = "linux")]
  {
    let webrtc_modules = ["gstreamer-webrtc-1.0", "nice"];
    let missing: Vec<&str> = webrtc_modules
      .iter()
      .filter(|module| {
        Command::new("pkg-config")
          .args(["--exists", module])
          .status()
          .map(|s| !s.success())
          .unwrap_or(true)
      })
      .copied()
      .collect();

    if !missing.is_empty() {
      println!("cargo:warning=");
      println!("cargo:warning=== Discord voice requires WebRTC-enabled WebKitGTK ===");
      println!("cargo:warning=Missing dependencies: {}", missing.join(", "));
      println!("cargo:warning=These are required for WebRTC support in WebKitGTK.");
      println!("cargo:warning=");
      println!("cargo:warning=Option 1: Use the Flatpak build (GNOME 48 runtime has WebRTC)");
      println!("cargo:warning=Option 2: Use Debian 12+ which ships WebRTC-enabled WebKitGTK:");
      println!("cargo:warning=  apt-get install libwebkit2gtk-4.1-dev");
      println!("cargo:warning=");
      println!("cargo:warning=Voice will not work without WebRTC-enabled WebKitGTK.");
      println!("cargo:warning=");
    }
  }
}

fn main() {
  check_webrtc_support();

  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=icons/icon.png");

  let out_dir = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR"));
  fs::create_dir_all(&out_dir).expect("failed to create OUT_DIR");

  for count in [-1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
    let badge_file_name = if count < 0 {
      "badge-unread.png".to_string()
    } else {
      format!("badge-{count}.png")
    };
    fs::write(out_dir.join(badge_file_name), render_badge_png(count))
      .expect("failed to write badge asset");
  }

  tauri_build::build()
}

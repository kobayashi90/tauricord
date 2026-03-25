use image::{
  ColorType, DynamicImage, ImageFormat,
  imageops::{self, FilterType},
};
use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
use std::{env, fs, path::PathBuf};

const BADGE_ICON_SIZE: usize = 16;
const TASKBAR_ICON_SIZE: u32 = 64;
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

fn blend_pixel(rgba: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
  if x < 0 || y < 0 {
    return;
  }

  let (x, y) = (x as usize, y as usize);
  if x >= BADGE_ICON_SIZE || y >= BADGE_ICON_SIZE {
    return;
  }

  let index = (y * BADGE_ICON_SIZE + x) * 4;
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

fn fill_rounded_rect(
  rgba: &mut [u8],
  x: i32,
  y: i32,
  rect_width: i32,
  rect_height: i32,
  radius: i32,
  color: [u8; 4],
) {
  let radius = radius.max(0).min(rect_width / 2).min(rect_height / 2);

  for py in y..(y + rect_height) {
    for px in x..(x + rect_width) {
      let inside = if radius == 0 {
        true
      } else if px < x + radius && py < y + radius {
        let dx = px - (x + radius);
        let dy = py - (y + radius);
        dx * dx + dy * dy <= radius * radius
      } else if px >= x + rect_width - radius && py < y + radius {
        let dx = px - (x + rect_width - radius - 1);
        let dy = py - (y + radius);
        dx * dx + dy * dy <= radius * radius
      } else if px < x + radius && py >= y + rect_height - radius {
        let dx = px - (x + radius);
        let dy = py - (y + rect_height - radius - 1);
        dx * dx + dy * dy <= radius * radius
      } else if px >= x + rect_width - radius && py >= y + rect_height - radius {
        let dx = px - (x + rect_width - radius - 1);
        let dy = py - (y + rect_height - radius - 1);
        dx * dx + dy * dy <= radius * radius
      } else {
        true
      };

      if inside {
        blend_pixel(rgba, px, py, color);
      }
    }
  }
}

fn draw_glyph(rgba: &mut [u8], x: i32, y: i32, ch: char, scale: i32, color: [u8; 4]) {
  let Some(glyph) = badge_glyph(ch) else {
    return;
  };

  for (row_index, row) in glyph.iter().enumerate() {
    for (column_index, bit) in row.chars().enumerate() {
      if bit != '1' {
        continue;
      }

      for sy in 0..scale {
        for sx in 0..scale {
          blend_pixel(
            rgba,
            x + (column_index as i32 * scale) + sx,
            y + (row_index as i32 * scale) + sy,
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
    fill_rounded_rect(&mut rgba, 1, 1, 14, 14, 7, BADGE_RED);
  } else {
    let scale = 2;
    let spacing = if label.len() > 1 { 1 } else { 0 };
    let glyph_width = 3 * scale;
    let glyph_height = 5 * scale;
    let label_width = (label.len() as i32 * glyph_width)
      + ((label.len().saturating_sub(1)) as i32 * spacing);
    let badge_height = 14;
    let badge_width = (label_width + 4).max(badge_height);
    let badge_x = ((BADGE_ICON_SIZE as i32 - badge_width) / 2).max(0);
    let badge_y = ((BADGE_ICON_SIZE as i32 - badge_height) / 2).max(0);

    fill_rounded_rect(
      &mut rgba,
      badge_x,
      badge_y,
      badge_width,
      badge_height,
      badge_height / 2,
      BADGE_RED,
    );

    let text_x = badge_x + ((badge_width - label_width) / 2);
    let text_y = badge_y + ((badge_height - glyph_height) / 2);

    for (index, ch) in label.iter().enumerate() {
      draw_glyph(
        &mut rgba,
        text_x + index as i32 * (glyph_width + spacing),
        text_y,
        *ch,
        scale,
        BADGE_WHITE,
      );
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

fn render_taskbar_icon_png(base_icon: &DynamicImage, count: i32) -> Vec<u8> {
  let mut canvas = base_icon
    .resize_exact(TASKBAR_ICON_SIZE, TASKBAR_ICON_SIZE, FilterType::Lanczos3)
    .to_rgba8();

  let badge = image::load_from_memory(&render_badge_png(count))
    .expect("generated badge png should load")
    .resize_exact(26, 26, FilterType::Lanczos3)
    .to_rgba8();

  let badge_x = (TASKBAR_ICON_SIZE as i64 - badge.width() as i64 - 2).max(0) as i64;
  let badge_y = 2_i64;
  imageops::overlay(&mut canvas, &badge, badge_x, badge_y);

  let mut png_bytes = Vec::new();
  {
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    image::write_buffer_with_format(
      &mut cursor,
      canvas.as_raw(),
      canvas.width(),
      canvas.height(),
      ColorType::Rgba8,
      ImageFormat::Png,
    )
    .expect("failed to encode taskbar icon png");
  }

  png_bytes
}

fn encode_ico(width: u32, height: u32, rgba: Vec<u8>) -> Vec<u8> {
  let mut icon_dir = IconDir::new(ResourceType::Icon);
  let icon_image = IconImage::from_rgba_data(width, height, rgba);
  icon_dir
    .add_entry(IconDirEntry::encode(&icon_image).expect("failed to encode ico entry"));

  let mut ico_bytes = Vec::new();
  icon_dir
    .write(std::io::Cursor::new(&mut ico_bytes))
    .expect("failed to encode ico file");
  ico_bytes
}

fn render_taskbar_icon_ico(base_icon: &DynamicImage, count: Option<i32>) -> Vec<u8> {
  let rgba = match count {
    None => base_icon
      .resize_exact(TASKBAR_ICON_SIZE, TASKBAR_ICON_SIZE, FilterType::Lanczos3)
      .to_rgba8()
      .into_raw(),
    Some(value) => image::load_from_memory(&render_taskbar_icon_png(base_icon, value))
      .expect("generated taskbar png should load")
      .to_rgba8()
      .into_raw(),
  };

  encode_ico(TASKBAR_ICON_SIZE, TASKBAR_ICON_SIZE, rgba)
}

fn main() {
  println!("cargo:rerun-if-changed=build.rs");
  println!("cargo:rerun-if-changed=icons/icon.png");

  let out_dir = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR"));
  fs::create_dir_all(&out_dir).expect("failed to create OUT_DIR");

  let icon_path = PathBuf::from("icons").join("icon.png");
  let base_icon = image::open(&icon_path).expect("failed to load icons/icon.png");

  let clear_icon = base_icon
    .resize_exact(TASKBAR_ICON_SIZE, TASKBAR_ICON_SIZE, FilterType::Lanczos3)
    .to_rgba8();
  let mut clear_png = Vec::new();
  {
    let mut cursor = std::io::Cursor::new(&mut clear_png);
    image::write_buffer_with_format(
      &mut cursor,
      clear_icon.as_raw(),
      clear_icon.width(),
      clear_icon.height(),
      ColorType::Rgba8,
      ImageFormat::Png,
    )
    .expect("failed to encode clear taskbar icon");
  }
  fs::write(out_dir.join("taskbar-clear.png"), clear_png)
    .expect("failed to write clear taskbar icon");
  fs::write(
    out_dir.join("taskbar-clear.ico"),
    render_taskbar_icon_ico(&base_icon, None),
  )
  .expect("failed to write clear taskbar ico");

  for count in [-1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
    let badge_file_name = if count < 0 {
      "badge-unread.png".to_string()
    } else {
      format!("badge-{count}.png")
    };
    let taskbar_file_name = if count < 0 {
      "taskbar-unread.png".to_string()
    } else {
      format!("taskbar-{count}.png")
    };
    let taskbar_ico_file_name = if count < 0 {
      "taskbar-unread.ico".to_string()
    } else {
      format!("taskbar-{count}.ico")
    };

    fs::write(out_dir.join(badge_file_name), render_badge_png(count))
      .expect("failed to write badge asset");
    fs::write(out_dir.join(taskbar_file_name), render_taskbar_icon_png(&base_icon, count))
      .expect("failed to write taskbar icon asset");
    fs::write(
      out_dir.join(taskbar_ico_file_name),
      render_taskbar_icon_ico(&base_icon, Some(count)),
    )
    .expect("failed to write taskbar ico asset");
  }

  tauri_build::build()
}

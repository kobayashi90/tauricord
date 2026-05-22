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

fn detect_distro() -> Option<(String, String)> {
  let os_release = std::fs::read_to_string("/etc/os-release").ok()?;
  let mut id = None;
  let mut version_id = None;
  for line in os_release.lines() {
    if let Some(val) = line.strip_prefix("ID=") {
      id = Some(val.trim_matches('"').to_lowercase());
    } else if let Some(val) = line.strip_prefix("VERSION_ID=") {
      version_id = Some(val.trim_matches('"').to_string());
    }
  }
  Some((id?, version_id?))
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

    let distro_hint: Option<String> = detect_distro().and_then(|(id, ver)| {
      match (id.as_str(), ver.as_str()) {
        ("ubuntu", v) if v.starts_with("22") => Some(format!(
          "Ubuntu 22.04 needs the WebRTC PPA (Ubuntu-only, won't work on Debian):\n  sudo add-apt-repository ppa:escalion/ppa-webkit2gtk-experimental\n  sudo apt update && sudo apt install libwebkit2gtk-4.1-dev"
        )),
        ("ubuntu", v) if v.starts_with("24") => None,
        ("ubuntu", v) => Some(format!(
          "Ubuntu {v} may need the WebRTC PPA (Ubuntu-only, won't work on Debian):\n  sudo add-apt-repository ppa:escalion/ppa-webkit2gtk-experimental\n  sudo apt update && sudo apt install libwebkit2gtk-4.1-dev"
        )),
        ("debian", v) if v.starts_with("12") => Some(
          "Debian 12 stock WebKitGTK 2.50.x lacks WebRTC. Options:\n  (1) Debian experimental (risky):\n      echo 'deb http://deb.debian.org/debian experimental main' >> /etc/apt/sources.list\n      apt update && apt install -t experimental libwebkit2gtk-4.1-dev\n  (2) Flatpak build (bundles WebRTC):\n      Use '--bundles flatpak' instead of '--bundles deb'\n  (3) Use the distro-agnostic AppImage".into(),
        ),
        ("debian", _) => Some(
          "Debian stock WebKitGTK may lack WebRTC. Options:\n  Install from experimental, use the Flatpak build, or the AppImage.".into(),
        ),
        ("arch", _) => Some("Arch Linux:\n  yay -S webkitgtk-4.1-webrtee".into()),
        ("fedora", _) => Some("Fedora:\n  sudo dnf copr enable grul/LibWebKit\n  sudo dnf install webkitgtk".into()),
        ("nixos", _) => Some("NixOS:\n  webkitgtk.override { enableWebRTC = true; }".into()),
        _ => None,
      }
    });

    let missing_runtime = ["gstreamer-webrtc-1.0", "nice"]
      .iter()
      .filter(|module| {
        Command::new("pkg-config").args(["--exists", module]).status().map(|s| !s.success()).unwrap_or(true)
      })
      .copied()
      .collect::<Vec<&str>>();

    if !missing.is_empty() {
      println!("cargo:warning=");
      println!("cargo:warning=== Discord voice requires WebRTC-enabled WebKitGTK ===");
      println!("cargo:warning=Missing build deps: {}", missing.join(", "));
      println!("cargo:warning=");
      println!("cargo:warning=WebKitGTK must have WebRTC compiled in at build time.");
      println!("cargo:warning=Ubuntu 24.04+ ships WebRTC-enabled WebKitGTK. Debian 12 stock does NOT.");
      println!("cargo:warning=The Ubuntu PPA (ppa:escalion/ppa-webkit2gtk-experimental) is Ubuntu-only;");
      if let Some(hint) = distro_hint.as_deref() {
        println!("cargo:warning={hint}");
      }
      println!("cargo:warning=");
      println!("cargo:warning=Runtime deps (install alongside the .deb):");
      println!("cargo:warning=  sudo apt install gstreamer1.0-plugins-bad libnice10 libwebrtc-audio-processing1");
      println!("cargo:warning=");
      println!("cargo:warning=Voice will not work without WebRTC-enabled WebKitGTK.");
      println!("cargo:warning=");
    } else if !missing_runtime.is_empty() {
      println!("cargo:warning=");
      println!("cargo:warning=Build deps found, but runtime packages may be missing:");
      println!("cargo:warning=  sudo apt install gstreamer1.0-plugins-bad libnice10 libwebrtc-audio-processing1");
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

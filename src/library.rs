//! Quét thư viện tài nguyên (tiêu đề động, chuyển cảnh, nền…).

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    Titles,
    Transitions,
    Gradients,
    Social,
    Media,
    Other,
}

impl Category {
    pub fn label_vi(self) -> &'static str {
        match self {
            Category::Titles => "Tiêu đề động",
            Category::Transitions => "Chuyển cảnh",
            Category::Gradients => "Nền chuyển sắc",
            Category::Social => "Mạng xã hội",
            Category::Media => "Phương tiện",
            Category::Other => "Khác",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssetKind {
    Gif,
    Image,
    Video,
    Audio,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub name: String,
    pub path: PathBuf,
    pub kind: AssetKind,
    pub category: Category,
}

fn kind_of(path: &Path) -> Option<AssetKind> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "gif" => Some(AssetKind::Gif),
        "png" | "jpg" | "jpeg" | "webp" | "bmp" => Some(AssetKind::Image),
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => Some(AssetKind::Video),
        "mp3" | "wav" | "m4a" | "aac" | "ogg" | "flac" => Some(AssetKind::Audio),
        _ => None,
    }
}

fn category_of(rel: &Path) -> Category {
    let text = rel.to_string_lossy().to_ascii_lowercase();
    if text.contains("title") || text.contains("lower third") {
        Category::Titles
    } else if text.contains("transition") {
        Category::Transitions
    } else if text.contains("gradient") {
        Category::Gradients
    } else if text.contains("social") {
        Category::Social
    } else {
        Category::Other
    }
}

/// Quét đệ quy một thư mục, giới hạn độ sâu để tránh treo.
pub fn scan(root: &Path) -> Vec<Asset> {
    let mut out = Vec::new();
    walk(root, root, 0, &mut out);
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<Asset>) {
    if depth > 5 || out.len() > 20_000 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk(root, &path, depth + 1, out);
        } else if let Some(kind) = kind_of(&path) {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            // bỏ qua các tệp html hướng dẫn và ảnh quá nhỏ
            if meta.len() < 1024 {
                continue;
            }
            out.push(Asset {
                name,
                path: path.clone(),
                kind,
                category: category_of(rel),
            });
        }
    }
}

/// Các vị trí có thể chứa thư viện đóng gói sẵn.
pub fn bundled_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.join("assets").join("library"));
            roots.push(dir.join("library"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("assets").join("library"));
        roots.push(cwd.join("library"));
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("library"));

    roots
}

pub fn default_root() -> PathBuf {
    for r in bundled_roots() {
        if r.is_dir() {
            return r;
        }
    }
    if let Some(home) = dirs_home() {
        let p = home.join("VieEdit").join("library");
        if p.is_dir() {
            return p;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn dirs_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

//! Hộp thoại chọn tệp dựng sẵn bằng egui (không phụ thuộc thư viện hệ thống).

use std::path::{Path, PathBuf};

#[derive(Clone)]
struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

pub struct FilePicker {
    pub open: bool,
    pub title: String,
    cwd: PathBuf,
    entries: Vec<Entry>,
    selected: Option<PathBuf>,
    exts: Vec<String>,
    save_mode: bool,
    save_name: String,
    show_all: bool,
    error: Option<String>,
    path_edit: String,
}

impl Default for FilePicker {
    fn default() -> Self {
        Self {
            open: false,
            title: String::new(),
            cwd: PathBuf::from("."),
            entries: Vec::new(),
            selected: None,
            exts: Vec::new(),
            save_mode: false,
            save_name: String::new(),
            show_all: false,
            error: None,
            path_edit: String::new(),
        }
    }
}

impl FilePicker {
    pub fn open_pick(&mut self, title: &str, start: &Path, exts: &[&str]) {
        self.open = true;
        self.title = title.to_string();
        self.save_mode = false;
        self.exts = exts.iter().map(|s| s.to_ascii_lowercase()).collect();
        self.show_all = self.exts.is_empty();
        self.selected = None;
        self.error = None;
        self.save_name.clear();
        self.set_dir(start);
    }

    pub fn open_save(&mut self, title: &str, start: &Path, default_name: &str, ext: &str) {
        self.open = true;
        self.title = title.to_string();
        self.save_mode = true;
        self.exts = vec![ext.to_ascii_lowercase()];
        self.show_all = false;
        self.selected = None;
        self.error = None;
        let base = start.to_path_buf();
        let dir = if base.is_dir() {
            base
        } else {
            base.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
        };
        self.save_name = default_name.to_string();
        self.set_dir(&dir);
    }

    fn set_dir(&mut self, dir: &Path) {
        let dir = dir.to_path_buf();
        self.cwd = if dir.is_dir() {
            dir
        } else {
            PathBuf::from(".")
        };
        self.path_edit = self.cwd.to_string_lossy().to_string();
        self.refresh();
    }

    fn refresh(&mut self) {
        self.entries.clear();
        let Ok(rd) = std::fs::read_dir(&self.cwd) else {
            self.error = Some(format!("Không đọc được thư mục: {}", self.cwd.display()));
            return;
        };
        self.error = None;
        for e in rd.flatten() {
            let path = e.path();
            let Ok(meta) = e.metadata() else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if meta.is_dir() {
                self.entries.push(Entry {
                    name,
                    path,
                    is_dir: true,
                    size: 0,
                });
            } else if self.show_all || self.matches(&path) {
                self.entries.push(Entry {
                    name,
                    path,
                    is_dir: false,
                    size: meta.len(),
                });
            }
        }
        self.entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    }

    fn matches(&self, path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        self.exts.iter().any(|e| *e == ext)
    }

    fn places(&self) -> Vec<(String, PathBuf)> {
        let mut v = Vec::new();
        #[cfg(windows)]
        {
            for letter in b'C'..=b'Z' {
                let p = PathBuf::from(format!("{}:\\", letter as char));
                if p.is_dir() {
                    v.push((format!("{}:", letter as char), p));
                }
            }
        }
        #[cfg(not(windows))]
        {
            v.push(("Gốc".into(), PathBuf::from("/")));
            v.push(("Nhà".into(), PathBuf::from("/home")));
        }
        if let Some(home) = crate::library::dirs_home() {
            v.insert(0, ("Nhà".into(), home.clone()));
            for sub in ["Desktop", "Downloads", "Videos", "Documents"] {
                let p = home.join(sub);
                if p.is_dir() {
                    v.push((sub.to_string(), p));
                }
            }
        }
        v
    }

    /// Trả về đường dẫn khi người dùng xác nhận.
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        if !self.open {
            return None;
        }

        let mut confirm: Option<PathBuf> = None;
        let mut close = false;
        let mut navigate: Option<PathBuf> = None;

        egui::Window::new(&self.title)
            .collapsible(false)
            .resizable(true)
            .default_size([760.0, 520.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (label, path) in self.places() {
                        if ui.small_button(label).clicked() {
                            navigate = Some(path);
                        }
                    }
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Đường dẫn:");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.path_edit)
                            .desired_width(f32::INFINITY),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        navigate = Some(PathBuf::from(self.path_edit.clone()));
                    }
                });

                ui.separator();

                let mut up = None;
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        if let Some(parent) = self.cwd.parent() {
                            if ui
                                .add(egui::Button::new("⬆  ..").frame(false))
                                .clicked()
                            {
                                up = Some(parent.to_path_buf());
                            }
                        }
                        let entries = self.entries.clone();
                        for e in entries {
                            if e.is_dir {
                                let text = format!("📁  {}", e.name);
                                if ui.add(egui::Button::new(text).frame(false)).clicked() {
                                    navigate = Some(e.path.clone());
                                }
                            } else {
                                let selected = self.selected.as_deref() == Some(e.path.as_path());
                                let text = format!(
                                    "🎞  {}   ({})",
                                    e.name,
                                    human_size(e.size)
                                );
                                let resp = ui.selectable_label(selected, text);
                                if resp.clicked() {
                                    self.selected = Some(e.path.clone());
                                    self.save_name = e.name.clone();
                                }
                                if resp.double_clicked() {
                                    confirm = Some(e.path.clone());
                                }
                            }
                        }
                    });
                if let Some(p) = up {
                    navigate = Some(p);
                }

                if let Some(err) = &self.error {
                    ui.colored_label(egui::Color32::from_rgb(255, 120, 120), err);
                }

                ui.separator();

                if self.save_mode {
                    ui.horizontal(|ui| {
                        ui.label("Tên tệp:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.save_name)
                                .desired_width(320.0),
                        );
                    });
                    ui.add_space(4.0);
                }

                ui.horizontal(|ui| {
                    if ui.button("Hủy").clicked() {
                        close = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if self.save_mode { "Lưu" } else { "Chọn" };
                        if ui.button(label).clicked() {
                            if self.save_mode {
                                let name = self.save_name.trim().to_string();
                                if !name.is_empty() {
                                    confirm = Some(self.cwd.join(name));
                                }
                            } else if let Some(p) = &self.selected {
                                confirm = Some(p.clone());
                            }
                        }
                    });
                });
            });

        if let Some(dir) = navigate {
            self.set_dir(&dir);
        }

        if let Some(path) = confirm {
            if self.save_mode {
                if let Some(parent) = path.parent() {
                    self.cwd = parent.to_path_buf();
                }
            }
            self.open = false;
            return Some(path);
        }
        if close {
            self.open = false;
        }
        None
    }
}

pub fn human_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b > 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GB", b / (1024.0 * 1024.0 * 1024.0))
    } else if b > 1024.0 * 1024.0 {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    } else if b > 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

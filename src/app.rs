//! Giao diện chính của VieEdit.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use eframe::egui;
use egui::{Color32, FontId, RichText, Sense};

use crate::filepicker::FilePicker;
use crate::i18n::{tr, Lang, Tr};
use crate::library::{self, Asset, AssetKind, Category};
use crate::media::{self, ExportJob, ExportMsg, FrameLoader, FrameReq, Quality};
use crate::model::{
    fmt_dur, fmt_time, Clip, Project, Source, SourceKind, TRACK_OVERLAY, TRACK_VIDEO,
};
use crate::settings;
use crate::theme;
use crate::timeline::{self, TimelineState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Media,
    Titles,
    Transitions,
    Gradients,
    Social,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PickPurpose {
    ImportMedia,
    OpenProject,
    SaveProject,
    ExportOutput,
    LibraryFolder,
}

struct Toast {
    text: String,
    ok: bool,
    until: f64,
}

struct GifAnim {
    frames: Vec<egui::TextureHandle>,
    delays: Vec<f32>,
    total: f32,
}

pub struct VieEdit {
    project: Project,
    project_path: Option<PathBuf>,
    lang: Lang,

    library_root: PathBuf,
    assets: Vec<Asset>,
    tab: Tab,
    search: String,

    selected: Option<u64>,
    playhead: f64,
    playing: bool,
    timeline: TimelineState,

    undo: Vec<Project>,
    redo: Vec<Project>,

    images: HashMap<u64, egui::TextureHandle>,
    gifs: HashMap<u64, GifAnim>,
    gif_order: VecDeque<u64>,
    frames: HashMap<String, egui::TextureHandle>,
    frame_order: VecDeque<String>,
    loader: FrameLoader,

    thumbs: HashMap<String, Option<egui::TextureHandle>>,
    thumb_budget: u32,

    picker: FilePicker,
    picker_purpose: PickPurpose,

    toast: Option<Toast>,
    pending_toast: Option<(String, bool)>,
    show_about: bool,
    show_shortcuts: bool,

    show_export: bool,
    export_quality: Quality,
    export_path: String,
    export_job: Option<ExportJob>,
    export_log: String,

    ffmpeg_ok: bool,
    ffmpeg_line: String,
}

impl VieEdit {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        let saved = settings::load();
        let lang = match saved.lang.as_deref() {
            Some("en") => Lang::En,
            _ => Lang::Vi,
        };

        let mut library_root = library::default_root();
        if let Some(root) = saved.library_root.as_deref() {
            let p = PathBuf::from(root);
            if p.is_dir() {
                library_root = p;
            }
        }

        let assets = if library_root.is_dir() {
            library::scan(&library_root)
        } else {
            Vec::new()
        };

        Self {
            project: Project::default(),
            project_path: None,
            lang,
            library_root,
            assets,
            tab: Tab::Titles,
            search: String::new(),
            selected: None,
            playhead: 0.0,
            playing: false,
            timeline: TimelineState::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            images: HashMap::new(),
            gifs: HashMap::new(),
            gif_order: VecDeque::new(),
            frames: HashMap::new(),
            frame_order: VecDeque::new(),
            loader: FrameLoader::new(),
            thumbs: HashMap::new(),
            thumb_budget: 3,
            picker: FilePicker::default(),
            picker_purpose: PickPurpose::ImportMedia,
            toast: None,
            pending_toast: None,
            show_about: false,
            show_shortcuts: false,
            show_export: false,
            export_quality: Quality::Standard,
            export_path: String::new(),
            export_job: None,
            export_log: String::new(),
            ffmpeg_ok: media::available(),
            ffmpeg_line: media::version_line(),
        }
    }

    // -----------------------------------------------------------------
    // Tiện ích
    // -----------------------------------------------------------------

    fn t(&self) -> Tr {
        tr(self.lang)
    }

    fn persist_settings(&self) {
        settings::save(&settings::Settings {
            library_root: Some(self.library_root.to_string_lossy().to_string()),
            lang: Some(self.lang.code().to_string()),
        });
    }

    fn toast_now(&mut self, ctx: &egui::Context, text: impl Into<String>, ok: bool) {
        let now = ctx.input(|i| i.time);
        self.toast = Some(Toast {
            text: text.into(),
            ok,
            until: now + 3.6,
        });
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.project.clone());
        if self.undo.len() > 80 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn do_undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(self.project.clone());
            self.project = prev;
            self.selected = None;
        }
    }

    fn do_redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.project.clone());
            self.project = next;
            self.selected = None;
        }
    }

    fn add_source_from_path(&mut self, path: &Path) -> Option<u64> {
        let path_str = path.to_string_lossy().to_string();
        if let Some(s) = self.project.sources.iter().find(|s| s.path == path_str) {
            return Some(s.id);
        }
        let mut src = media::probe(path)?;
        src.id = self.project.new_id();
        let id = src.id;
        self.project.sources.push(src);
        Some(id)
    }

    fn add_clip_from_source(&mut self, src_id: u64, at: f64) -> Option<u64> {
        let src = self.project.source(src_id)?.clone();
        let track = match src.kind {
            SourceKind::Video => TRACK_VIDEO,
            _ => TRACK_OVERLAY,
        };
        let default_dur = match src.kind {
            SourceKind::Video => src.duration,
            SourceKind::Audio => src.duration,
            _ => 3.0,
        };
        let dur = default_dur.clamp(0.3, 600.0);
        let start = self.project.find_free_start(track, dur, at.max(0.0));
        let id = self.project.new_id();
        self.project.clips.push(Clip {
            id,
            source: src_id,
            track,
            start,
            in_pt: 0.0,
            dur,
            volume: 1.0,
            opacity: 1.0,
            text: String::new(),
        });
        Some(id)
    }

    fn add_asset(&mut self, ctx: &egui::Context, asset: &Asset) {
        let Some(src_id) = self.add_source_from_path(&asset.path) else {
            let msg = self.t().msg_unsupported.to_string();
            self.toast_now(ctx, msg, false);
            return;
        };
        self.checkpoint();
        let at = self.playhead;
        if let Some(id) = self.add_clip_from_source(src_id, at) {
            self.selected = Some(id);
            let msg = format!("Đã thêm “{}” vào dòng thời gian.", asset.name);
            self.toast_now(ctx, msg, true);
        }
    }

    fn split_at_playhead(&mut self) {
        let Some(id) = self.selected else { return };
        let Some(clip) = self.project.clip(id).cloned() else {
            return;
        };
        let t = self.playhead;
        if t <= clip.start + 0.05 || t >= clip.end() - 0.05 {
            return;
        }
        self.checkpoint();
        let offset = t - clip.start;
        let new_id = self.project.new_id();
        if let Some(c) = self.project.clip_mut(id) {
            c.dur = offset;
        }
        self.project.clips.push(Clip {
            id: new_id,
            source: clip.source,
            track: clip.track,
            start: t,
            in_pt: clip.in_pt + offset,
            dur: clip.dur - offset,
            volume: clip.volume,
            opacity: clip.opacity,
            text: clip.text.clone(),
        });
        self.selected = Some(new_id);
    }

    fn delete_selected(&mut self) {
        if let Some(id) = self.selected {
            self.checkpoint();
            self.project.clips.retain(|c| c.id != id);
            self.selected = None;
        }
    }

    fn duplicate_selected(&mut self) {
        let Some(id) = self.selected else { return };
        let Some(clip) = self.project.clip(id).cloned() else {
            return;
        };
        self.checkpoint();
        let new_id = self.project.new_id();
        let start = self.project.find_free_start(clip.track, clip.dur, clip.end());
        self.project.clips.push(Clip {
            id: new_id,
            start,
            ..clip
        });
        self.selected = Some(new_id);
    }

    // -----------------------------------------------------------------
    // Thanh trên
    // -----------------------------------------------------------------

    fn top_bar(&mut self, ctx: &egui::Context) {
        let tr = self.t();
        egui::TopBottomPanel::top("topbar")
            .exact_height(46.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new("VIE")
                            .color(theme::ACCENT)
                            .size(17.0)
                            .strong(),
                    );
                    ui.label(RichText::new("EDIT").color(theme::TEXT).size(17.0).strong());
                    ui.add_space(3.0);
                    ui.label(RichText::new(tr.tagline).size(10.0).color(theme::FAINT));
                    ui.add_space(14.0);

                    egui::menu::bar(ui, |ui| {
                        ui.menu_button(tr.menu_file, |ui| {
                            if ui.button(tr.menu_new).clicked() {
                                self.checkpoint();
                                self.project = Project::default();
                                self.project_path = None;
                                self.selected = None;
                                self.playhead = 0.0;
                                ui.close_menu();
                            }
                            if ui.button(tr.menu_open).clicked() {
                                self.picker_purpose = PickPurpose::OpenProject;
                                let start = self
                                    .project_path
                                    .clone()
                                    .unwrap_or_else(|| PathBuf::from("."));
                                self.picker
                                    .open_pick(&tr.menu_open, &start, &["vieedit", "json"]);
                                ui.close_menu();
                            }
                            if ui.button(tr.menu_save).clicked() {
                                self.save_project(false);
                                ui.close_menu();
                            }
                            if ui.button(tr.menu_save_as).clicked() {
                                self.save_project(true);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(tr.menu_export).clicked() {
                                self.open_export(ctx);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(tr.menu_exit).clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });

                        ui.menu_button(tr.menu_edit, |ui| {
                            let can_undo = !self.undo.is_empty();
                            if ui
                                .add_enabled(can_undo, egui::Button::new(tr.menu_undo))
                                .clicked()
                            {
                                self.do_undo();
                                ui.close_menu();
                            }
                            let can_redo = !self.redo.is_empty();
                            if ui
                                .add_enabled(can_redo, egui::Button::new(tr.menu_redo))
                                .clicked()
                            {
                                self.do_redo();
                                ui.close_menu();
                            }
                            ui.separator();
                            let has_sel = self.selected.is_some();
                            if ui
                                .add_enabled(has_sel, egui::Button::new(tr.btn_split))
                                .clicked()
                            {
                                self.split_at_playhead();
                                ui.close_menu();
                            }
                            if ui
                                .add_enabled(has_sel, egui::Button::new(tr.btn_duplicate))
                                .clicked()
                            {
                                self.duplicate_selected();
                                ui.close_menu();
                            }
                            if ui
                                .add_enabled(has_sel, egui::Button::new(tr.btn_delete))
                                .clicked()
                            {
                                self.delete_selected();
                                ui.close_menu();
                            }
                        });

                        ui.menu_button(tr.menu_help, |ui| {
                            if ui.button(tr.menu_shortcuts).clicked() {
                                self.show_shortcuts = true;
                                ui.close_menu();
                            }
                            if ui.button(tr.menu_about).clicked() {
                                self.show_about = true;
                                ui.close_menu();
                            }
                            ui.separator();
                            ui.label(RichText::new(tr.menu_language).small().color(theme::FAINT));
                            if ui.selectable_label(self.lang == Lang::Vi, "Tiếng Việt").clicked() {
                                self.lang = Lang::Vi;
                                self.persist_settings();
                                ui.close_menu();
                            }
                            if ui.selectable_label(self.lang == Lang::En, "English").clicked() {
                                self.lang = Lang::En;
                                self.persist_settings();
                                ui.close_menu();
                            }
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(theme::accent_button(&format!("⬇  {}", tr.btn_export)))
                            .clicked()
                        {
                            self.open_export(ctx);
                        }
                        ui.add_space(8.0);
                        let status = if self.ffmpeg_ok {
                            RichText::new("● FFmpeg").size(10.0).color(theme::GREEN)
                        } else {
                            RichText::new("● FFmpeg").size(10.0).color(theme::ACCENT)
                        };
                        ui.label(status);
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(format!(
                                "{} × {} · {:.0} fps",
                                self.project.width, self.project.height, self.project.fps
                            ))
                            .size(10.0)
                            .color(theme::FAINT),
                        );
                    });
                });
            });
    }

    // -----------------------------------------------------------------
    // Thư viện
    // -----------------------------------------------------------------

    fn library_panel(&mut self, ctx: &egui::Context) {
        let tr = self.t();
        egui::SidePanel::left("library")
            .resizable(true)
            .default_width(316.0)
            .width_range(230.0..=540.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr.panel_library).size(13.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(tr.btn_reload).clicked() {
                            self.reload_library();
                        }
                    });
                });
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui
                        .add(theme::accent_button(&format!("＋ {}", tr.btn_import)))
                        .clicked()
                    {
                        self.picker_purpose = PickPurpose::ImportMedia;
                        let start = library::dirs_home().unwrap_or_else(|| PathBuf::from("."));
                        self.picker.open_pick(
                            tr.btn_import,
                            &start,
                            &[
                                "mp4", "mov", "mkv", "webm", "avi", "m4v", "png", "jpg", "jpeg",
                                "webp", "gif", "mp3", "wav", "m4a", "aac", "ogg", "flac",
                            ],
                        );
                    }
                    if ui.add(theme::ghost_button("📁")).clicked() {
                        self.picker_purpose = PickPurpose::LibraryFolder;
                        let start = self.library_root.clone();
                        self.picker.open_pick(tr.lbl_library_path, &start, &[]);
                    }
                });

                ui.add_space(6.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(tr.lbl_search)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(6.0);

                let tabs = [
                    (Tab::Media, tr.tab_media),
                    (Tab::Titles, tr.tab_titles),
                    (Tab::Transitions, tr.tab_transitions),
                    (Tab::Gradients, tr.tab_gradients),
                    (Tab::Social, tr.tab_social),
                ];
                ui.horizontal_wrapped(|ui| {
                    for (tab, label) in tabs {
                        let on = self.tab == tab;
                        let btn = egui::Button::new(
                            RichText::new(label)
                                .size(10.5)
                                .color(if on { Color32::WHITE } else { theme::DIM }),
                        )
                        .fill(if on { theme::ACCENT_DIM } else { theme::PANEL_2 })
                        .rounding(egui::Rounding::same(20.0))
                        .stroke(egui::Stroke::new(                             1.0_f32,
                            if on { theme::ACCENT } else { theme::BORDER },
                        ));
                        if ui.add(btn).clicked() {
                            self.tab = tab;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.separator();

                if self.tab == Tab::Media {
                    self.media_grid(ui);
                } else {
                    self.asset_grid(ui);
                }
            });
    }

    fn reload_library(&mut self) {
        self.thumbs.clear();
        self.assets = library::scan(&self.library_root);
    }

    fn asset_grid(&mut self, ui: &mut egui::Ui) {
        let tr = self.t();
        let cat = match self.tab {
            Tab::Titles => Some(Category::Titles),
            Tab::Transitions => Some(Category::Transitions),
            Tab::Gradients => Some(Category::Gradients),
            Tab::Social => Some(Category::Social),
            Tab::Media => None,
        };
        let needle = self.search.to_lowercase();
        let assets: Vec<Asset> = self
            .assets
            .iter()
            .filter(|a| cat.map(|c| a.category == c).unwrap_or(true))
            .filter(|a| needle.is_empty() || a.name.to_lowercase().contains(&needle))
            .take(600)
            .cloned()
            .collect();

        if assets.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(tr.msg_empty_library).color(theme::FAINT));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(self.library_root.display().to_string())
                        .size(9.5)
                        .color(theme::FAINT),
                );
                ui.add_space(10.0);
                if ui.add(theme::ghost_button(tr.lbl_library_path)).clicked() {
                    self.picker_purpose = PickPurpose::LibraryFolder;
                    let start = self.library_root.clone();
                    self.picker.open_pick(tr.lbl_library_path, &start, &[]);
                }
            });
            return;
        }

        let cols = ((ui.available_width() + 8.0) / 118.0).floor().max(1.0) as usize;
        let cell_w = (ui.available_width() - 8.0 * (cols as f32 - 1.0)) / cols as f32;
        self.thumb_budget = 3;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for chunk in assets.chunks(cols) {
                    ui.horizontal(|ui| {
                        for a in chunk {
                            self.asset_card(ui, a, cell_w);
                        }
                    });
                    ui.add_space(4.0);
                }
            });
    }

    fn asset_card(&mut self, ui: &mut egui::Ui, asset: &Asset, w: f32) {
        let key = asset.path.to_string_lossy().to_string();
        let ctx = ui.ctx().clone();
        let tex = self.thumb_for(&ctx, &key, &asset.path, asset.kind);

        let h = w * 0.62;
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h + 26.0), Sense::click_and_drag());

        let painter = ui.painter();
        painter.rect_filled(rect, egui::Rounding::same(7.0), theme::PANEL_2);

        let img_rect = egui::Rect::from_min_size(rect.min, egui::vec2(w, h));
        if let Some(t) = &tex {
            paint_tex_into(painter, img_rect, t, 1.0);
        } else {
            painter.text(
                img_rect.center(),
                egui::Align2::CENTER_CENTER,
                "…",
                FontId::proportional(16.0),
                theme::FAINT,
            );
        }

        painter.text(
            egui::pos2(rect.left() + 6.0, rect.bottom() - 17.0),
            egui::Align2::LEFT_TOP,
            truncate(&asset.name, 18),
            FontId::proportional(9.5),
            theme::DIM,
        );

        if resp.hovered() {
            painter.rect_stroke(
                rect,
                egui::Rounding::same(7.0),
                egui::Stroke::new(1.5_f32, theme::ACCENT),
            );
        }

        if resp.double_clicked() {
            self.add_asset(&ctx, asset);
        }

        let add_label = self.t().btn_add_timeline;
        resp.context_menu(|ui| {
            if ui.button(add_label).clicked() {
                ui.close_menu();
            }
        });
    }

    fn thumb_for(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        path: &Path,
        kind: AssetKind,
    ) -> Option<egui::TextureHandle> {
        if let Some(v) = self.thumbs.get(key) {
            return v.clone();
        }
        if self.thumb_budget == 0 {
            return None;
        }
        self.thumb_budget -= 1;
        let tex = make_thumb(ctx, path, kind);
        self.thumbs.insert(key.to_string(), tex.clone());
        tex
    }

    fn media_grid(&mut self, ui: &mut egui::Ui) {
        let tr = self.t();
        if self.project.sources.is_empty() {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(tr.msg_empty_media).color(theme::FAINT));
            });
            return;
        }
        let needle = self.search.to_lowercase();
        let items: Vec<Source> = self
            .project
            .sources
            .iter()
            .filter(|s| needle.is_empty() || s.name.to_lowercase().contains(&needle))
            .cloned()
            .collect();

        let cols = ((ui.available_width() + 8.0) / 118.0).floor().max(1.0) as usize;
        let cell_w = (ui.available_width() - 8.0 * (cols as f32 - 1.0)) / cols as f32;
        self.thumb_budget = 3;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for chunk in items.chunks(cols) {
                    ui.horizontal(|ui| {
                        for s in chunk {
                            self.source_card(ui, s, cell_w);
                        }
                    });
                    ui.add_space(4.0);
                }
            });
    }

    fn source_card(&mut self, ui: &mut egui::Ui, s: &Source, w: f32) {
        let ctx = ui.ctx().clone();
        let key = format!("src:{}", s.id);
        let kind = match s.kind {
            SourceKind::Video => AssetKind::Video,
            SourceKind::Image => AssetKind::Image,
            SourceKind::Gif => AssetKind::Gif,
            SourceKind::Audio => AssetKind::Audio,
        };
        let tex = self.thumb_for(&ctx, &key, Path::new(&s.path), kind);

        let h = w * 0.62;
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h + 34.0), Sense::click());
        let painter = ui.painter();
        painter.rect_filled(rect, egui::Rounding::same(7.0), theme::PANEL_2);
        let img_rect = egui::Rect::from_min_size(rect.min, egui::vec2(w, h));
        if let Some(t) = &tex {
            paint_tex_into(painter, img_rect, t, 1.0);
        }
        painter.text(
            egui::pos2(rect.left() + 6.0, rect.bottom() - 31.0),
            egui::Align2::LEFT_TOP,
            truncate(&s.name, 18),
            FontId::proportional(9.5),
            theme::DIM,
        );
        painter.text(
            egui::pos2(rect.left() + 6.0, rect.bottom() - 17.0),
            egui::Align2::LEFT_TOP,
            fmt_dur(s.duration),
            FontId::proportional(8.5),
            theme::FAINT,
        );

        if resp.double_clicked() {
            self.checkpoint();
            let at = self.playhead;
            if let Some(id) = self.add_clip_from_source(s.id, at) {
                self.selected = Some(id);
            }
        }
    }

    // -----------------------------------------------------------------
    // Thuộc tính
    // -----------------------------------------------------------------

    fn properties_panel(&mut self, ctx: &egui::Context) {
        let tr = self.t();
        egui::SidePanel::right("props")
            .resizable(true)
            .default_width(278.0)
            .width_range(215.0..=430.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                ui.label(RichText::new(tr.panel_props).size(13.0).strong());
                ui.add_space(6.0);
                ui.separator();

                let Some(id) = self.selected else {
                    ui.add_space(16.0);
                    ui.label(RichText::new(tr.msg_select_clip).color(theme::FAINT).size(11.0));
                    ui.add_space(20.0);
                    ui.separator();
                    self.project_info(ui);
                    return;
                };

                let Some(clip) = self.project.clip(id).cloned() else {
                    return;
                };
                let src = self.project.source(clip.source).cloned();

                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        src.as_ref()
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| "?".into()),
                    )
                    .size(12.5)
                    .strong(),
                );
                ui.add_space(2.0);
                if let Some(s) = &src {
                    ui.label(
                        RichText::new(s.path.clone())
                            .size(8.5)
                            .color(theme::FAINT),
                    );
                }
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr.lbl_start).size(10.5).color(theme::DIM));
                    let mut v = clip.start;
                    if ui
                        .add(egui::DragValue::new(&mut v).speed(0.02).range(0.0..=100000.0))
                        .changed()
                    {
                        if let Some(c) = self.project.clip_mut(id) {
                            c.start = v.max(0.0);
                        }
                    }
                    ui.label(RichText::new("s").size(9.0).color(theme::FAINT));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr.lbl_duration).size(10.5).color(theme::DIM));
                    let mut v = clip.dur;
                    if ui
                        .add(egui::DragValue::new(&mut v).speed(0.02).range(0.1..=10000.0))
                        .changed()
                    {
                        if let Some(c) = self.project.clip_mut(id) {
                            c.dur = v.max(0.1);
                        }
                    }
                    ui.label(RichText::new("s").size(9.0).color(theme::FAINT));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("In").size(10.5).color(theme::DIM));
                    let mut v = clip.in_pt;
                    if ui
                        .add(egui::DragValue::new(&mut v).speed(0.02).range(0.0..=10000.0))
                        .changed()
                    {
                        if let Some(c) = self.project.clip_mut(id) {
                            c.in_pt = v.max(0.0);
                        }
                    }
                    ui.label(RichText::new("s").size(9.0).color(theme::FAINT));
                });

                ui.add_space(8.0);
                ui.label(RichText::new(tr.lbl_opacity).size(10.5).color(theme::DIM));
                let mut op = clip.opacity;
                if ui.add(egui::Slider::new(&mut op, 0.0..=1.0)).changed() {
                    if let Some(c) = self.project.clip_mut(id) {
                        c.opacity = op;
                    }
                }
                ui.label(RichText::new(tr.lbl_volume).size(10.5).color(theme::DIM));
                let mut vol = clip.volume;
                if ui.add(egui::Slider::new(&mut vol, 0.0..=2.0)).changed() {
                    if let Some(c) = self.project.clip_mut(id) {
                        c.volume = vol;
                    }
                }

                ui.add_space(10.0);
                ui.label(RichText::new("Chữ trên video").size(10.5).color(theme::DIM));
                let mut text = clip.text.clone();
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut text)
                            .desired_rows(2)
                            .desired_width(f32::INFINITY),
                    )
                    .changed()
                {
                    if let Some(c) = self.project.clip_mut(id) {
                        c.text = text;
                    }
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.add(theme::ghost_button(tr.btn_split)).clicked() {
                        self.split_at_playhead();
                    }
                    if ui.add(theme::ghost_button(tr.btn_duplicate)).clicked() {
                        self.duplicate_selected();
                    }
                });
                ui.add_space(4.0);
                if ui.add(theme::ghost_button(tr.btn_delete)).clicked() {
                    self.delete_selected();
                }

                ui.add_space(14.0);
                ui.separator();
                self.project_info(ui);
            });
    }

    fn project_info(&mut self, ui: &mut egui::Ui) {
        let tr = self.t();
        ui.add_space(8.0);
        ui.label(RichText::new("Dự án").size(11.0).color(theme::DIM).strong());
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(tr.lbl_name).size(10.0).color(theme::FAINT));
            let mut name = self.project.name.clone();
            if ui
                .add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(150.0)
                        .hint_text(tr.untitled),
                )
                .changed()
            {
                self.project.name = name;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr.lbl_resolution).size(10.0).color(theme::FAINT));
            let mut w = self.project.width;
            let mut h = self.project.height;
            let cw = ui.add(egui::DragValue::new(&mut w).speed(1.0).range(160..=7680));
            ui.label("×");
            let ch = ui.add(egui::DragValue::new(&mut h).speed(1.0).range(120..=4320));
            if cw.changed() || ch.changed() {
                self.project.width = (w / 2) * 2;
                self.project.height = (h / 2) * 2;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr.lbl_fps).size(10.0).color(theme::FAINT));
            let mut fps = self.project.fps;
            if ui
                .add(egui::DragValue::new(&mut fps).speed(1.0).range(12.0..=120.0))
                .changed()
            {
                self.project.fps = fps;
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "{}: {}",
                tr.lbl_duration,
                fmt_time(self.project.duration())
            ))
            .size(10.0)
            .color(theme::FAINT),
        );
        ui.label(
            RichText::new(format!(
                "{}: {}",
                tr.lbl_source,
                self.project.sources.len()
            ))
            .size(10.0)
            .color(theme::FAINT),
        );

        ui.add_space(10.0);
        if !self.ffmpeg_ok {
            egui::Frame::none()
                .fill(Color32::from_rgb(60, 30, 30))
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(tr.msg_no_ffmpeg)
                            .size(10.5)
                            .color(theme::ACCENT)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(tr.msg_ffmpeg_hint)
                            .size(9.0)
                            .color(theme::DIM),
                    );
                });
        } else {
            ui.label(RichText::new(&self.ffmpeg_line).size(8.5).color(theme::FAINT));
        }
    }

    // -----------------------------------------------------------------
    // Dòng thời gian
    // -----------------------------------------------------------------

    fn timeline_panel(&mut self, ctx: &egui::Context) {
        let tr = self.t();
        egui::TopBottomPanel::bottom("timeline")
            .resizable(true)
            .default_height(292.0)
            .height_range(160.0..=580.0)
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr.panel_timeline).size(12.0).strong());

                    ui.add_space(10.0);
                    let play_label = if self.playing { "⏸" } else { "▶" };
                    if ui.add(theme::ghost_button(play_label)).clicked() {
                        self.playing = !self.playing;
                    }
                    if ui.add(theme::ghost_button("⏮")).clicked() {
                        self.playhead = 0.0;
                    }
                    if ui.add(theme::ghost_button("⏭")).clicked() {
                        self.playhead = self.project.duration();
                    }

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "{} / {}",
                            fmt_time(self.playhead),
                            fmt_time(self.project.duration())
                        ))
                        .size(11.0)
                        .color(theme::TEXT),
                    );
                    ui.add_space(10.0);
                    if ui.add(theme::ghost_button("✂")).clicked() {
                        self.split_at_playhead();
                    }
                    if ui.add(theme::ghost_button("🗑")).clicked() {
                        self.delete_selected();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Slider::new(&mut self.timeline.zoom, 12.0..=400.0)
                                .show_value(false),
                        );
                        ui.label(RichText::new(tr.lbl_zoom).size(9.5).color(theme::FAINT));
                    });
                });
                ui.add_space(4.0);

                let out = timeline::show(
                    ui,
                    &mut self.project,
                    &mut self.timeline,
                    &mut self.selected,
                    &mut self.playhead,
                    &tr,
                );
                let _ = out.changed;
                if let Some(snap) = out.snapshot {
                    self.undo.push(snap);
                    if self.undo.len() > 80 {
                        self.undo.remove(0);
                    }
                    self.redo.clear();
                }
            });
    }

    // -----------------------------------------------------------------
    // Xem trước
    // -----------------------------------------------------------------

    fn preview_panel(&mut self, ctx: &egui::Context) {
        let tr = self.t();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr.panel_preview).size(11.0).color(theme::FAINT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} × {}",
                                self.project.width, self.project.height
                            ))
                            .size(9.5)
                            .color(theme::FAINT),
                        );
                    });
                });
                ui.add_space(4.0);

                let avail = ui.available_size();
                let aspect = self.project.width as f32 / self.project.height.max(1) as f32;
                let mut w = avail.x;
                let mut h = w / aspect;
                if h > avail.y {
                    h = avail.y;
                    w = h * aspect;
                }
                let (rect, _) = ui.allocate_exact_size(avail, Sense::click());
                let canvas =
                    egui::Rect::from_center_size(rect.center(), egui::vec2(w.max(40.0), h.max(40.0)));

                let (base, overlay, text) = self.preview_layers(ctx);

                let painter = ui.painter();
                painter.rect_filled(rect, egui::Rounding::same(8.0), Color32::from_rgb(8, 9, 12));
                painter.rect_filled(canvas, egui::Rounding::same(5.0), Color32::BLACK);

                if let Some(tex) = base {
                    draw_texture_fit(painter, canvas, &tex, 1.0);
                } else {
                    painter.text(
                        canvas.center(),
                        egui::Align2::CENTER_CENTER,
                        if self.project.clips.is_empty() {
                            tr.msg_drop_hint
                        } else {
                            ""
                        },
                        FontId::proportional(12.0),
                        theme::FAINT,
                    );
                }
                if let Some((tex, op)) = overlay {
                    draw_texture_fit(painter, canvas, &tex, op);
                }
                if let Some(txt) = text {
                    painter.text(
                        egui::pos2(canvas.center().x, canvas.bottom() - canvas.height() * 0.12),
                        egui::Align2::CENTER_CENTER,
                        txt,
                        FontId::proportional((canvas.height() * 0.055).max(10.0)),
                        Color32::WHITE,
                    );
                }

                painter.rect_stroke(
                    canvas,
                    egui::Rounding::same(5.0),
                    egui::Stroke::new(1.0_f32, theme::BORDER),
                );
            });
    }

    fn preview_layers(
        &mut self,
        ctx: &egui::Context,
    ) -> (
        Option<egui::TextureHandle>,
        Option<(egui::TextureHandle, f32)>,
        Option<String>,
    ) {
        let mut base = None;
        let mut overlay = None;
        let mut text = None;

        if !self
            .project
            .tracks
            .get(TRACK_VIDEO)
            .map(|t| !t.hidden)
            .unwrap_or(true)
        {
            // track hình ảnh đang bị ẩn
        } else if let Some(clip) = self.project.clip_at(TRACK_VIDEO, self.playhead).cloned() {
            let local = clip.in_pt + (self.playhead - clip.start);
            base = self.texture_for_clip(ctx, &clip, local);
            if !clip.text.trim().is_empty() {
                text = Some(clip.text.clone());
            }
        }

        if self
            .project
            .tracks
            .get(TRACK_OVERLAY)
            .map(|t| !t.hidden)
            .unwrap_or(true)
        {
            if let Some(clip) = self.project.clip_at(TRACK_OVERLAY, self.playhead).cloned() {
                let local = clip.in_pt + (self.playhead - clip.start);
                if let Some(t) = self.texture_for_clip(ctx, &clip, local) {
                    overlay = Some((t, clip.opacity));
                }
                if !clip.text.trim().is_empty() {
                    text = Some(clip.text.clone());
                }
            }
        }

        (base, overlay, text)
    }

    fn texture_for_clip(
        &mut self,
        ctx: &egui::Context,
        clip: &Clip,
        local: f64,
    ) -> Option<egui::TextureHandle> {
        let src = self.project.source(clip.source)?.clone();
        match src.kind {
            SourceKind::Image => self.ensure_image(ctx, &src),
            SourceKind::Gif => self.ensure_gif_frame(ctx, &src, local),
            SourceKind::Video => self.ensure_video_frame(&src, local),
            SourceKind::Audio => None,
        }
    }

    fn ensure_image(&mut self, ctx: &egui::Context, src: &Source) -> Option<egui::TextureHandle> {
        if let Some(t) = self.images.get(&src.id) {
            return Some(t.clone());
        }
        let img = image::open(&src.path).ok()?;
        let img = if img.width() > 1280 {
            img.thumbnail(1280, 1280)
        } else {
            img
        };
        let tex = ctx.load_texture(
            format!("img:{}", src.id),
            to_color_image(&img),
            egui::TextureOptions::LINEAR,
        );
        self.images.insert(src.id, tex.clone());
        Some(tex)
    }

    fn ensure_gif_frame(
        &mut self,
        ctx: &egui::Context,
        src: &Source,
        local: f64,
    ) -> Option<egui::TextureHandle> {
        if !self.gifs.contains_key(&src.id) {
            let anim = decode_gif(ctx, Path::new(&src.path))?;
            self.gifs.insert(src.id, anim);
            self.gif_order.push_back(src.id);
            while self.gif_order.len() > 2 {
                if let Some(old) = self.gif_order.pop_front() {
                    self.gifs.remove(&old);
                }
            }
        }
        let anim = self.gifs.get(&src.id)?;
        if anim.frames.is_empty() {
            return None;
        }
        let total = anim.total.max(0.01);
        let mut t = (local as f32) % total;
        if t < 0.0 {
            t += total;
        }
        let mut idx = 0usize;
        let mut acc = 0.0f32;
        for (i, d) in anim.delays.iter().enumerate() {
            acc += *d;
            if t < acc {
                idx = i;
                break;
            }
            idx = i;
        }
        anim.frames.get(idx).cloned()
    }

    fn ensure_video_frame(&mut self, src: &Source, local: f64) -> Option<egui::TextureHandle> {
        let q = (local.max(0.0) * 8.0).floor() / 8.0;
        let key = format!("{}:{:.3}", src.id, q);
        if let Some(t) = self.frames.get(&key) {
            return Some(t.clone());
        }
        if self.loader.in_flight.is_none() {
            self.loader.request(FrameReq {
                key: key.clone(),
                path: src.path.clone(),
                time: q,
                width: 1280,
            });
        }
        let prefix = format!("{}:", src.id);
        self.frame_order
            .iter()
            .rev()
            .find(|k| k.starts_with(&prefix))
            .and_then(|k| self.frames.get(k))
            .cloned()
    }

    fn poll_loader(&mut self, ctx: &egui::Context) {
        if let Some(res) = self.loader.poll() {
            if let Ok(bytes) = res.data {
                if let Ok(img) = image::load_from_memory(&bytes) {
                    let tex = ctx.load_texture(
                        format!("frame:{}", res.key),
                        to_color_image(&img),
                        egui::TextureOptions::LINEAR,
                    );
                    self.frames.insert(res.key.clone(), tex);
                    self.frame_order.push_back(res.key);
                    while self.frame_order.len() > 48 {
                        if let Some(old) = self.frame_order.pop_front() {
                            self.frames.remove(&old);
                        }
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // Dự án
    // -----------------------------------------------------------------

    fn save_project(&mut self, ask: bool) {
        if !ask {
            if let Some(path) = self.project_path.clone() {
                self.write_project(&path);
                return;
            }
        }
        self.picker_purpose = PickPurpose::SaveProject;
        let start = self
            .project_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        let name = if self.project.name.is_empty() {
            "du-an-moi.vieedit".to_string()
        } else {
            format!("{}.vieedit", self.project.name)
        };
        self.picker
            .open_save("Lưu dự án", &start, &name, "vieedit");
    }

    fn write_project(&mut self, path: &Path) {
        let json = serde_json::to_string_pretty(&self.project).unwrap_or_default();
        if std::fs::write(path, json).is_ok() {
            self.project_path = Some(path.to_path_buf());
            let msg = self.t().msg_saved.to_string();
            self.pending_toast = Some((msg, true));
        } else {
            let msg = self.t().msg_save_failed.to_string();
            self.pending_toast = Some((msg, false));
        }
    }

    fn load_project(&mut self, path: &Path) {
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<Project>(&text) {
                Ok(p) => {
                    self.checkpoint();
                    self.project = p;
                    self.project_path = Some(path.to_path_buf());
                    self.selected = None;
                    self.playhead = 0.0;
                    self.images.clear();
                    self.gifs.clear();
                    self.gif_order.clear();
                    self.frames.clear();
                    self.frame_order.clear();
                    let msg = self.t().msg_loaded.to_string();
                    self.pending_toast = Some((msg, true));
                }
                Err(e) => {
                    self.pending_toast = Some((format!("JSON: {}", e), false));
                }
            },
            Err(e) => {
                self.pending_toast = Some((e.to_string(), false));
            }
        }
    }

    // -----------------------------------------------------------------
    // Xuất video
    // -----------------------------------------------------------------

    fn open_export(&mut self, ctx: &egui::Context) {
        if self.project.duration() <= 0.01 {
            let msg = self.t().msg_nothing_to_export.to_string();
            self.toast_now(ctx, msg, false);
            return;
        }
        self.show_export = true;
        if self.export_path.is_empty() {
            let dir = self
                .project_path
                .as_ref()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."));
            let name = if self.project.name.is_empty() {
                "viedit-xuat.mp4".to_string()
            } else {
                format!("{}.mp4", self.project.name)
            };
            self.export_path = dir.join(name).to_string_lossy().to_string();
        }
    }

    fn export_windows(&mut self, ctx: &egui::Context) {
        let tr = self.t();

        // cập nhật tiến trình trước khi vẽ
        let mut logs: Option<String> = None;
        if let Some(job) = &mut self.export_job {
            while let Ok(msg) = job.rx.try_recv() {
                match msg {
                    ExportMsg::Progress(p) => job.progress = p,
                    ExportMsg::Log(s) => logs = Some(s),
                    ExportMsg::Done(res) => {
                        job.finished = true;
                        job.ok = res.is_ok();
                        if job.ok {
                            job.progress = 1.0;
                        }
                    }
                }
            }
            if !job.finished {
                ctx.request_repaint_after(std::time::Duration::from_millis(120));
            }
        }
        if let Some(s) = logs {
            self.export_log = s;
        }

        if self.show_export {
            let mut open = true;
            egui::Window::new(tr.dlg_export)
                .collapsible(false)
                .resizable(false)
                .default_width(470.0)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new(tr.lbl_output).size(11.0).color(theme::DIM));
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.export_path)
                                .desired_width(340.0),
                        );
                        if ui.add(theme::ghost_button(tr.btn_choose)).clicked() {
                            self.picker_purpose = PickPurpose::ExportOutput;
                            let start = PathBuf::from(&self.export_path);
                            self.picker.open_save(tr.dlg_export, &start, "viedit-xuat.mp4", "mp4");
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(RichText::new(tr.lbl_quality).size(11.0).color(theme::DIM));
                    ui.horizontal(|ui| {
                        for (q, label) in [
                            (Quality::Draft, "Nháp"),
                            (Quality::Standard, "Tiêu chuẩn"),
                            (Quality::High, "Cao"),
                        ] {
                            if ui.selectable_label(self.export_quality == q, label).clicked() {
                                self.export_quality = q;
                            }
                        }
                    });

                    ui.add_space(10.0);
                    if let Some(job) = &self.export_job {
                        ui.add(
                            egui::ProgressBar::new(job.progress)
                                .show_percentage()
                                .desired_width(430.0),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} · {:.1}s / {:.1}s",
                                tr.msg_exporting,
                                job.progress as f64 * job.total,
                                job.total
                            ))
                            .size(10.0)
                            .color(theme::DIM),
                        );
                        if job.finished {
                            if job.ok {
                                ui.colored_label(theme::GREEN, tr.msg_export_done);
                            } else {
                                ui.colored_label(theme::ACCENT, tr.msg_export_failed);
                            }
                        }
                    }
                    if !self.export_log.is_empty() {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(truncate(&self.export_log, 160))
                                .size(8.5)
                                .color(theme::FAINT),
                        );
                    }

                    ui.add_space(12.0);
                    let busy = self
                        .export_job
                        .as_ref()
                        .map(|j| !j.finished)
                        .unwrap_or(false);
                    ui.horizontal(|ui| {
                        if ui.add(theme::ghost_button(tr.btn_close)).clicked() {
                            self.show_export = false;
                            if !busy {
                                self.export_job = None;
                                self.export_log.clear();
                            }
                        }
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add_enabled(
                                        !busy && self.ffmpeg_ok,
                                        theme::accent_button(&format!("⬇  {}", tr.btn_export)),
                                    )
                                    .clicked()
                                {
                                    let project = self.project.clone();
                                    let path = PathBuf::from(self.export_path.clone());
                                    if let Some(parent) = path.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    match media::start_export(
                                        &project,
                                        &path,
                                        self.export_quality,
                                    ) {
                                        Ok(job) => {
                                            self.export_job = Some(job);
                                            self.export_log.clear();
                                        }
                                        Err(e) => {
                                            self.export_log = e;
                                        }
                                    }
                                }
                            },
                        );
                    });
                });
            if !open {
                self.show_export = false;
            }
        }
    }

    fn about_windows(&mut self, ctx: &egui::Context) {
        let tr = self.t();
        if self.show_about {
            let mut open = true;
            egui::Window::new(tr.dlg_about)
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("VieEdit")
                                .size(24.0)
                                .strong()
                                .color(theme::ACCENT),
                        );
                        ui.label(RichText::new("v0.1.0").size(10.0).color(theme::FAINT));
                        ui.add_space(8.0);
                    });
                    ui.label(RichText::new(tr.about_body).size(11.5).color(theme::DIM));
                    ui.add_space(8.0);
                    ui.label(RichText::new(&self.ffmpeg_line).size(9.0).color(theme::FAINT));
                });
            if !open {
                self.show_about = false;
            }
        }

        if self.show_shortcuts {
            let mut open = true;
            egui::Window::new(tr.menu_shortcuts)
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new(tr.shortcuts_body).size(11.5).color(theme::DIM));
                });
            if !open {
                self.show_shortcuts = false;
            }
        }
    }

    // -----------------------------------------------------------------
    // Phím tắt
    // -----------------------------------------------------------------

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if ctx.wants_keyboard_input() {
            return;
        }
        let (space, s_key, del, ctrl_s, ctrl_e, ctrl_z, ctrl_y, left, right, home, end) =
            ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Space),
                    i.key_pressed(egui::Key::S) && !i.modifiers.ctrl,
                    i.key_pressed(egui::Key::Delete),
                    i.key_pressed(egui::Key::S) && i.modifiers.ctrl,
                    i.key_pressed(egui::Key::E) && i.modifiers.ctrl,
                    i.key_pressed(egui::Key::Z) && i.modifiers.ctrl,
                    i.key_pressed(egui::Key::Y) && i.modifiers.ctrl,
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::ArrowRight),
                    i.key_pressed(egui::Key::Home),
                    i.key_pressed(egui::Key::End),
                )
            });

        let step = 1.0 / self.project.fps.max(1.0);
        if space {
            self.playing = !self.playing;
        }
        if s_key {
            self.split_at_playhead();
        }
        if del {
            self.delete_selected();
        }
        if ctrl_s {
            self.save_project(false);
        }
        if ctrl_e {
            self.open_export(ctx);
        }
        if ctrl_z {
            self.do_undo();
        }
        if ctrl_y {
            self.do_redo();
        }
        if left {
            self.playhead = (self.playhead - step).max(0.0);
        }
        if right {
            self.playhead += step;
        }
        if home {
            self.playhead = 0.0;
        }
        if end {
            self.playhead = self.project.duration();
        }
    }

    fn handle_picker(&mut self, ctx: &egui::Context) {
        if let Some(path) = self.picker.ui(ctx) {
            match self.picker_purpose {
                PickPurpose::ImportMedia => self.import_path(&path),
                PickPurpose::OpenProject => self.load_project(&path),
                PickPurpose::SaveProject => {
                    let p = if path.extension().is_none() {
                        path.with_extension("vieedit")
                    } else {
                        path
                    };
                    self.write_project(&p);
                }
                PickPurpose::ExportOutput => {
                    self.export_path = path.to_string_lossy().to_string();
                }
                PickPurpose::LibraryFolder => {
                    self.library_root = path;
                    self.reload_library();
                    self.persist_settings();
                    let msg = format!(
                        "Đã nạp {} tài nguyên từ thư mục mới.",
                        self.assets.len()
                    );
                    self.toast_now(ctx, msg, true);
                }
            }
        }
    }

    fn import_path(&mut self, path: &Path) {
        if path.is_dir() {
            let mut count = 0;
            if let Ok(rd) = std::fs::read_dir(path) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_file() && self.add_source_from_path(&p).is_some() {
                        count += 1;
                    }
                }
            }
            if count > 0 {
                self.checkpoint();
                self.tab = Tab::Media;
                self.pending_toast = Some((format!("Đã nhập {} tệp.", count), true));
            }
            return;
        }
        if let Some(id) = self.add_source_from_path(path) {
            self.checkpoint();
            self.tab = Tab::Media;
            let at = self.playhead;
            if let Some(cid) = self.add_clip_from_source(id, at) {
                self.selected = Some(cid);
            }
        } else {
            let msg = self.t().msg_unsupported.to_string();
            self.pending_toast = Some((msg, false));
        }
    }
}

impl eframe::App for VieEdit {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.playing {
            let dt = ctx.input(|i| i.stable_dt) as f64;
            self.playhead += dt;
            let end = self.project.duration();
            if self.playhead >= end {
                self.playhead = end;
                self.playing = false;
            }
            ctx.request_repaint();
        }

        self.poll_loader(ctx);
        self.handle_keys(ctx);

        self.top_bar(ctx);
        self.timeline_panel(ctx);
        self.library_panel(ctx);
        self.properties_panel(ctx);
        self.preview_panel(ctx);

        self.export_windows(ctx);
        self.about_windows(ctx);
        self.handle_picker(ctx);

        if let Some((text, ok)) = self.pending_toast.take() {
            self.toast_now(ctx, text, ok);
        }

        let now = ctx.input(|i| i.time);
        let expired = self
            .toast
            .as_ref()
            .map(|t| now >= t.until)
            .unwrap_or(false);
        if expired {
            self.toast = None;
        }
        if let Some(t) = &self.toast {
            let text = t.text.clone();
            let ok = t.ok;
            let alpha = ((t.until - now) as f32 / 0.6).clamp(0.0, 1.0);
            egui::Area::new(egui::Id::new("toast"))
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -18.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(if ok {
                            Color32::from_rgb(24, 60, 48)
                        } else {
                            Color32::from_rgb(64, 28, 28)
                        })
                        .rounding(egui::Rounding::same(8.0))
                        .inner_margin(egui::Margin::symmetric(14.0, 8.0))
                        .stroke(egui::Stroke::new(                             1.0_f32,
                            if ok { theme::GREEN } else { theme::ACCENT },
                        ))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(&text)
                                    .size(11.5)
                                    .color(Color32::WHITE.gamma_multiply(alpha)),
                            );
                        });
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
    }
}

// ---------------------------------------------------------------------------
// Tiện ích ảnh
// ---------------------------------------------------------------------------

fn to_color_image(img: &image::DynamicImage) -> egui::ColorImage {
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw())
}

fn make_thumb(ctx: &egui::Context, path: &Path, kind: AssetKind) -> Option<egui::TextureHandle> {
    let key = format!("thumb:{}", path.to_string_lossy());
    match kind {
        AssetKind::Gif => {
            use image::AnimationDecoder;
            let file = std::fs::File::open(path).ok()?;
            let dec = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(file)).ok()?;
            let frame = dec.into_frames().next()?.ok()?;
            let img = image::DynamicImage::ImageRgba8(frame.into_buffer()).thumbnail(256, 256);
            Some(ctx.load_texture(key, to_color_image(&img), egui::TextureOptions::LINEAR))
        }
        AssetKind::Image => {
            let img = image::open(path).ok()?.thumbnail(256, 256);
            Some(ctx.load_texture(key, to_color_image(&img), egui::TextureOptions::LINEAR))
        }
        AssetKind::Video => {
            let bytes = media::extract_frame_png(path.to_string_lossy().as_ref(), 0.5, 256).ok()?;
            let img = image::load_from_memory(&bytes).ok()?;
            Some(ctx.load_texture(key, to_color_image(&img), egui::TextureOptions::LINEAR))
        }
        AssetKind::Audio => None,
    }
}

fn decode_gif(ctx: &egui::Context, path: &Path) -> Option<GifAnim> {
    use image::AnimationDecoder;
    let file = std::fs::File::open(path).ok()?;
    let dec = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(file)).ok()?;
    let frames: Vec<image::Frame> = dec
        .into_frames()
        .take(60)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    let mut textures = Vec::new();
    let mut delays = Vec::new();
    let mut total = 0.0f32;

    for f in frames {
        let (num, den) = f.delay().numer_denom_ms();
        let mut d = if den > 0 {
            num as f32 / den as f32 / 1000.0
        } else {
            0.05
        };
        if d <= 0.001 {
            d = 0.05;
        }
        total += d;
        delays.push(d);

        let img = image::DynamicImage::ImageRgba8(f.into_buffer());
        let small = if img.width() > 640 {
            img.thumbnail(640, 640)
        } else {
            img
        };
        textures.push(ctx.load_texture(
            format!("gif:{}:{}", path.to_string_lossy(), delays.len()),
            to_color_image(&small),
            egui::TextureOptions::LINEAR,
        ));
    }

    Some(GifAnim {
        frames: textures,
        delays,
        total: total.max(0.05),
    })
}

fn paint_tex_into(
    painter: &egui::Painter,
    area: egui::Rect,
    tex: &egui::TextureHandle,
    opacity: f32,
) {
    let size = tex.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let scale = (area.width() / size.x).min(area.height() / size.y);
    let draw = egui::vec2(size.x * scale, size.y * scale);
    let pos = area.center() - draw * 0.5;
    let mut mesh = egui::Mesh::with_texture(tex.id());
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    mesh.add_rect_with_uv(
        egui::Rect::from_min_size(pos, draw),
        uv,
        Color32::WHITE.gamma_multiply(opacity.clamp(0.0, 1.0)),
    );
    painter.add(egui::Shape::mesh(mesh));
}

fn draw_texture_fit(
    painter: &egui::Painter,
    rect: egui::Rect,
    tex: &egui::TextureHandle,
    opacity: f32,
) {
    paint_tex_into(painter, rect, tex, opacity);
}

fn truncate(text: &str, max: usize) -> String {
    let clean: String = text.replace('\n', " ");
    if clean.chars().count() <= max {
        clean
    } else {
        let s: String = clean.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", s)
    }
}

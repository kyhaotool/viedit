//! Widget dòng thời gian: thước thời gian, các track và các đoạn phim.

use egui::{Color32, FontId, Id, Rect, Sense, Stroke};

use crate::i18n::Tr;
use crate::model::{fmt_time, Clip, Project, TrackKind};
use crate::theme;

const HEADER_W: f32 = 112.0;
const RULER_H: f32 = 26.0;
const TRACK_H: f32 = 56.0;
const TRACK_GAP: f32 = 5.0;
const MIN_DUR: f64 = 0.1;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Move,
    Left,
    Right,
}

struct Drag {
    clip: u64,
    kind: DragKind,
    x0: f32,
    orig_start: f64,
    orig_in: f64,
    orig_dur: f64,
}

pub struct TimelineState {
    pub zoom: f32,
    pub scroll: f64,
    drag: Option<Drag>,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            zoom: 90.0,
            scroll: 0.0,
            drag: None,
        }
    }
}

pub struct Outcome {
    pub changed: bool,
    pub undo_checkpoint: bool,
    pub snapshot: Option<Project>,
}

impl TimelineState {
    fn x_of(&self, content_left: f32, t: f64) -> f32 {
        content_left + ((t - self.scroll) * self.zoom as f64) as f32
    }
    fn t_of(&self, content_left: f32, x: f32) -> f64 {
        self.scroll + (x - content_left) as f64 / self.zoom as f64
    }
}

fn track_color(kind: TrackKind) -> Color32 {
    match kind {
        TrackKind::Video => theme::TRACK_VIDEO,
        TrackKind::Overlay => theme::TRACK_OVERLAY,
        TrackKind::Audio => theme::TRACK_AUDIO,
    }
}

pub fn show(
    ui: &mut egui::Ui,
    project: &mut Project,
    state: &mut TimelineState,
    selected: &mut Option<u64>,
    playhead: &mut f64,
    tr: &Tr,
) -> Outcome {
    let mut changed = false;
    let mut undo_checkpoint = false;
    let mut snapshot: Option<Project> = None;

    let avail = ui.available_rect_before_wrap();
    let (rect, _big) = ui.allocate_exact_size(avail.size(), Sense::hover());

    let content_left = rect.left() + HEADER_W;
    let content_right = rect.right();
    let ruler_rect = Rect::from_min_max(
        egui::pos2(content_left, rect.top()),
        egui::pos2(content_right, rect.top() + RULER_H),
    );

    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, egui::Rounding::same(0.0), theme::PANEL);

    // --- thước thời gian ---
    painter.rect_filled(
        Rect::from_min_max(rect.min, egui::pos2(content_right, rect.top() + RULER_H)),
        egui::Rounding::same(0.0),
        theme::PANEL_2,
    );

    let step = pick_step(state.zoom as f64);
    let start_t = state.scroll.max(0.0);
    let end_t = state.scroll + (content_right - content_left) as f64 / state.zoom as f64;

    let mut t = (start_t / step).floor() * step;
    while t <= end_t + step {
        let x = state.x_of(content_left, t);
        if x >= content_left - 2.0 {
            painter.line_segment(
                [egui::pos2(x, rect.top() + 4.0), egui::pos2(x, rect.bottom())],
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 12)),
            );
            painter.text(
                egui::pos2(x + 4.0, rect.top() + 7.0),
                egui::Align2::LEFT_TOP,
                short_time(t),
                FontId::proportional(10.0),
                theme::FAINT,
            );
        }
        t += step;
    }

    // --- header các track ---
    let mut y = rect.top() + RULER_H + TRACK_GAP;
    let mut row_rects: Vec<Rect> = Vec::new();
    for i in 0..project.tracks.len() {
        let row = Rect::from_min_max(
            egui::pos2(content_left, y),
            egui::pos2(content_right, y + TRACK_H),
        );
        row_rects.push(row);

        let head = Rect::from_min_max(
            egui::pos2(rect.left(), y),
            egui::pos2(content_left, y + TRACK_H),
        );
        painter.rect_filled(head, egui::Rounding::same(6.0), theme::PANEL_2);

        let kind = project.tracks[i].kind;
        let color = track_color(kind);
        painter.rect_filled(
            Rect::from_min_max(head.min, egui::pos2(head.left() + 3.0, head.bottom())),
            egui::Rounding::same(2.0),
            color,
        );

        let name = match kind {
            TrackKind::Video => tr.track_video,
            TrackKind::Overlay => tr.track_overlay,
            TrackKind::Audio => tr.track_audio,
        };
        painter.text(
            egui::pos2(head.left() + 12.0, head.top() + 10.0),
            egui::Align2::LEFT_TOP,
            name,
            FontId::proportional(12.0),
            theme::TEXT,
        );

        // nút tắt tiếng / ẩn
        let m_rect = Rect::from_min_size(
            egui::pos2(head.left() + 11.0, head.bottom() - 22.0),
            egui::vec2(24.0, 16.0),
        );
        let h_rect = Rect::from_min_size(
            egui::pos2(head.left() + 39.0, head.bottom() - 22.0),
            egui::vec2(24.0, 16.0),
        );

        let muted = project.tracks[i].muted;
        let hidden = project.tracks[i].hidden;
        let m = ui.put(
            m_rect,
            egui::Button::new(
                egui::RichText::new("M").size(9.5).color(if muted {
                    theme::ACCENT
                } else {
                    theme::FAINT
                }),
            )
            .fill(if muted { theme::ACCENT_DIM } else { theme::PANEL_3 })
            .rounding(egui::Rounding::same(4.0)),
        );
        if m.clicked() {
            project.tracks[i].muted = !muted;
            changed = true;
        }
        let h = ui.put(
            h_rect,
            egui::Button::new(
                egui::RichText::new("👁").size(9.0).color(if hidden {
                    theme::ACCENT
                } else {
                    theme::FAINT
                }),
            )
            .fill(if hidden { theme::ACCENT_DIM } else { theme::PANEL_3 })
            .rounding(egui::Rounding::same(4.0)),
        );
        if h.clicked() {
            project.tracks[i].hidden = !hidden;
            changed = true;
        }

        painter.rect_filled(row, egui::Rounding::same(6.0), theme::PANEL_2.gamma_multiply(0.55));

        y += TRACK_H + TRACK_GAP;
    }

    // --- các đoạn phim ---
    let mut hit_clip = false;
    let pointer = ui.ctx().pointer_interact_pos();

    let clip_ids: Vec<u64> = project.clips.iter().map(|c| c.id).collect();
    for id in clip_ids {
        let Some(clip) = project.clip(id) else { continue };
        if clip.track >= row_rects.len() {
            continue;
        }
        let row = row_rects[clip.track];
        let x0 = state.x_of(content_left, clip.start);
        let x1 = state.x_of(content_left, clip.end());
        if x1 < content_left || x0 > content_right {
            continue;
        }
        let clip_rect = Rect::from_min_max(
            egui::pos2(x0.max(content_left), row.top() + 4.0),
            egui::pos2(x1.min(content_right), row.bottom() - 4.0),
        );
        if clip_rect.width() < 1.0 {
            continue;
        }

        let kind = project.tracks[clip.track].kind;
        let base = track_color(kind);
        let is_sel = *selected == Some(id);

        let resp = ui.interact(
            clip_rect,
            Id::new(("clip", id)),
            Sense::click_and_drag(),
        );

        let fill = if is_sel {
            base.linear_multiply(0.95)
        } else {
            base.linear_multiply(0.72)
        };
        painter.rect_filled(clip_rect, egui::Rounding::same(5.0), fill);

        // vạch trang trí
        painter.rect_filled(
            Rect::from_min_max(
                clip_rect.min,
                egui::pos2(clip_rect.left() + 3.0, clip_rect.bottom()),
            ),
            egui::Rounding::same(2.0),
            base.linear_multiply(1.35),
        );

        let name = project
            .source(clip.source)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "?".to_string());
        painter.text(
            egui::pos2(clip_rect.left() + 9.0, clip_rect.top() + 7.0),
            egui::Align2::LEFT_TOP,
            truncate(&name, 26),
            FontId::proportional(11.0),
            Color32::from_rgb(240, 244, 250),
        );
        painter.text(
            egui::pos2(clip_rect.left() + 9.0, clip_rect.bottom() - 15.0),
            egui::Align2::LEFT_TOP,
            fmt_time(clip.dur),
            FontId::proportional(9.5),
            Color32::from_rgba_unmultiplied(255, 255, 255, 150),
        );

        if is_sel {
            painter.rect_stroke(
                clip_rect,
                egui::Rounding::same(5.0),
                Stroke::new(2.0_f32, theme::ACCENT),
            );
            let hw = 4.0;
            for x in [clip_rect.left(), clip_rect.right() - hw] {
                let handle = Rect::from_min_size(
                    egui::pos2(x, clip_rect.top()),
                    egui::vec2(hw, clip_rect.height()),
                );
                painter.rect_filled(handle, egui::Rounding::same(2.0), theme::ACCENT);
            }
        }

        if resp.clicked() || resp.drag_started() {
            *selected = Some(id);
            hit_clip = true;
        }

        if resp.drag_started() {
            if let Some(pos) = pointer {
                let kind = if (pos.x - clip_rect.left()).abs() < 7.0 {
                    DragKind::Left
                } else if (pos.x - clip_rect.right()).abs() < 7.0 {
                    DragKind::Right
                } else {
                    DragKind::Move
                };
                state.drag = Some(Drag {
                    clip: id,
                    kind,
                    x0: pos.x,
                    orig_start: clip.start,
                    orig_in: clip.in_pt,
                    orig_dur: clip.dur,
                });
                undo_checkpoint = true;
                snapshot = Some(project.clone());
            }
        }

        if resp.dragged() {
            if let (Some(drag), Some(pos)) = (state.drag.as_ref(), pointer) {
                if drag.clip == id {
                    let dt = (pos.x - drag.x0) as f64 / state.zoom as f64;
                    let src_dur = project
                        .source(clip.source)
                        .map(|s| s.duration)
                        .unwrap_or(600.0)
                        .max(0.2);

                    let mut start = drag.orig_start;
                    let mut in_pt = drag.orig_in;
                    let mut dur = drag.orig_dur;

                    match drag.kind {
                        DragKind::Move => {
                            start = (drag.orig_start + dt).max(0.0);
                            let snap = snap_target(project, id, clip.track, start, *playhead, state, content_left, pos.x);
                            start = snap.unwrap_or(start).max(0.0);
                        }
                        DragKind::Left => {
                            let mut new_start = (drag.orig_start + dt).max(0.0);
                            if let Some(s) =
                                snap_target(project, id, clip.track, new_start, *playhead, state, content_left, pos.x)
                            {
                                new_start = s.max(0.0);
                            }
                            let max_start = drag.orig_start + drag.orig_dur - MIN_DUR;
                            new_start = new_start.min(max_start);
                            let delta = new_start - drag.orig_start;
                            if drag.orig_in + delta >= 0.0 {
                                in_pt = drag.orig_in + delta;
                                dur = (drag.orig_dur - delta).max(MIN_DUR);
                                start = new_start;
                            }
                        }
                        DragKind::Right => {
                            let mut new_end = drag.orig_start + drag.orig_dur + dt;
                            if let Some(s) =
                                snap_target(project, id, clip.track, new_end, *playhead, state, content_left, pos.x)
                            {
                                new_end = s;
                            }
                            let max_end = drag.orig_start + (src_dur - drag.orig_in).max(MIN_DUR);
                            new_end = new_end.min(max_end);
                            dur = (new_end - drag.orig_start).max(MIN_DUR);
                        }
                    }

                    if let Some(c) = project.clip_mut(id) {
                        if (c.start - start).abs() > 1e-9
                            || (c.in_pt - in_pt).abs() > 1e-9
                            || (c.dur - dur).abs() > 1e-9
                        {
                            c.start = start;
                            c.in_pt = in_pt;
                            c.dur = dur;
                            changed = true;
                        }
                    }
                }
            }
        }

        if resp.drag_stopped() {
            state.drag = None;
        }

        // menu ngữ cảnh
        resp.context_menu(|ui| {
            if ui.button(tr.btn_split).clicked() {
                ui.close_menu();
            }
            if ui.button(tr.btn_delete).clicked() {
                ui.close_menu();
            }
        });
    }

    // nhấp vùng trống để di chuyển con trỏ
    if let Some(pos) = pointer {
        if ui.input(|i| i.pointer.primary_clicked())
            && pos.x > content_left
            && pos.y > rect.top() + RULER_H
            && !hit_clip
        {
            *playhead = state.t_of(content_left, pos.x).max(0.0);
        }
    }

    // kéo trên thước để tua
    let ruler_resp = ui.interact(ruler_rect, Id::new("ruler"), Sense::click_and_drag());
    if ruler_resp.dragged() || ruler_resp.clicked() {
        if let Some(pos) = pointer {
            *playhead = state.t_of(content_left, pos.x).max(0.0);
        }
    }

    // cuộn ngang + thu phóng
    if ui.rect_contains_pointer(rect) {
        let (scroll_delta, zoom_mod, pointer_x) = ui.input(|i| {
            (
                i.smooth_scroll_delta.x + if i.modifiers.shift { -i.smooth_scroll_delta.y } else { 0.0 },
                i.modifiers.ctrl,
                i.pointer.hover_pos().map(|p| p.x).unwrap_or(0.0),
            )
        });
        if zoom_mod {
            let before = state.t_of(content_left, pointer_x);
            state.zoom = (state.zoom * (1.0 + scroll_delta * 0.002)).clamp(12.0, 900.0);
            let after = state.t_of(content_left, pointer_x);
            state.scroll = (state.scroll + (before - after)).max(0.0);
        } else if scroll_delta.abs() > 0.5 {
            state.scroll = (state.scroll - scroll_delta as f64 / state.zoom as f64).max(0.0);
        }
    }

    // con trỏ thời gian
    let px = state.x_of(content_left, *playhead);
    if px >= content_left - 1.0 && px <= content_right {
        painter.line_segment(
            [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
            Stroke::new(1.5_f32, theme::ACCENT),
        );
        let head = vec![
            egui::pos2(px - 6.0, rect.top() + 1.0),
            egui::pos2(px + 6.0, rect.top() + 1.0),
            egui::pos2(px, rect.top() + 11.0),
        ];
        painter.add(egui::Shape::convex_polygon(
            head,
            theme::ACCENT,
            Stroke::NONE,
        ));
    }

    let _ = undo_checkpoint;

    Outcome {
        changed,
        undo_checkpoint: snapshot.is_some(),
        snapshot,
    }
}

fn snap_target(
    project: &Project,
    exclude: u64,
    track: usize,
    value: f64,
    playhead: f64,
    state: &TimelineState,
    content_left: f32,
    pointer_x: f32,
) -> Option<f64> {
    let threshold = 9.0 / state.zoom as f64;
    let pointer_t = state.t_of(content_left, pointer_x);
    let _ = pointer_t;

    let mut candidates = vec![0.0, playhead];
    for c in project.clips_on_track(track) {
        if c.id == exclude {
            continue;
        }
        candidates.push(c.start);
        candidates.push(c.end());
    }
    let mut best: Option<f64> = None;
    let mut best_d = threshold;
    for c in candidates {
        let d = (c - value).abs();
        if d < best_d {
            best_d = d;
            best = Some(c);
        }
    }
    best
}

fn pick_step(zoom: f64) -> f64 {
    let target = 90.0 / zoom;
    for s in [0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0] {
        if s >= target {
            return s;
        }
    }
    900.0
}

fn short_time(t: f64) -> String {
    let total = t.max(0.0);
    let m = (total / 60.0).floor() as i64;
    let s = total - (m as f64) * 60.0;
    if m > 0 {
        format!("{}:{:04.1}", m, s)
    } else {
        format!("{:.1}s", s)
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let s: String = text.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", s)
    }
}

/// Tiện ích: đoạn đang chọn.
pub fn selected_clip<'a>(project: &'a Project, selected: Option<u64>) -> Option<&'a Clip> {
    selected.and_then(|id| project.clip(id))
}

//! Mô hình dữ liệu của dự án.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Video,
    Image,
    Audio,
    Gif,
}

impl SourceKind {
    pub fn is_visual(self) -> bool {
        !matches!(self, SourceKind::Audio)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Source {
    pub id: u64,
    pub path: String,
    pub name: String,
    pub kind: SourceKind,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub has_audio: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackKind {
    Video,
    Overlay,
    Audio,
    Caption,
}

impl TrackKind {
    /// Làn phụ đề chỉ hiển thị chữ, không có tệp nguồn.
    pub fn is_caption(self) -> bool {
        matches!(self, TrackKind::Caption)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Track {
    pub kind: TrackKind,
    pub name: String,
    pub muted: bool,
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Clip {
    pub id: u64,
    pub source: u64,
    pub track: usize,
    /// Vị trí bắt đầu trên dòng thời gian (giây).
    pub start: f64,
    /// Điểm vào trong tệp nguồn (giây).
    pub in_pt: f64,
    /// Độ dài hiển thị trên dòng thời gian (giây).
    pub dur: f64,
    pub volume: f32,
    pub opacity: f32,
    /// Chữ hiển thị (dùng cho tiêu đề chèn và phụ đề).
    pub text: String,
    /// Chuỗi hiệu ứng FFmpeg áp dụng khi xuất (ví dụ `hue=s=0`).
    #[serde(default)]
    pub effect: String,
    /// Độ sáng, 0.0 là giữ nguyên (-1..1).
    #[serde(default)]
    pub brightness: f32,
    /// Độ tương phản, 1.0 là giữ nguyên.
    #[serde(default = "one")]
    pub contrast: f32,
    /// Độ bão hoà màu, 1.0 là giữ nguyên.
    #[serde(default = "one")]
    pub saturation: f32,
}

fn one() -> f32 {
    1.0
}

impl Default for Clip {
    fn default() -> Self {
        Self {
            id: 0,
            source: 0,
            track: 0,
            start: 0.0,
            in_pt: 0.0,
            dur: 1.0,
            volume: 1.0,
            opacity: 1.0,
            text: String::new(),
            effect: String::new(),
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
        }
    }
}

impl Clip {
    pub fn end(&self) -> f64 {
        self.start + self.dur
    }
    pub fn contains(&self, t: f64) -> bool {
        t >= self.start && t < self.end()
    }
    /// Có chỉnh màu khác mặc định hay không.
    pub fn has_color_adjust(&self) -> bool {
        self.brightness.abs() > 1e-3
            || (self.contrast - 1.0).abs() > 1e-3
            || (self.saturation - 1.0).abs() > 1e-3
    }
}

/// Kiểu chữ phụ đề dùng chung cho cả dự án.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CaptionStyle {
    /// Cỡ chữ tính theo tỉ lệ chiều cao khung hình.
    pub font_scale: f32,
    pub color: [u8; 3],
    pub outline: bool,
    /// Khoảng cách từ đáy khung hình (tỉ lệ chiều cao).
    pub bottom: f32,
}

impl Default for CaptionStyle {
    fn default() -> Self {
        Self {
            font_scale: 0.052,
            color: [255, 255, 255],
            outline: true,
            bottom: 0.10,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub sources: Vec<Source>,
    pub tracks: Vec<Track>,
    pub clips: Vec<Clip>,
    pub next_id: u64,
    #[serde(default)]
    pub caption: CaptionStyle,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: String::new(),
            width: 1920,
            height: 1080,
            fps: 30.0,
            sources: Vec::new(),
            tracks: vec![
                Track {
                    kind: TrackKind::Overlay,
                    name: String::new(),
                    muted: false,
                    hidden: false,
                },
                Track {
                    kind: TrackKind::Video,
                    name: String::new(),
                    muted: false,
                    hidden: false,
                },
                Track {
                    kind: TrackKind::Audio,
                    name: String::new(),
                    muted: false,
                    hidden: false,
                },
                Track {
                    kind: TrackKind::Caption,
                    name: String::new(),
                    muted: false,
                    hidden: false,
                },
            ],
            clips: Vec::new(),
            next_id: 1,
            caption: CaptionStyle::default(),
        }
    }
}

pub const TRACK_OVERLAY: usize = 0;
pub const TRACK_VIDEO: usize = 1;
pub const TRACK_AUDIO: usize = 2;
pub const TRACK_CAPTION: usize = 3;

/// Thứ tự hiển thị các làn từ trên xuống trong dòng thời gian.
pub const TRACK_DISPLAY_ORDER: [usize; 4] = [TRACK_CAPTION, TRACK_OVERLAY, TRACK_VIDEO, TRACK_AUDIO];

impl Project {
    pub fn new_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn duration(&self) -> f64 {
        self.clips
            .iter()
            .map(|c| c.end())
            .fold(0.0_f64, f64::max)
    }

    pub fn source(&self, id: u64) -> Option<&Source> {
        self.sources.iter().find(|s| s.id == id)
    }

    pub fn clip(&self, id: u64) -> Option<&Clip> {
        self.clips.iter().find(|c| c.id == id)
    }

    pub fn clip_mut(&mut self, id: u64) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|c| c.id == id)
    }

    pub fn clips_on_track(&self, track: usize) -> Vec<&Clip> {
        let mut list: Vec<&Clip> = self.clips.iter().filter(|c| c.track == track).collect();
        list.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
        list
    }

    /// Đoạn đang hiển thị tại thời điểm `t` trên một track.
    pub fn clip_at(&self, track: usize, t: f64) -> Option<&Clip> {
        self.clips
            .iter()
            .filter(|c| c.track == track && c.contains(t))
            .max_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Track hình ảnh đang hoạt động tại `t`: ưu tiên lớp phủ, rồi tới video.
    pub fn visual_at(&self, t: f64) -> Option<&Clip> {
        self.clip_at(TRACK_OVERLAY, t)
            .or_else(|| self.clip_at(TRACK_VIDEO, t))
    }

    /// Bổ sung làn phụ đề cho các dự án cũ (lưu từ bản 0.1).
    pub fn ensure_tracks(&mut self) {
        while self.tracks.len() <= TRACK_CAPTION {
            self.tracks.push(Track {
                kind: TrackKind::Caption,
                name: String::new(),
                muted: false,
                hidden: false,
            });
        }
        if !self.tracks[TRACK_CAPTION].kind.is_caption() {
            self.tracks[TRACK_CAPTION].kind = TrackKind::Caption;
        }
    }

    /// Xoá toàn bộ phụ đề.
    pub fn clear_captions(&mut self) {
        self.clips.retain(|c| c.track != TRACK_CAPTION);
    }

    pub fn caption_clips(&self) -> Vec<&Clip> {
        self.clips_on_track(TRACK_CAPTION)
    }

    /// Phụ đề đang hiển thị tại thời điểm `t`.
    pub fn caption_at(&self, t: f64) -> Option<&Clip> {
        self.clip_at(TRACK_CAPTION, t)
    }

    pub fn find_free_start(&self, track: usize, dur: f64, want: f64) -> f64 {
        let mut start = want.max(0.0);
        let mut moved = true;
        let mut guard = 0;
        while moved && guard < 200 {
            moved = false;
            guard += 1;
            for c in self.clips_on_track(track) {
                if start < c.end() && start + dur > c.start {
                    start = c.end();
                    moved = true;
                }
            }
        }
        start
    }
}

/// Định dạng thời gian dạng mm:ss.ff
pub fn fmt_time(t: f64) -> String {
    let t = t.max(0.0);
    let total = t.floor() as i64;
    let m = total / 60;
    let s = total % 60;
    let f = ((t - t.floor()) * 100.0).floor() as i64;
    format!("{:02}:{:02}.{:02}", m, s, f)
}

pub fn fmt_dur(t: f64) -> String {
    let t = t.max(0.0);
    let total = t.round() as i64;
    let m = total / 60;
    let s = total % 60;
    format!("{:02}:{:02}", m, s)
}

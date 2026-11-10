//! Lá»›p giao tiáº¿p vá»›i FFmpeg: Ä‘á»c thÃ´ng tin tá»‡p, trÃ­ch khung hÃ¬nh, xuáº¥t video.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;

use crate::model::{
    Clip, Project, Source, SourceKind, TRACK_AUDIO, TRACK_CAPTION, TRACK_OVERLAY, TRACK_VIDEO,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn find_tool(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    // 1) cÃ¹ng thÆ° má»¥c vá»›i chÆ°Æ¡ng trÃ¬nh
    for sub in ["", "ffmpeg/bin/", "bin/"] {
        let p = exe_dir().join(sub).join(&exe);
        if p.exists() {
            return Some(p);
        }
    }

    // 2) thá»­ gá»i trá»±c tiáº¿p (dá»±a vÃ o PATH)
    let mut cmd = Command::new(&exe);
    cmd.arg("-version").stdout(Stdio::null()).stderr(Stdio::null());
    hidden(&mut cmd);
    if let Ok(out) = cmd.output() {
        if out.status.success() {
            return Some(PathBuf::from(exe));
        }
    }
    None
}

pub fn ffmpeg() -> Option<&'static PathBuf> {
    static FFMPEG: OnceLock<Option<PathBuf>> = OnceLock::new();
    FFMPEG.get_or_init(|| find_tool("ffmpeg")).as_ref()
}

pub fn ffprobe() -> Option<&'static PathBuf> {
    static FFPROBE: OnceLock<Option<PathBuf>> = OnceLock::new();
    FFPROBE.get_or_init(|| find_tool("ffprobe")).as_ref()
}

pub fn available() -> bool {
    ffmpeg().is_some()
}

pub fn version_line() -> String {
    let Some(p) = ffmpeg() else {
        return "FFmpeg: khÃ´ng tÃ¬m tháº¥y".to_string();
    };
    let mut cmd = Command::new(p);
    cmd.arg("-version").stdout(Stdio::piped()).stderr(Stdio::null());
    hidden(&mut cmd);
    if let Ok(out) = cmd.output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(first) = text.lines().next() {
            return first.trim().to_string();
        }
    }
    "FFmpeg: Ä‘Ã£ tÃ¬m tháº¥y".to_string()
}

// ---------------------------------------------------------------------------
// Äá»c thÃ´ng tin tá»‡p
// ---------------------------------------------------------------------------

fn ext_of(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn probe(path: &Path) -> Option<Source> {
    let ext = ext_of(path);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("khÃ´ng tÃªn")
        .to_string();

    let image_like = matches!(
        ext.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "bmp"
    );
    let gif = ext == "gif";
    let audio_ext = matches!(ext.as_str(), "mp3" | "wav" | "m4a" | "aac" | "ogg" | "flac");

    if image_like || gif {
        let (w, h) = image::image_dimensions(path).unwrap_or((0, 0));
        return Some(Source {
            id: 0,
            path: path.to_string_lossy().to_string(),
            name,
            kind: if gif { SourceKind::Gif } else { SourceKind::Image },
            duration: if gif { 4.0 } else { 5.0 },
            width: w,
            height: h,
            fps: 30.0,
            has_audio: false,
        });
    }

    let mut duration = if audio_ext { 10.0 } else { 5.0 };
    let mut width = 0u32;
    let mut height = 0u32;
    let mut fps = 30.0f64;
    let mut has_audio = audio_ext;
    let mut is_video = !audio_ext;

    if let Some(probe_bin) = ffprobe() {
        let mut cmd = Command::new(probe_bin);
        cmd.args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
        hidden(&mut cmd);

        if let Ok(out) = cmd.output() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                if let Some(d) = json
                    .get("format")
                    .and_then(|f| f.get("duration"))
                    .and_then(|d| d.as_str())
                    .and_then(|d| d.parse::<f64>().ok())
                {
                    if d > 0.05 {
                        duration = d;
                    }
                }
                if let Some(streams) = json.get("streams").and_then(|s| s.as_array()) {
                    let mut found_video = false;
                    for s in streams {
                        match s.get("codec_type").and_then(|c| c.as_str()) {
                            Some("video") => {
                                found_video = true;
                                width = s
                                    .get("width")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0) as u32;
                                height = s
                                    .get("height")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0) as u32;
                                if let Some(rate) = s.get("r_frame_rate").and_then(|r| r.as_str()) {
                                    if let Some((a, b)) = rate.split_once('/') {
                                        let a = a.parse::<f64>().unwrap_or(0.0);
                                        let b = b.parse::<f64>().unwrap_or(1.0);
                                        if b > 0.0 && a > 0.0 {
                                            fps = a / b;
                                        }
                                    }
                                }
                            }
                            Some("audio") => has_audio = true,
                            _ => {}
                        }
                    }
                    is_video = found_video;
                }
            }
        }
    }

    Some(Source {
        id: 0,
        path: path.to_string_lossy().to_string(),
        name,
        kind: if is_video {
            SourceKind::Video
        } else {
            SourceKind::Audio
        },
        duration,
        width,
        height,
        fps,
        has_audio,
    })
}

// ---------------------------------------------------------------------------
// TrÃ­ch khung hÃ¬nh
// ---------------------------------------------------------------------------

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("viedit_frames");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// TrÃ­ch má»™t khung hÃ¬nh táº¡i thá»i Ä‘iá»ƒm `time`, tráº£ vá» bytes PNG.
pub fn extract_frame_png(path: &str, time: f64, max_width: u32) -> Result<Vec<u8>, String> {
    let ff = ffmpeg().ok_or_else(|| "KhÃ´ng tÃ¬m tháº¥y FFmpeg".to_string())?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let out = temp_dir().join(format!("f_{}_{}.png", std::process::id(), stamp));

    let mut cmd = Command::new(ff);
    cmd.arg("-v")
        .arg("error")
        .arg("-ss")
        .arg(format!("{:.3}", time.max(0.0)))
        .arg("-i")
        .arg(path)
        .arg("-frames:v")
        .arg("1")
        .arg("-vf")
        .arg(format!(
            "scale='min({},iw)':-2",
            max_width.max(160)
        ))
        .arg("-y")
        .arg(&out)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    hidden(&mut cmd);

    let result = cmd.output();
    let bytes = std::fs::read(&out);
    let _ = std::fs::remove_file(&out);

    match (result, bytes) {
        (Ok(o), Ok(b)) if o.status.success() && !b.is_empty() => Ok(b),
        (Ok(o), _) => Err(String::from_utf8_lossy(&o.stderr).to_string()),
        (Err(e), _) => Err(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Náº¡p khung hÃ¬nh cháº¡y ná»n
// ---------------------------------------------------------------------------

pub struct FrameReq {
    pub key: String,
    pub path: String,
    pub time: f64,
    pub width: u32,
}

pub struct FrameRes {
    pub key: String,
    pub data: Result<Vec<u8>, String>,
}

pub struct FrameLoader {
    req_tx: Sender<FrameReq>,
    res_rx: Receiver<FrameRes>,
    pub in_flight: Option<String>,
}

impl FrameLoader {
    pub fn new() -> Self {
        let (req_tx, req_rx) = channel::<FrameReq>();
        let (res_tx, res_rx) = channel::<FrameRes>();

        std::thread::spawn(move || {
            while let Ok(req) = req_rx.recv() {
                let data = extract_frame_png(&req.path, req.time, req.width);
                if res_tx
                    .send(FrameRes {
                        key: req.key,
                        data,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            req_tx,
            res_rx,
            in_flight: None,
        }
    }

    pub fn request(&mut self, req: FrameReq) {
        self.in_flight = Some(req.key.clone());
        let _ = self.req_tx.send(req);
    }

    pub fn poll(&mut self) -> Option<FrameRes> {
        if let Ok(res) = self.res_rx.try_recv() {
            self.in_flight = None;
            return Some(res);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Xuáº¥t video
// ---------------------------------------------------------------------------

pub enum ExportMsg {
    Progress(f32),
    Log(String),
    Done(Result<String, String>),
}

pub struct ExportJob {
    pub rx: Receiver<ExportMsg>,
    pub progress: f32,
    pub log: String,
    pub finished: bool,
    pub ok: bool,
    pub total: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    Draft,
    Standard,
    High,
}

impl Quality {
    pub fn crf(self) -> u32 {
        match self {
            Quality::Draft => 28,
            Quality::Standard => 22,
            Quality::High => 17,
        }
    }
    pub fn preset(self) -> &'static str {
        match self {
            Quality::Draft => "veryfast",
            Quality::Standard => "medium",
            Quality::High => "slow",
        }
    }
}


// ---------------------------------------------------------------------------
// Phụ đề (SRT)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Caption {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

fn parse_ts(s: &str) -> Option<f64> {
    let s = s.trim().replace(',', ".");
    let mut parts = s.split(':');
    let h: f64 = parts.next()?.trim().parse().ok()?;
    let m: f64 = parts.next()?.trim().parse().ok()?;
    let sec: f64 = parts.next()?.trim().parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec)
}

fn srt_ts(t: f64) -> String {
    let t = t.max(0.0);
    let ms = (t * 1000.0).round() as i64;
    let h = ms / 3_600_000;
    let m = (ms / 60_000) % 60;
    let s = (ms / 1000) % 60;
    let r = ms % 1000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, r)
}

/// Đọc nội dung tệp SRT thành danh sách phụ đề.
pub fn parse_srt(text: &str) -> Vec<Caption> {
    let text = text.trim_start_matches('\u{feff}');
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = Vec::new();
    for block in normalized.split("\n\n") {
        let mut start = None;
        let mut end = None;
        let mut body: Vec<String> = Vec::new();
        for line in block.lines() {
            let t = line.trim();
            if let Some((a, b)) = t.split_once("-->") {
                start = parse_ts(a);
                let b = b.split_whitespace().next().unwrap_or(b);
                end = parse_ts(b);
            } else if start.is_some() && end.is_some() && !t.is_empty() {
                body.push(t.to_string());
            }
        }
        if let (Some(s), Some(e)) = (start, end) {
            let cap_text = body.join(" ").trim().to_string();
            if !cap_text.is_empty() && e > s {
                out.push(Caption {
                    start: s,
                    end: e,
                    text: cap_text,
                });
            }
        }
    }
    out
}

/// Sinh nội dung SRT từ danh sách phụ đề.
pub fn format_srt(caps: &[Caption]) -> String {
    let mut out = String::new();
    for (i, c) in caps.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_ts(c.start),
            srt_ts(c.end),
            c.text.replace('\n', " ")
        ));
    }
    out
}

/// Phụ đề hiện có trong dự án, sắp theo thời gian.
pub fn captions_of(project: &Project) -> Vec<Caption> {
    let mut caps: Vec<Caption> = project
        .clips_on_track(TRACK_CAPTION)
        .iter()
        .filter(|c| !c.text.trim().is_empty())
        .map(|c| Caption {
            start: c.start,
            end: c.end(),
            text: c.text.trim().to_string(),
        })
        .collect();
    caps.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    caps
}

// ---------------------------------------------------------------------------
// Tuỳ chọn xuất
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct ExportOptions {
    pub quality: Quality,
    /// Chuẩn hoá âm lượng theo EBU R128 (mức -16 LUFS).
    pub loudnorm: bool,
    /// Ghi phụ đề trực tiếp vào khung hình.
    pub burn_captions: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            quality: Quality::Standard,
            loudnorm: false,
            burn_captions: true,
        }
    }
}

fn absolute(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|d| d.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

struct Seg {
    source_idx: usize,
    in_pt: f64,
    dur: f64,
    fx: String,
}

/// Chuỗi hiệu ứng (FFmpeg) của một đoạn phim: hiệu ứng chọn sẵn + chỉnh màu.
fn clip_fx(c: &Clip) -> String {
    let mut s = String::new();
    let effect = c.effect.trim().trim_matches(',');
    if !effect.is_empty() {
        s.push(',');
        s.push_str(effect);
    }
    if c.has_color_adjust() {
        s.push_str(&format!(
            ",eq=brightness={:.3}:contrast={:.3}:saturation={:.3}",
            c.brightness.clamp(-1.0, 1.0),
            c.contrast.clamp(0.0, 3.0),
            c.saturation.clamp(0.0, 3.0)
        ));
    }
    s
}

fn push_segments(
    p: &Project,
    t0: f64,
    t1: f64,
    out: &mut Vec<Seg>,
    input_of: &dyn Fn(u64) -> usize,
) {
    let hidden = p.tracks.get(TRACK_VIDEO).map(|t| t.hidden).unwrap_or(false);
    if !hidden {
        let mut vids: Vec<&Clip> = p
            .clips
            .iter()
            .filter(|c| c.track == TRACK_VIDEO)
            .collect();
        vids.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

        let mut cursor = t0;
        for c in vids {
            if c.end() <= t0 || c.start >= t1 {
                continue;
            }
            let s = c.start.max(t0);
            let e = c.end().min(t1);
            if s > cursor + 1e-4 {
                out.push(Seg {
                    source_idx: usize::MAX,
                    in_pt: 0.0,
                    dur: s - cursor,
                    fx: String::new(),
                });
            }
            out.push(Seg {
                source_idx: input_of(c.source),
                in_pt: c.in_pt + (s - c.start),
                dur: e - s,
                fx: clip_fx(c),
            });
            cursor = e;
        }
        if t1 > cursor + 1e-4 {
            out.push(Seg {
                source_idx: usize::MAX,
                in_pt: 0.0,
                dur: t1 - cursor,
                fx: String::new(),
            });
        }
    }
    if out.is_empty() {
        out.push(Seg {
            source_idx: usize::MAX,
            in_pt: 0.0,
            dur: t1.max(0.05),
            fx: String::new(),
        });
    }
}

struct InputSpec {
    path: String,
    pre: Vec<String>,
}

fn build_inputs(
    project: &Project,
    total: f64,
    fps: f64,
) -> (Vec<InputSpec>, Vec<u64>, std::collections::HashMap<u64, usize>) {
    let mut inputs: Vec<InputSpec> = Vec::new();
    let mut ids: Vec<u64> = Vec::new();
    let mut index: std::collections::HashMap<u64, usize> = Default::default();

    for s in &project.sources {
        if !project.clips.iter().any(|c| c.source == s.id) {
            continue;
        }
        let pre = match s.kind {
            SourceKind::Image => vec![
                "-loop".to_string(),
                "1".to_string(),
                "-framerate".to_string(),
                format!("{:.3}", fps.max(1.0)),
                "-t".to_string(),
                format!("{:.3}", total + 1.0),
            ],
            SourceKind::Gif => vec![
                "-stream_loop".to_string(),
                "-1".to_string(),
                "-t".to_string(),
                format!("{:.3}", total + 1.0),
            ],
            _ => Vec::new(),
        };
        index.insert(s.id, inputs.len());
        ids.push(s.id);
        inputs.push(InputSpec {
            path: absolute(Path::new(&s.path)).to_string_lossy().to_string(),
            pre,
        });
    }
    (inputs, ids, index)
}

fn source_has_audio(project: &Project, id: u64) -> bool {
    project
        .source(id)
        .map(|s| s.has_audio && s.kind == SourceKind::Video)
        .unwrap_or(false)
}

/// Trộn các đoạn nhạc/giọng nói trên làn Âm thanh vào chương trình tiếng.
fn audio_track_chain(
    project: &Project,
    input_index: &std::collections::HashMap<u64, usize>,
    filters: &mut Vec<String>,
) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    if project
        .tracks
        .get(TRACK_AUDIO)
        .map(|t| t.muted)
        .unwrap_or(false)
    {
        return labels;
    }
    for (i, c) in project.clips_on_track(TRACK_AUDIO).iter().enumerate() {
        let Some(&k) = input_index.get(&c.source) else {
            continue;
        };
        if !source_has_audio(project, c.source) && project.source(c.source).is_none() {
            continue;
        }
        let src_audio = project
            .source(c.source)
            .map(|s| s.has_audio)
            .unwrap_or(false);
        if !src_audio {
            continue;
        }
        let vol = c.volume.clamp(0.0, 4.0);
        let delay_ms = (c.start.max(0.0) * 1000.0).round() as i64;
        filters.push(format!(
            "[{k}:a]atrim=start={a:.3}:end={b:.3},asetpts=PTS-STARTPTS,\
             aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo,\
             volume={vol:.3},adelay={d}|{d}[mt{i}]",
            k = k,
            a = c.in_pt,
            b = c.in_pt + c.dur,
            vol = vol,
            d = delay_ms,
            i = i
        ));
        labels.push(format!("[mt{}]", i));
    }
    labels
}

/// Gộp chương trình tiếng gốc với làn âm thanh, trả về nhãn cuối cùng.
fn mix_audio(
    project: &Project,
    input_index: &std::collections::HashMap<u64, usize>,
    filters: &mut Vec<String>,
    base: &str,
    loudnorm: bool,
) -> String {
    let labels = audio_track_chain(project, input_index, filters);
    let mut label = base.to_string();
    if !labels.is_empty() {
        filters.push(format!(
            "[{}]{}amix=inputs={}:duration=first:dropout_transition=0:normalize=0,\
             alimiter=limit=0.95[mixout]",
            base,
            labels.concat(),
            labels.len() + 1
        ));
        label = "mixout".to_string();
    }
    if loudnorm {
        filters.push(format!(
            "[{}]loudnorm=I=-16:TP=-1.5:LRA=11[lnout]",
            label
        ));
        label = "lnout".to_string();
    }
    label
}

pub fn start_export(
    project: &Project,
    output: &Path,
    quality: Quality,
) -> Result<ExportJob, String> {
    start_export_with(
        project,
        output,
        ExportOptions {
            quality,
            ..ExportOptions::default()
        },
    )
}

pub fn start_export_with(
    project: &Project,
    output: &Path,
    opts: ExportOptions,
) -> Result<ExportJob, String> {
    if ffmpeg().is_none() {
        return Err("Không tìm thấy FFmpeg".into());
    }

    let total = project.duration();
    if total <= 0.01 {
        return Err("Dòng thời gian đang trống".into());
    }

    let w = project.width.max(2);
    let h = project.height.max(2);
    let fps = project.fps.max(1.0);

    let (inputs, _ids, input_index) = build_inputs(project, total, fps);
    if inputs.is_empty() {
        return Err("Dòng thời gian đang trống".into());
    }
    let idx_of = |id: u64| -> usize { *input_index.get(&id).unwrap_or(&0) };

    let mut segs: Vec<Seg> = Vec::new();
    push_segments(project, 0.0, total, &mut segs, &idx_of);

    let mut filters: Vec<String> = Vec::new();
    let mut chain = String::new();
    let mut n = 0usize;

    for seg in &segs {
        let d = seg.dur.max(0.02);
        if seg.source_idx == usize::MAX {
            filters.push(format!(
                "color=c=black:s={}x{}:r={}:d={:.3},setsar=1,format=yuv420p[v{}]",
                w, h, fps, d, n
            ));
            filters.push(format!(
                "anullsrc=r=48000:cl=stereo,atrim=duration={:.3},\
                 aformat=sample_fmts=fltp:channel_layouts=stereo[a{}]",
                d, n
            ));
        } else {
            let has_audio = _ids
                .get(seg.source_idx)
                .map(|id| source_has_audio(project, *id))
                .unwrap_or(false);

            filters.push(format!(
                "[{k}:v]trim=start={a:.3}:end={b:.3},setpts=PTS-STARTPTS,\
                 scale={w}:{h}:force_original_aspect_ratio=decrease,\
                 pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps}\
                 {fx},format=yuv420p[v{n}]",
                k = seg.source_idx,
                a = seg.in_pt,
                b = seg.in_pt + d,
                w = w,
                h = h,
                fps = fps,
                fx = seg.fx,
                n = n
            ));

            if has_audio {
                filters.push(format!(
                    "[{k}:a]atrim=start={a:.3}:end={b:.3},asetpts=PTS-STARTPTS,\
                     aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo[a{n}]",
                    k = seg.source_idx,
                    a = seg.in_pt,
                    b = seg.in_pt + d,
                    n = n
                ));
            } else {
                filters.push(format!(
                    "anullsrc=r=48000:cl=stereo,atrim=duration={:.3},\
                     aformat=sample_fmts=fltp:channel_layouts=stereo[a{}]",
                    d, n
                ));
            }
        }
        chain.push_str(&format!("[v{}][a{}]", n, n));
        n += 1;
    }

    filters.push(format!(
        "{}concat=n={}:v=1:a=1[basev][basea]",
        chain, n
    ));

    // Lớp phủ
    let mut overlays: Vec<&Clip> = project
        .clips
        .iter()
        .filter(|c| c.track == TRACK_OVERLAY)
        .collect();
    overlays.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));

    let overlays_hidden = project
        .tracks
        .get(TRACK_OVERLAY)
        .map(|t| t.hidden)
        .unwrap_or(false);

    let mut current_v = "basev".to_string();
    if !overlays_hidden {
        for (i, ov) in overlays.iter().enumerate() {
            let k = idx_of(ov.source);
            let opacity = ov.opacity.clamp(0.0, 1.0);
            filters.push(format!(
                "[{k}:v]trim=start={a:.3}:end={b:.3},setpts=PTS-STARTPTS+{s:.3}/TB,\
                 scale={w}:{h}:force_original_aspect_ratio=decrease{fx},format=rgba,\
                 colorchannelmixer=aa={op:.2}[ov{i}]",
                k = k,
                a = ov.in_pt,
                b = ov.in_pt + ov.dur,
                s = ov.start,
                w = w,
                h = h,
                fx = clip_fx(ov),
                op = opacity,
                i = i
            ));
            let next = format!("v{}", i + 1);
            filters.push(format!(
                "[{cur}][ov{i}]overlay=(W-w)/2:(H-h)/2:eof_action=pass[{next}]",
                cur = current_v,
                i = i,
                next = next
            ));
            current_v = next;
        }
    }

    // Phụ đề cháy sẵn
    let caps = captions_of(project);
    let captions_hidden = project
        .tracks
        .get(TRACK_CAPTION)
        .map(|t| t.hidden)
        .unwrap_or(false);
    let mut cleanup: Option<PathBuf> = None;
    if opts.burn_captions && !caps.is_empty() && !captions_hidden {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = temp_dir().join(format!("caps_{}_{}", std::process::id(), stamp));
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(
                dir.join("font.ttf"),
                include_bytes!("../assets/fonts/BeVietnamPro-Bold.ttf"),
            );
            let fontsize = ((h as f32) * project.caption.font_scale.clamp(0.015, 0.16))
                .round()
                .max(10.0) as u32;
            let margin = ((h as f32) * project.caption.bottom.clamp(0.0, 0.45)).round() as u32;
            let bw = if project.caption.outline {
                (h as f32 / 260.0).round().max(2.0) as u32
            } else {
                0
            };
            let [cr, cg, cb] = project.caption.color;
            let color = format!("0x{:02X}{:02X}{:02X}", cr, cg, cb);

            let mut ok = true;
            for (i, c) in caps.iter().enumerate() {
                let text = c.text.replace(['\n', '\r'], " ");
                if std::fs::write(dir.join(format!("c{}.txt", i)), text).is_err() {
                    ok = false;
                    break;
                }
            }
            if ok {
                for (i, c) in caps.iter().enumerate() {
                    let next = format!("vcap{}", i);
                    filters.push(format!(
                        "[{cur}]drawtext=fontfile=font.ttf:textfile=c{i}.txt:expansion=none:\
                         fontsize={fs}:fontcolor={col}:borderw={bw}:bordercolor=black@0.85:\
                         x=(w-text_w)/2:y=h-text_h-{m}:enable='between(t,{a:.3},{b:.3})'[{next}]",
                        cur = current_v,
                        i = i,
                        fs = fontsize,
                        col = color,
                        bw = bw,
                        m = margin,
                        a = c.start.max(0.0),
                        b = c.end.min(total),
                        next = next
                    ));
                    current_v = next;
                }
                cleanup = Some(dir);
            } else {
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }

    let audio_label = mix_audio(project, &input_index, &mut filters, "basea", opts.loudnorm);
    let filter_complex = filters.join(";");

    let out_path = absolute(output);
    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut cmd = Command::new(ffmpeg().unwrap());
    for input in &inputs {
        for a in &input.pre {
            cmd.arg(a);
        }
        cmd.arg("-i").arg(&input.path);
    }
    if let Some(dir) = &cleanup {
        cmd.current_dir(dir);
    }
    cmd.arg("-filter_complex")
        .arg(&filter_complex)
        .arg("-map")
        .arg(format!("[{}]", current_v))
        .arg("-map")
        .arg(format!("[{}]", audio_label))
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg(opts.quality.preset())
        .arg("-crf")
        .arg(opts.quality.crf().to_string())
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("192k")
        .arg("-movflags")
        .arg("+faststart")
        .arg("-progress")
        .arg("pipe:1")
        .arg("-nostats")
        .arg("-y")
        .arg(&out_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hidden(&mut cmd);

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx, rx) = channel::<ExportMsg>();

    if let Some(out) = stdout {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(out);
            for line in reader.lines().flatten() {
                if let Some(v) = line.strip_prefix("out_time_ms=") {
                    if let Ok(ms) = v.trim().parse::<f64>() {
                        let secs = ms / 1_000_000.0;
                        let p = (secs / total.max(0.01)).clamp(0.0, 1.0) as f32;
                        let _ = tx.send(ExportMsg::Progress(p));
                    }
                }
            }
        });
    }

    if let Some(err) = stderr {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(err);
            let mut tail: Vec<String> = Vec::new();
            for line in reader.lines().flatten() {
                tail.push(line);
                if tail.len() > 40 {
                    tail.remove(0);
                }
            }
            let _ = tx.send(ExportMsg::Log(tail.join("\n")));
        });
    }

    let tx_done = tx;
    std::thread::spawn(move || {
        let status = child.wait();
        let ok = status.map(|s| s.success()).unwrap_or(false);
        if let Some(dir) = cleanup {
            let _ = std::fs::remove_dir_all(dir);
        }
        let _ = tx_done.send(ExportMsg::Done(if ok {
            Ok("ok".into())
        } else {
            Err("ffmpeg trả về lỗi".into())
        }));
    });

    Ok(ExportJob {
        rx,
        progress: 0.0,
        log: String::new(),
        finished: false,
        ok: false,
        total,
    })
}

// ---------------------------------------------------------------------------
// Nhận dạng giọng nói (phụ đề tự động) qua filter whisper của FFmpeg
// ---------------------------------------------------------------------------

pub enum TranscribeMsg {
    Status(String),
    Done(Result<String, String>),
}

pub struct TranscribeJob {
    pub rx: Receiver<TranscribeMsg>,
    pub status: String,
    pub finished: bool,
    pub result: Option<Result<String, String>>,
}

/// Các thư mục có thể chứa mô hình ggml của Whisper.
pub fn model_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            dirs.push(d.join("models"));
            dirs.push(d.join("assets").join("models"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("assets").join("models"));
        dirs.push(cwd.join("models"));
    }
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("models"));
    dirs
}

/// Thư mục để tải mô hình mới về.
pub fn models_dir() -> PathBuf {
    for d in model_dirs() {
        if d.is_dir() {
            return d;
        }
    }
    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join("models");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// Tìm mô hình whisper đã có trên máy.
pub fn find_model() -> Option<PathBuf> {
    for dir in model_dirs() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if name.starts_with("ggml-") && name.ends_with(".bin") {
                return Some(p);
            }
        }
    }
    None
}

/// Tải mô hình bằng curl (có sẵn trên Windows 10+ và Fedora).
pub fn start_model_download(url: &str, dest: &Path) -> Receiver<Result<(), String>> {
    let (tx, rx) = channel::<Result<(), String>>();
    let url = url.to_string();
    let dest = dest.to_path_buf();
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::thread::spawn(move || {
        let mut cmd = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
        cmd.arg("-L")
            .arg("--fail")
            .arg("--retry")
            .arg("2")
            .arg("-o")
            .arg(&dest)
            .arg(&url)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        hidden(&mut cmd);
        let res = match cmd.output() {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => Err(format!(
                "curl: {}",
                String::from_utf8_lossy(&o.stderr).trim().to_string()
            )),
            Err(e) => Err(e.to_string()),
        };
        let _ = tx.send(res);
    });
    rx
}

/// Liệt kê các tệp có thể dùng làm audio gốc để nhận dạng.
/// Dựng đoạn từ làn Âm thanh (dùng khi làn Hình ảnh không có tiếng).
fn push_audio_segments(
    p: &Project,
    t0: f64,
    t1: f64,
    out: &mut Vec<Seg>,
    input_index: &std::collections::HashMap<u64, usize>,
) {
    let mut items: Vec<&Clip> = p.clips_on_track(TRACK_AUDIO);
    items.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    let mut cursor = t0;
    for c in items {
        if c.end() <= t0 || c.start >= t1 {
            continue;
        }
        let Some(&k) = input_index.get(&c.source) else {
            continue;
        };
        let s = c.start.max(t0);
        let e = c.end().min(t1);
        if s > cursor + 1e-4 {
            out.push(Seg {
                source_idx: usize::MAX,
                in_pt: 0.0,
                dur: s - cursor,
                fx: String::new(),
            });
        }
        out.push(Seg {
            source_idx: k,
            in_pt: c.in_pt + (s - c.start),
            dur: e - s,
            fx: String::new(),
        });
        cursor = e;
    }
    if t1 > cursor + 1e-4 {
        out.push(Seg {
            source_idx: usize::MAX,
            in_pt: 0.0,
            dur: t1 - cursor,
            fx: String::new(),
        });
    }
    if out.is_empty() {
        out.push(Seg {
            source_idx: usize::MAX,
            in_pt: 0.0,
            dur: t1.max(0.05),
            fx: String::new(),
        });
    }
}

fn audio_program_filters(
    project: &Project,
    total: f64,
    input_index: &std::collections::HashMap<u64, usize>,
    idx_of: &dyn Fn(u64) -> usize,
    ids: &[u64],
) -> (Vec<String>, String) {
    // Ưu tiên tiếng của làn Hình ảnh; nếu làn này không có tiếng thì lấy làn Âm thanh.
    let has_video_audio = project
        .clips_on_track(TRACK_VIDEO)
        .iter()
        .any(|c| source_has_audio(project, c.source));

    let mut segs: Vec<Seg> = Vec::new();
    if has_video_audio {
        push_segments(project, 0.0, total, &mut segs, idx_of);
    } else {
        push_audio_segments(project, 0.0, total, &mut segs, input_index);
    }

    let mut filters: Vec<String> = Vec::new();
    let mut chain = String::new();
    let mut n = 0usize;
    for seg in &segs {
        let d = seg.dur.max(0.02);
        let has_audio = seg.source_idx != usize::MAX
            && ids
                .get(seg.source_idx)
                .and_then(|id| project.source(*id))
                .map(|s| s.has_audio)
                .unwrap_or(false);
        if has_audio {
            filters.push(format!(
                "[{k}:a]atrim=start={a:.3}:end={b:.3},asetpts=PTS-STARTPTS,\
                 aresample=16000,aformat=sample_fmts=fltp:channel_layouts=mono[a{n}]",
                k = seg.source_idx,
                a = seg.in_pt,
                b = seg.in_pt + d,
                n = n
            ));
        } else {
            filters.push(format!(
                "anullsrc=r=16000:cl=mono,atrim=duration={:.3},\
                 aformat=sample_fmts=fltp:channel_layouts=mono[a{}]",
                d, n
            ));
        }
        chain.push_str(&format!("[a{}]", n));
        n += 1;
    }
    filters.push(format!("{}concat=n={}:v=0:a=1[basea]", chain, n));
    let label = if has_video_audio {
        mix_audio(project, input_index, &mut filters, "basea", false)
    } else {
        "basea".to_string()
    };
    (filters, label)
}

pub fn start_transcribe(
    project: &Project,
    model: &Path,
    language: &str,
) -> Result<TranscribeJob, String> {
    let ff = ffmpeg().ok_or_else(|| "Không tìm thấy FFmpeg".to_string())?;
    if !model.is_file() {
        return Err("Chưa có mô hình Whisper".into());
    }
    let total = project.duration();
    if total <= 0.05 {
        return Err("Dòng thời gian đang trống".into());
    }

    let (inputs, ids, input_index) = build_inputs(project, total, project.fps.max(1.0));
    if inputs.is_empty() {
        return Err("Dòng thời gian đang trống".into());
    }
    let idx_of = |id: u64| -> usize { *input_index.get(&id).unwrap_or(&0) };
    let (filters, audio_label) = audio_program_filters(project, total, &input_index, &idx_of, &ids);

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = temp_dir().join(format!("asr_{}_{}", std::process::id(), stamp));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let model = model.to_path_buf();
    let language = language.to_string();
    let (tx, rx) = channel::<TranscribeMsg>();
    let ff = ff.clone();

    std::thread::spawn(move || {
        let _ = tx.send(TranscribeMsg::Status("audio".into()));
        let wav = dir.join("program.wav");
        let mut cmd = Command::new(&ff);
        for input in &inputs {
            for a in &input.pre {
                cmd.arg(a);
            }
            cmd.arg("-i").arg(&input.path);
        }
        cmd.arg("-filter_complex")
            .arg(filters.join(";"))
            .arg("-map")
            .arg(format!("[{}]", audio_label))
            .arg("-ar")
            .arg("16000")
            .arg("-ac")
            .arg("1")
            .arg("-c:a")
            .arg("pcm_s16le")
            .arg("-y")
            .arg(&wav)
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        hidden(&mut cmd);
        let out = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                let _ = tx.send(TranscribeMsg::Done(Err(e.to_string())));
                return;
            }
        };
        if !out.status.success() || !wav.is_file() {
            let _ = tx.send(TranscribeMsg::Done(Err(format!(
                "Không trích được âm thanh: {}",
                String::from_utf8_lossy(&out.stderr)
                    .lines()
                    .last()
                    .unwrap_or("")
            ))));
            return;
        }

        let _ = tx.send(TranscribeMsg::Status("whisper".into()));

        // Dùng đường dẫn tương đối để tránh phải thoát ký tự ':' của Windows.
        let model_local = dir.join("ggml-model.bin");
        let linked = std::fs::hard_link(&model, &model_local).is_ok()
            || std::fs::copy(&model, &model_local).is_ok();
        if !linked {
            let _ = tx.send(TranscribeMsg::Done(Err("Không đọc được mô hình".into())));
            return;
        }

        let mut cmd = Command::new(&ff);
        cmd.current_dir(&dir)
            .arg("-v")
            .arg("error")
            .arg("-nostdin")
            .arg("-i")
            .arg("program.wav")
            .arg("-af")
            .arg(format!(
                "whisper=model=ggml-model.bin:language={}:format=srt:destination=caps.srt",
                language
            ))
            .arg("-f")
            .arg("null")
            .arg("-")
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        hidden(&mut cmd);
        let out = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                let _ = tx.send(TranscribeMsg::Done(Err(e.to_string())));
                let _ = std::fs::remove_dir_all(&dir);
                return;
            }
        };

        let srt_path = dir.join("caps.srt");
        let srt = std::fs::read_to_string(&srt_path).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);

        if !out.status.success() || srt.trim().is_empty() {
            let msg = String::from_utf8_lossy(&out.stderr);
            let tail: Vec<&str> = msg.lines().rev().take(3).collect();
            let _ = tx.send(TranscribeMsg::Done(Err(format!(
                "whisper: {}",
                tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
            ))));
            return;
        }
        let _ = tx.send(TranscribeMsg::Done(Ok(srt)));
    });

    Ok(TranscribeJob {
        rx,
        status: String::new(),
        finished: false,
        result: None,
    })
}

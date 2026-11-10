//! Lớp giao tiếp với FFmpeg: đọc thông tin tệp, trích khung hình, xuất video.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;

use crate::model::{Clip, Project, Source, SourceKind, TRACK_AUDIO, TRACK_OVERLAY, TRACK_VIDEO};

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

    // 1) cùng thư mục với chương trình
    for sub in ["", "ffmpeg/bin/", "bin/"] {
        let p = exe_dir().join(sub).join(&exe);
        if p.exists() {
            return Some(p);
        }
    }

    // 2) thử gọi trực tiếp (dựa vào PATH)
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
        return "FFmpeg: không tìm thấy".to_string();
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
    "FFmpeg: đã tìm thấy".to_string()
}

// ---------------------------------------------------------------------------
// Đọc thông tin tệp
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
        .unwrap_or("không tên")
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
// Trích khung hình
// ---------------------------------------------------------------------------

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join("viedit_frames");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Trích một khung hình tại thời điểm `time`, trả về bytes PNG.
pub fn extract_frame_png(path: &str, time: f64, max_width: u32) -> Result<Vec<u8>, String> {
    let ff = ffmpeg().ok_or_else(|| "Không tìm thấy FFmpeg".to_string())?;

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
// Nạp khung hình chạy nền
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
// Xuất video
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

struct Seg {
    source_idx: usize,
    in_pt: f64,
    dur: f64,
}

fn push_segments(p: &Project, t0: f64, t1: f64, out: &mut Vec<Seg>, input_of: &dyn Fn(u64) -> usize) {
    let clips: Vec<&Clip> = p
        .clips
        .iter()
        .filter(|c| c.track == TRACK_VIDEO || c.track == TRACK_AUDIO)
        .collect();

    // chỉ lấy clip video để dựng hình nền
    let mut vids: Vec<&Clip> = clips
        .into_iter()
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
            });
        }
        out.push(Seg {
            source_idx: input_of(c.source),
            in_pt: c.in_pt + (s - c.start),
            dur: e - s,
        });
        cursor = e;
    }
    if t1 > cursor + 1e-4 {
        out.push(Seg {
            source_idx: usize::MAX,
            in_pt: 0.0,
            dur: t1 - cursor,
        });
    }
}

pub fn start_export(
    project: &Project,
    output: &Path,
    quality: Quality,
) -> Result<ExportJob, String> {
    if ffmpeg().is_none() {
        return Err("Không tìm thấy FFmpeg".into());
    }

    let total = project.duration();
    if total <= 0.01 {
        return Err("Dòng thời gian đang trống".into());
    }

    // Bảng input: mỗi tệp nguồn được nạp một lần
    let mut inputs: Vec<String> = Vec::new();
    let mut input_index: std::collections::HashMap<u64, usize> = Default::default();
    for s in &project.sources {
        if project.clips.iter().any(|c| c.source == s.id) {
            input_index.insert(s.id, inputs.len());
            inputs.push(s.path.clone());
        }
    }

    let idx_of = |id: u64| -> usize { *input_index.get(&id).unwrap_or(&0) };

    let mut segs: Vec<Seg> = Vec::new();
    push_segments(project, 0.0, total, &mut segs, &idx_of);
    if segs.is_empty() {
        return Err("Dòng thời gian đang trống".into());
    }

    let w = project.width;
    let h = project.height;
    let fps = project.fps;

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
                "anullsrc=r=48000:cl=stereo,atrim=duration={:.3},aformat=sample_fmts=fltp:channel_layouts=stereo[a{}]",
                d, n
            ));
        } else {
            let has_audio = project
                .sources
                .iter()
                .find(|s| input_index.get(&s.id) == Some(&seg.source_idx))
                .map(|s| s.has_audio)
                .unwrap_or(false);

            filters.push(format!(
                "[{k}:v]trim=start={a:.3}:end={b:.3},setpts=PTS-STARTPTS,\
                 scale={w}:{h}:force_original_aspect_ratio=decrease,\
                 pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps},format=yuv420p[v{n}]",
                k = seg.source_idx,
                a = seg.in_pt,
                b = seg.in_pt + d,
                w = w,
                h = h,
                fps = fps,
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
                    "anullsrc=r=48000:cl=stereo,atrim=duration={:.3},aformat=sample_fmts=fltp:channel_layouts=stereo[a{}]",
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

    let mut current_v = "basev".to_string();
    for (i, ov) in overlays.iter().enumerate() {
        let k = idx_of(ov.source);
        let opacity = ov.opacity.clamp(0.0, 1.0);
        filters.push(format!(
            "[{k}:v]trim=start={a:.3}:end={b:.3},setpts=PTS-STARTPTS+{s:.3}/TB,\
             scale={w}:{h}:force_original_aspect_ratio=decrease,format=rgba,\
             colorchannelmixer=aa={op:.2}[ov{i}]",
            k = k,
            a = ov.in_pt,
            b = ov.in_pt + ov.dur,
            s = ov.start,
            w = w,
            h = h,
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

    let filter_complex = filters.join(";");

    let mut cmd = Command::new(ffmpeg().unwrap());
    for input in &inputs {
        cmd.arg("-i").arg(input);
    }
    cmd.arg("-filter_complex")
        .arg(&filter_complex)
        .arg("-map")
        .arg(format!("[{}]", current_v))
        .arg("-map")
        .arg("[basea]")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg(quality.preset())
        .arg("-crf")
        .arg(quality.crf().to_string())
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
        .arg(output)
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

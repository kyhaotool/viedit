//! Kiểm thử tích hợp: dựng một dự án nhỏ rồi xuất ra MP4 bằng FFmpeg.
//!
//! Bỏ qua (skip) nếu máy không có FFmpeg hoặc thiếu tài nguyên mẫu.

use std::path::{Path, PathBuf};
use std::time::Duration;

use viedit::media::{self, ExportMsg, Quality};
use viedit::model::{Clip, Project, SourceKind, TRACK_OVERLAY, TRACK_VIDEO};

fn library_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("library")
}

fn first_with_ext(root: &Path, ext: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, ext: &str, depth: usize) -> Option<PathBuf> {
        if depth > 4 {
            return None;
        }
        let entries = std::fs::read_dir(dir).ok()?;
        let mut dirs = Vec::new();
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
            {
                return Some(p);
            }
        }
        for d in dirs {
            if let Some(found) = walk(&d, ext, depth + 1) {
                return Some(found);
            }
        }
        None
    }
    walk(root, ext, 0)
}

#[test]
fn xuat_video_ra_mp4() {
    if !media::available() {
        eprintln!("bỏ qua: máy chưa có FFmpeg");
        return;
    }

    let lib = library_dir();
    let gif_path = first_with_ext(&lib, "gif").expect("cần ít nhất một tệp .gif mẫu");
    let mov_path = first_with_ext(&lib, "mp4");

    let mut project = Project::default();
    project.width = 1280;
    project.height = 720;
    project.fps = 30.0;
    project.name = "kiem-thu".into();

    // nền: một video ngắn (nếu có), còn không thì nền đen
    if let Some(mp4) = mov_path {
        if let Some(mut src) = media::probe(&mp4) {
            src.id = project.new_id();
            let id = src.id;
            let dur = src.duration.clamp(0.5, 2.0);
            project.sources.push(src);
            let clip_id = project.new_id();
            project.clips.push(Clip {
                id: clip_id,
                source: id,
                track: TRACK_VIDEO,
                start: 0.0,
                in_pt: 0.0,
                dur,
                volume: 1.0,
                opacity: 1.0,
                text: String::new(),
            });
        }
    }

    // lớp phủ: tiêu đề động GIF
    let mut gif_src = media::probe(&gif_path).expect("đọc được gif");
    assert_eq!(gif_src.kind, SourceKind::Gif);
    gif_src.id = project.new_id();
    let gif_id = gif_src.id;
    project.sources.push(gif_src);
    let overlay_clip = project.new_id();
    project.clips.push(Clip {
        id: overlay_clip,
        source: gif_id,
        track: TRACK_OVERLAY,
        start: 0.3,
        in_pt: 0.0,
        dur: 1.5,
        volume: 1.0,
        opacity: 0.9,
        text: String::new(),
    });

    assert!(project.duration() > 0.5);

    let out = std::env::temp_dir().join("viedit-test-export.mp4");
    let _ = std::fs::remove_file(&out);

    let mut job = media::start_export(&project, &out, Quality::Draft)
        .expect("khởi động được tiến trình xuất");

    let mut result: Option<Result<String, String>> = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(180);
    while std::time::Instant::now() < deadline {
        match job.rx.recv_timeout(Duration::from_millis(500)) {
            Ok(ExportMsg::Done(res)) => {
                result = Some(res);
                break;
            }
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }

    let res = result.expect("tiến trình xuất phải kết thúc");
    assert!(res.is_ok(), "ffmpeg báo lỗi: {:?}", res);

    let meta = std::fs::metadata(&out).expect("tệp kết quả phải tồn tại");
    assert!(
        meta.len() > 4096,
        "tệp kết quả quá nhỏ: {} bytes",
        meta.len()
    );

    eprintln!(
        "xuất thành công: {} ({:.1} KB)",
        out.display(),
        meta.len() as f64 / 1024.0
    );
    let _ = std::fs::remove_file(&out);
}

#[test]
fn thoi_gian_dinh_dang_dung() {
    use viedit::model::{fmt_dur, fmt_time};
    assert_eq!(fmt_time(0.0), "00:00.00");
    assert_eq!(fmt_time(65.5), "01:05.50");
    assert_eq!(fmt_dur(65.5), "01:06");
}

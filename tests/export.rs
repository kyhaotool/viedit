//! Kiểm thử tích hợp: dựng một dự án nhỏ rồi xuất ra MP4 bằng FFmpeg.
//!
//! Bỏ qua (skip) nếu máy không có FFmpeg hoặc thiếu tài nguyên mẫu.

use std::path::{Path, PathBuf};
use std::time::Duration;

use viedit::media::{self, ExportMsg, Quality};
use viedit::model::{Clip, Project, SourceKind, TRACK_AUDIO, TRACK_CAPTION, TRACK_OVERLAY, TRACK_VIDEO};

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
                ..Default::default()
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
        ..Default::default()
    });

    assert!(project.duration() > 0.5);

    let out = std::env::temp_dir().join("viedit-test-export.mp4");
    let _ = std::fs::remove_file(&out);

    let job = media::start_export(&project, &out, Quality::Draft)
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

/// Sinh một tệp WAV hình sin nhỏ để thử làn âm thanh.
fn sine_wav(path: &Path, secs: f64) -> Option<()> {
    use std::io::Write;
    let rate = 48_000u32;
    let n = (rate as f64 * secs) as u32;
    let data_len = n * 4;
    let mut f = std::fs::File::create(path).ok()?;
    f.write_all(b"RIFF").ok()?;
    f.write_all(&(36 + data_len).to_le_bytes()).ok()?;
    f.write_all(b"WAVEfmt ").ok()?;
    f.write_all(&16u32.to_le_bytes()).ok()?;
    f.write_all(&1u16.to_le_bytes()).ok()?;
    f.write_all(&2u16.to_le_bytes()).ok()?;
    f.write_all(&rate.to_le_bytes()).ok()?;
    f.write_all(&(rate * 4).to_le_bytes()).ok()?;
    f.write_all(&4u16.to_le_bytes()).ok()?;
    f.write_all(&16u16.to_le_bytes()).ok()?;
    f.write_all(b"data").ok()?;
    f.write_all(&data_len.to_le_bytes()).ok()?;
    for i in 0..n {
        let t = i as f64 / rate as f64;
        let v = (t * 440.0 * std::f64::consts::TAU).sin() * 0.2;
        let s = (v * 32767.0) as i16;
        f.write_all(&s.to_le_bytes()).ok()?;
        f.write_all(&s.to_le_bytes()).ok()?;
    }
    Some(())
}

#[test]
fn doc_ghi_srt() {
    let raw = "\u{feff}1\r\n00:00:01,200 --> 00:00:03,400\r\nXin chào\r\n\r\n\
               2\r\n00:00:04,000 --> 00:00:05,500\r\nDòng thứ hai\r\n";
    let caps = media::parse_srt(raw);
    assert_eq!(caps.len(), 2);
    assert!((caps[0].start - 1.2).abs() < 1e-6);
    assert!((caps[0].end - 3.4).abs() < 1e-6);
    assert_eq!(caps[0].text, "Xin chào");
    assert_eq!(caps[1].text, "Dòng thứ hai");

    let encoded = media::format_srt(&caps);
    let again = media::parse_srt(&encoded);
    assert_eq!(again.len(), 2);
    assert_eq!(again[1].text, "Dòng thứ hai");
    assert!((again[1].end - 5.5).abs() < 1e-6);
}

#[test]
fn xuat_co_phu_de_va_nhac_nen() {
    if !media::available() {
        eprintln!("bỏ qua: máy chưa có FFmpeg");
        return;
    }
    let lib = library_dir();
    let Some(gif_path) = first_with_ext(&lib, "gif") else {
        eprintln!("bỏ qua: chưa có tệp GIF mẫu");
        return;
    };

    let mut project = Project::default();
    project.width = 960;
    project.height = 540;
    project.fps = 24.0;

    let mut gif_src = media::probe(&gif_path).expect("đọc được gif");
    gif_src.id = project.new_id();
    let gif_id = gif_src.id;
    gif_src.duration = 3.0;
    project.sources.push(gif_src);

    let clip_id = project.new_id();
    project.clips.push(Clip {
        id: clip_id,
        source: gif_id,
        track: TRACK_VIDEO,
        start: 0.0,
        in_pt: 0.0,
        dur: 3.0,
        text: String::new(),
        effect: "hue=s=0".into(),
        brightness: 0.08,
        contrast: 1.1,
        ..Default::default()
    });

    let wav = std::env::temp_dir().join("viedit-test-nhac.wav");
    let _ = std::fs::remove_file(&wav);
    if sine_wav(&wav, 2.0).is_some() {
        if let Some(mut src) = media::probe(&wav) {
            src.id = project.new_id();
            let aid = src.id;
            project.sources.push(src);
            let cid = project.new_id();
            project.clips.push(Clip {
                id: cid,
                source: aid,
                track: TRACK_AUDIO,
                start: 0.4,
                in_pt: 0.0,
                dur: 1.6,
                volume: 0.5,
                ..Default::default()
            });
        }
    }

    let cap_a = project.new_id();
    project.clips.push(Clip {
        id: cap_a,
        source: 0,
        track: TRACK_CAPTION,
        start: 0.2,
        dur: 1.4,
        text: "Xin chào, đây là VieEdit!".into(),
        ..Default::default()
    });
    let cap_b = project.new_id();
    project.clips.push(Clip {
        id: cap_b,
        source: 0,
        track: TRACK_CAPTION,
        start: 1.7,
        dur: 1.2,
        text: "Phụ đề tiếng Việt có dấu.".into(),
        ..Default::default()
    });

    let out = std::env::temp_dir().join("viedit-test-captions.mp4");
    let _ = std::fs::remove_file(&out);

    let opts = media::ExportOptions {
        quality: Quality::Draft,
        loudnorm: true,
        burn_captions: true,
    };
    let job = media::start_export_with(&project, &out, opts)
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
    assert!(meta.len() > 8192, "tệp kết quả quá nhỏ: {}", meta.len());
    eprintln!(
        "xuất kèm phụ đề + nhạc nền: {} ({:.1} KB)",
        out.display(),
        meta.len() as f64 / 1024.0
    );
    let _ = std::fs::remove_file(&out);
    let _ = std::fs::remove_file(&wav);
}

#[test]
fn nhan_dang_giong_noi() {
    // Chỉ chạy khi có tệp mẫu giọng nói và mô hình Whisper.
    // Đặt biến môi trường VIEDIT_ASR_SAMPLE trỏ tới một tệp WAV/MP3 có lời.
    let Ok(sample) = std::env::var("VIEDIT_ASR_SAMPLE") else {
        eprintln!("bỏ qua: chưa đặt VIEDIT_ASR_SAMPLE");
        return;
    };
    let Some(model) = media::find_model() else {
        eprintln!("bỏ qua: chưa có mô hình Whisper");
        return;
    };

    let mut project = Project::default();
    project.name = "asr".into();
    let Some(mut src) = media::probe(Path::new(&sample)) else {
        eprintln!("bỏ qua: không đọc được tệp mẫu");
        return;
    };
    src.id = project.new_id();
    let sid = src.id;
    let dur = src.duration.clamp(0.5, 30.0);
    project.sources.push(src);
    let cid = project.new_id();
    project.clips.push(Clip {
        id: cid,
        source: sid,
        track: TRACK_AUDIO,
        start: 0.0,
        in_pt: 0.0,
        dur,
        ..Default::default()
    });

    let job = media::start_transcribe(&project, &model, "en").expect("khởi động được ASR");
    let mut result: Option<Result<String, String>> = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(600);
    while std::time::Instant::now() < deadline {
        match job.rx.recv_timeout(Duration::from_millis(500)) {
            Ok(media::TranscribeMsg::Done(res)) => {
                result = Some(res);
                break;
            }
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }

    let srt = result.expect("ASR phải kết thúc").expect("ASR phải thành công");
    let caps = media::parse_srt(&srt);
    assert!(!caps.is_empty(), "phải nhận được ít nhất một câu");
    eprintln!("nhận dạng được {} câu: {:?}", caps.len(), caps[0].text);
}
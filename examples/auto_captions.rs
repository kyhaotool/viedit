//! Ví dụ đầu–cuối: tự tạo phụ đề cho một video rồi xuất bản có phụ đề cháy sẵn.
//!
//! ```bash
//! cargo run --release --example auto_captions -- input.mp4 output.mp4 vi
//! ```
//!
//! Cần: FFmpeg có filter `whisper`, và mô hình `ggml-*.bin` trong `assets/models`.
//! Ví dụ này dùng chính thư viện lõi của VieEdit (`src/lib.rs`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use viedit::media::{self, ExportMsg, ExportOptions, Quality, TranscribeMsg};
use viedit::model::{Clip, Project, TRACK_CAPTION, TRACK_VIDEO};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Dùng: auto_captions <video vào> <video ra> [vi|en|auto]");
        std::process::exit(2);
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let lang = args.get(3).map(|s| s.as_str()).unwrap_or("vi");

    if let Err(e) = run(&input, &output, lang) {
        eprintln!("Lỗi: {e}");
        std::process::exit(1);
    }
}

fn run(input: &Path, output: &Path, lang: &str) -> Result<(), String> {
    let mut project = Project::default();
    let mut src = media::probe(input).ok_or("không đọc được tệp đầu vào")?;
    project.width = src.width.max(2);
    project.height = src.height.max(2);
    if src.fps > 1.0 {
        project.fps = src.fps;
    }
    src.id = project.new_id();
    let src_id = src.id;
    let dur = src.duration.max(0.5);
    project.sources.push(src);

    let clip_id = project.new_id();
    project.clips.push(Clip {
        id: clip_id,
        source: src_id,
        track: TRACK_VIDEO,
        start: 0.0,
        in_pt: 0.0,
        dur,
        ..Default::default()
    });
    println!("Video: {} giây, {}x{}", dur, project.width, project.height);

    let model = media::find_model().ok_or("chưa có mô hình ggml-*.bin trong assets/models")?;
    println!("Mô hình: {}", model.display());

    let job = media::start_transcribe(&project, &model, lang)?;
    let mut srt = None;
    let deadline = Instant::now() + Duration::from_secs(1800);
    while Instant::now() < deadline {
        match job.rx.recv_timeout(Duration::from_millis(400)) {
            Ok(TranscribeMsg::Status(s)) => println!("  · {s}"),
            Ok(TranscribeMsg::Done(res)) => {
                srt = Some(res?);
                break;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
    let srt = srt.ok_or("quá thời gian nhận dạng")?;

    let caps = media::parse_srt(&srt);
    if caps.is_empty() {
        return Err("không nhận dạng được lời nói nào".into());
    }
    for c in &caps {
        println!("  [{:7.2} → {:7.2}] {}", c.start, c.end, c.text);
    }

    for c in &caps {
        let id = project.new_id();
        project.clips.push(Clip {
            id,
            source: 0,
            track: TRACK_CAPTION,
            start: c.start,
            in_pt: 0.0,
            dur: (c.end - c.start).max(0.2),
            text: c.text.clone(),
            ..Default::default()
        });
    }

    let opts = ExportOptions {
        quality: Quality::High,
        loudnorm: true,
        burn_captions: true,
    };
    println!("Đang xuất {} …", output.display());
    let job = media::start_export_with(&project, output, opts)?;
    let mut done = None;
    let deadline = Instant::now() + Duration::from_secs(3600);
    while Instant::now() < deadline {
        match job.rx.recv_timeout(Duration::from_millis(500)) {
            Ok(ExportMsg::Progress(p)) => {
                if (p * 100.0) as i32 % 10 == 0 {
                    print!("\r  {:>3.0}%", p * 100.0);
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
            }
            Ok(ExportMsg::Done(res)) => {
                done = Some(res);
                break;
            }
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => break,
        }
    }
    println!();
    done.ok_or("quá thời gian xuất")??;

    let size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    println!(
        "Xong: {} ({:.1} MB, {} câu phụ đề)",
        output.display(),
        size as f64 / 1_048_576.0,
        caps.len()
    );
    Ok(())
}

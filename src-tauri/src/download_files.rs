//! Only the file reported by this task's downloader may become its result.
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub const RESULT_PREFIX: &str = "DOWNLOADED:";

pub fn downloader_directory(path: &Path) -> String {
    // aria2 cannot open Rust's Windows verbatim paths (\\?\C:\...).
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
    }
}

pub fn identity_arguments() -> Vec<String> {
    [
        "--ignore-config",
        "--no-playlist",
        "--abort-on-error",
        "--no-simulate",
        "--progress",
        "--print",
        "after_move:DOWNLOADED:%(filepath)j",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            _ => c,
        })
        .collect();
    let short: String = cleaned.trim_matches([' ', '.']).chars().take(80).collect();
    let mut safe = short.trim_end_matches([' ', '.']).to_string();
    let stem = safe.split('.').next().unwrap_or("").to_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        safe.insert(0, '_');
    }
    if safe.is_empty() {
        "video".to_string()
    } else {
        safe
    }
}

pub fn task_directory(output: &Path, task_id: &str) -> Result<PathBuf, String> {
    // Do not sanitize IDs into the same name: reject invalid IDs instead.
    if task_id.is_empty()
        || task_id.len() > 100
        || !task_id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err("下载任务 ID 无效".into());
    }
    Ok(output.join(".bilibili-downloads").join(task_id))
}

pub fn reported_path(line: &str) -> Result<Option<PathBuf>, String> {
    line.strip_prefix(RESULT_PREFIX)
        .map(|json| {
            serde_json::from_str::<String>(json)
                .map(PathBuf::from)
                .map_err(|e| format!("下载结果路径无效: {e}"))
        })
        .transpose()
}

pub fn finalize_download(
    process_success: bool,
    reported: &[PathBuf],
    working_dir: &Path,
    destination: &Path,
    title: &str,
    audio_only: bool,
) -> Result<PathBuf, String> {
    if !process_success {
        return Err("下载进程失败，保留临时文件以便重试".into());
    }
    if reported.len() != 1 {
        return Err("未取得唯一的下载结果，未移动任何文件".into());
    }
    let ext = if audio_only { "mp3" } else { "mp4" };
    let root = fs::canonicalize(working_dir).map_err(|e| format!("无法读取下载目录: {e}"))?;
    let source = fs::canonicalize(&reported[0]).map_err(|e| format!("下载结果不存在: {e}"))?;
    // A fixed basename prevents a stale stream or an unrelated file becoming a result.
    if source != root.join(format!("media.{ext}")) {
        return Err("下载结果不属于当前任务，未移动任何文件".into());
    }
    if !valid_media(&source, audio_only) {
        return Err("下载文件为空或格式不正确".into());
    }
    fs::create_dir_all(destination).map_err(|e| format!("无法创建保存目录: {e}"))?;
    let base = sanitize_filename(title);
    for index in 0..10_000 {
        let name = if index == 0 {
            format!("{base}.{ext}")
        } else {
            format!("{base} ({index}).{ext}")
        };
        let target = destination.join(name);
        // Same-volume downloads can be moved without copying a large video.
        match fs::hard_link(&source, &target) {
            Ok(()) => {
                let _ = fs::remove_file(&source);
                let _ = fs::remove_dir(&root);
                return Ok(target);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => {} // Other volumes/filesystems use the exclusive copy below.
        }
        // create_new is atomic, including when several tasks finish together.
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("保存下载文件失败: {e}")),
        };
        let copied = (|| {
            let mut input = fs::File::open(&source)?;
            io::copy(&mut input, &mut file)?;
            file.sync_all()
        })();
        drop(file);
        if let Err(e) = copied {
            let _ = fs::remove_file(&target);
            return Err(format!("保存下载文件失败，临时文件已保留: {e}"));
        }
        let _ = fs::remove_file(&source);
        // Only remove an empty task directory; failed/resumable streams are preserved.
        let _ = fs::remove_dir(&root);
        return Ok(target);
    }
    Err("同名文件过多，未覆盖已有文件".into())
}

fn valid_media(path: &Path, audio: bool) -> bool {
    use io::Read;
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut header = [0u8; 12];
    if file.read_exact(&mut header).is_err() {
        return false;
    }
    if audio {
        &header[..3] == b"ID3" || (header[0] == 0xff && header[1] & 0xe0 == 0xe0)
    } else {
        &header[4..8] == b"ftyp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Sandbox(PathBuf);
    impl Sandbox {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("bilibili-files-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn media(&self, task: &str, audio: bool) -> PathBuf {
            let dir = task_directory(&self.0, task).unwrap();
            fs::create_dir_all(&dir).unwrap();
            let file = dir.join(if audio { "media.mp3" } else { "media.mp4" });
            // A short valid header intentionally stays below the former 1 MB threshold.
            fs::write(
                &file,
                if audio {
                    b"ID3small-audio".as_slice()
                } else {
                    b"\0\0\0\x18ftypisomsmall-video".as_slice()
                },
            )
            .unwrap();
            file
        }
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn short_video_never_selects_larger_existing_video() {
        let s = Sandbox::new();
        let old = s.0.join("old.mp4");
        fs::write(&old, vec![42; 2_000_000]).unwrap();
        let source = s.media("task_short", false);
        let result = finalize_download(
            true,
            &[source.clone()],
            source.parent().unwrap(),
            &s.0,
            "绝望舞步",
            false,
        )
        .unwrap();
        assert_eq!(fs::read(result).unwrap(), b"\0\0\0\x18ftypisomsmall-video");
        assert_eq!(fs::metadata(old).unwrap().len(), 2_000_000);
    }

    #[test]
    fn failed_or_ambiguous_download_does_not_move_files() {
        let s = Sandbox::new();
        let source = s.media("task_failed", false);
        for (ok, paths) in [
            (false, vec![source.clone()]),
            (true, vec![]),
            (true, vec![source.clone(), source.clone()]),
        ] {
            assert!(
                finalize_download(ok, &paths, source.parent().unwrap(), &s.0, "wrong", false)
                    .is_err()
            );
        }
        assert!(source.exists());
        assert!(!s.0.join("wrong.mp4").exists());
    }

    #[test]
    fn another_tasks_file_and_unmerged_stream_are_rejected() {
        let s = Sandbox::new();
        let first = s.media("task_p1", false);
        let second = s.media("task_p2", false);
        assert!(finalize_download(
            true,
            &[second.clone()],
            first.parent().unwrap(),
            &s.0,
            "P01",
            false
        )
        .is_err());
        let stream = first.with_file_name("media.f100.mp4");
        fs::copy(&first, &stream).unwrap();
        assert!(
            finalize_download(true, &[stream], first.parent().unwrap(), &s.0, "P01", false)
                .is_err()
        );
        assert!(first.exists() && second.exists());
    }

    #[test]
    fn concurrent_same_titles_keep_both_outputs_and_existing_file() {
        let s = Sandbox::new();
        fs::write(s.0.join("同名.mp4"), b"original").unwrap();
        let handles: Vec<_> = ["task_a", "task_b"]
            .into_iter()
            .map(|id| {
                let source = s.media(id, false);
                let output = s.0.clone();
                std::thread::spawn(move || {
                    finalize_download(
                        true,
                        &[source.clone()],
                        source.parent().unwrap(),
                        &output,
                        "同名",
                        false,
                    )
                    .unwrap()
                })
            })
            .collect();
        let paths: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_ne!(paths[0], paths[1]);
        assert!(paths.iter().all(|p| p.exists()));
        assert_eq!(fs::read(s.0.join("同名.mp4")).unwrap(), b"original");
    }

    #[test]
    fn save_failure_preserves_source_and_audio_names() {
        let s = Sandbox::new();
        let source = s.media("task_audio", true);
        let blocked = s.0.join("not_a_directory");
        fs::write(&blocked, b"old").unwrap();
        assert!(finalize_download(
            true,
            &[source.clone()],
            source.parent().unwrap(),
            &blocked,
            "音乐",
            true
        )
        .is_err());
        assert!(source.exists());
        let result = finalize_download(
            true,
            &[source.clone()],
            source.parent().unwrap(),
            &s.0,
            "音乐",
            true,
        )
        .unwrap();
        assert_eq!(result.file_name().unwrap(), "音乐.mp3");
    }

    #[test]
    fn invalid_ids_and_windows_names_are_safe() {
        assert_eq!(
            downloader_directory(Path::new(r"\\?\C:\Videos\task")),
            r"C:\Videos\task"
        );
        assert_eq!(
            downloader_directory(Path::new(r"\\?\UNC\server\share\task")),
            r"\\server\share\task"
        );
        assert!(task_directory(Path::new("."), "../other").is_err());
        assert_eq!(sanitize_filename("CON.txt"), "_CON.txt");
        assert_eq!(sanitize_filename(" .. "), "video");
        assert_eq!(sanitize_filename("P02. 名称:测试"), "P02. 名称_测试");
        assert_eq!(reported_path("other output").unwrap(), None);
        assert_eq!(
            reported_path(r#"DOWNLOADED:"C:\\目录\\media.mp4""#).unwrap(),
            Some(PathBuf::from("C:\\目录\\media.mp4"))
        );
    }

    #[test]
    #[ignore = "requires BILIBILI_QA_URL and a real network download"]
    fn live_video_and_audio_download() {
        use std::process::Command;
        let url = std::env::var("BILIBILI_QA_URL").expect("set a short test video URL");
        let s = Sandbox::new();
        let tools = Path::new(env!("CARGO_MANIFEST_DIR"));
        let original = s.0.join("previous_video.mp4");
        fs::write(&original, vec![42; 2_000_000]).unwrap();
        for audio in [false, true] {
            let work =
                task_directory(&s.0, if audio { "task_audio" } else { "task_video" }).unwrap();
            fs::create_dir_all(&work).unwrap();
            let work = fs::canonicalize(work).unwrap();
            let mut command = Command::new(tools.join("yt-dlp.exe"));
            command
                .args(identity_arguments())
                .args(["--newline", "--encoding", "utf-8", "-P"])
                .arg(downloader_directory(&work))
                .args(["-o", "media.%(ext)s", "--ffmpeg-location"])
                .arg(tools.join("ffmpeg.exe"));
            command
                .args([
                    "--windows-filenames",
                    "--restrict-filenames",
                    "--external-downloader",
                ])
                .arg(tools.join("aria2c.exe"));
            for proxy in [
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "http_proxy",
                "https_proxy",
                "ALL_PROXY",
                "all_proxy",
            ] {
                command.env_remove(proxy);
            }
            command.args(["--add-header", "User-Agent:Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
                "--add-header", "Referer:https://www.bilibili.com/", "--add-header", "Origin:https://www.bilibili.com"]);
            if audio {
                command.args(["-x", "--audio-format", "mp3", "-f", "bestaudio/best"]);
            } else {
                command.args([
                    "--merge-output-format",
                    "mp4",
                    "-f",
                    "bestvideo[height<=480]+bestaudio/bestvideo+bestaudio/best",
                ]);
            }
            command.arg(&url);
            let result = command.output().unwrap();
            assert!(
                result.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&result.stderr),
                String::from_utf8_lossy(&result.stdout)
            );
            let paths = String::from_utf8_lossy(&result.stdout)
                .lines()
                .filter_map(|line| reported_path(line).unwrap())
                .collect::<Vec<_>>();
            let target =
                finalize_download(true, &paths, &work, &s.0, "实际下载：绝望舞步", audio).unwrap();
            println!(
                "verified {} ({} bytes)",
                target.file_name().unwrap().to_string_lossy(),
                fs::metadata(&target).unwrap().len()
            );
            assert!(valid_media(&target, audio));
            assert_eq!(fs::metadata(&original).unwrap().len(), 2_000_000);
        }
    }
}

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackType {
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub index: usize,
    pub stream_index: usize,
    pub track_type: TrackType,
    pub codec_name: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    pub path: PathBuf,
    pub filename: String,
    pub duration_secs: f64,
    pub tracks: Vec<TrackInfo>,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Option<Vec<FfprobeStream>>,
    format: Option<FfprobeFormat>,
}

#[derive(Deserialize)]
struct FfprobeStream {
    index: usize,
    codec_type: String,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
}

#[derive(Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

pub fn probe_media_file(path: &Path) -> Result<MediaFile, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            path.to_str().ok_or("Invalid path string")?,
        ])
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "ffprobe exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: FfprobeOutput = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse ffprobe JSON output: {}", e))?;

    let duration_secs = parsed
        .format
        .as_ref()
        .and_then(|f| f.duration.as_ref())
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    let mut tracks = Vec::new();
    if let Some(streams) = parsed.streams {
        for s in streams {
            let codec_name = s.codec_name.unwrap_or_else(|| "unknown".to_string());
            if s.codec_type == "video" {
                let w = s.width.unwrap_or(0);
                let h = s.height.unwrap_or(0);
                let fps = s.r_frame_rate.unwrap_or_else(|| "".to_string());
                let detail = format!("{}x{} @ {} fps", w, h, fps);
                let track_idx = tracks.len();
                tracks.push(TrackInfo {
                    index: track_idx,
                    stream_index: s.index,
                    track_type: TrackType::Video,
                    codec_name,
                    detail,
                });
            } else if s.codec_type == "audio" {
                let sr = s.sample_rate.unwrap_or_else(|| "44100".to_string());
                let ch = s.channels.unwrap_or(2);
                let detail = format!("{} Hz, {} channels", sr, ch);
                let track_idx = tracks.len();
                tracks.push(TrackInfo {
                    index: track_idx,
                    stream_index: s.index,
                    track_type: TrackType::Audio,
                    codec_name,
                    detail,
                });
            }
        }
    }

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(MediaFile {
        path: path.to_path_buf(),
        filename,
        duration_secs,
        tracks,
    })
}

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct SignalPoint {
    pub time_ms: f64,
    pub raw_value: f64,
    pub normalized_value: f64,
}

#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub track_name: String,
    pub peak_time_ms: f64,
    #[allow(dead_code)]
    pub max_intensity: f64,
    pub confidence: f64,
    pub points: Vec<SignalPoint>,
}

/// Analyzes video track for the first 5 seconds to detect sharp visual changes (flash/flicker).
pub fn analyze_video_track(
    path: &Path,
    stream_index: usize,
    max_duration_secs: f64,
) -> Result<AnalysisResult, String> {
    let width = 32usize;
    let height = 18usize;
    let frame_bytes = width * height;
    let fps = 30.0f64;

    let duration_str = format!("{:.2}", max_duration_secs);

    let mut child = Command::new("ffmpeg")
        .args([
            "-ss",
            "0",
            "-t",
            &duration_str,
            "-i",
            path.to_str().ok_or("Invalid path")?,
            "-map",
            &format!("0:{}", stream_index),
            "-vf",
            &format!("scale={}:{},format=gray", width, height),
            "-r",
            &format!("{}", fps),
            "-f",
            "rawvideo",
            "-pix_fmt",
            "gray",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg for video analysis: {}", e))?;

    let mut stdout = child.stdout.take().ok_or("Failed to open ffmpeg stdout")?;
    let mut buffer = vec![0u8; frame_bytes];
    let mut prev_frame: Option<Vec<u8>> = None;
    let mut points = Vec::new();

    let mut max_val = 0.0f64;
    let mut peak_time_ms = 0.0f64;
    let mut frame_index = 0usize;

    while stdout.read_exact(&mut buffer).is_ok() {
        let time_ms = (frame_index as f64 / fps) * 1000.0;
        let delta = if let Some(ref prev) = prev_frame {
            let mut sum_diff = 0u64;
            for i in 0..frame_bytes {
                sum_diff += (buffer[i] as i16 - prev[i] as i16).abs() as u64;
            }
            sum_diff as f64 / frame_bytes as f64
        } else {
            0.0
        };

        if delta > max_val {
            max_val = delta;
            peak_time_ms = time_ms;
        }

        points.push(SignalPoint {
            time_ms,
            raw_value: delta,
            normalized_value: 0.0,
        });

        prev_frame = Some(buffer.clone());
        frame_index += 1;
    }

    let _ = child.wait();

    if points.is_empty() {
        return Err("No video frames extracted within the analysis window.".to_string());
    }

    // Normalize points between 0.0 and 1.0
    let max_raw = points
        .iter()
        .map(|p| p.raw_value)
        .fold(0.0f64, |a, b| a.max(b));

    if max_raw > 1e-6 {
        for p in &mut points {
            p.normalized_value = p.raw_value / max_raw;
        }
    }

    // Calculate confidence score (peak height vs mean background noise)
    let mean_val: f64 = points.iter().map(|p| p.raw_value).sum::<f64>() / points.len() as f64;
    let confidence = if mean_val > 1e-6 {
        (max_val / mean_val).min(10.0) / 10.0
    } else {
        0.0
    };

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Video".to_string());

    Ok(AnalysisResult {
        track_name: format!("Video: {}", filename),
        peak_time_ms,
        max_intensity: max_val,
        confidence,
        points,
    })
}

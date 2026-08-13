use super::video_detector::{AnalysisResult, SignalPoint};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

/// Analyzes audio track for the first 5 seconds to detect sudden transient / plosive sound bursts.
pub fn analyze_audio_track(
    path: &Path,
    stream_index: usize,
    max_duration_secs: f64,
) -> Result<AnalysisResult, String> {
    let sample_rate = 44100usize;
    let duration_str = format!("{:.2}", max_duration_secs);

    let mut child = Command::new(crate::utils::get_ffmpeg_path())
        .args([
            "-ss",
            "0",
            "-t",
            &duration_str,
            "-i",
            path.to_str().ok_or("Invalid path")?,
            "-map",
            &format!("0:{}", stream_index),
            "-ar",
            &format!("{}", sample_rate),
            "-ac",
            "1",
            "-f",
            "s16le",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg for audio analysis: {}", e))?;

    let mut stdout = child.stdout.take().ok_or("Failed to open ffmpeg stdout")?;
    let mut pcm_bytes = Vec::new();
    stdout
        .read_to_end(&mut pcm_bytes)
        .map_err(|e| format!("Failed to read PCM stream: {}", e))?;

    let _ = child.wait();

    if pcm_bytes.len() < 2 {
        return Err("No audio PCM data extracted within analysis window.".to_string());
    }

    let samples: Vec<i16> = pcm_bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    // 10ms frame windows = 441 samples at 44.1kHz
    let window_size = 441usize;
    let mut energies = Vec::new();
    let mut points = Vec::new();

    let mut prev_energy = 0.0f64;
    let mut max_onset = 0.0f64;
    let mut peak_time_ms = 0.0f64;

    for (win_idx, window) in samples.chunks(window_size).enumerate() {
        let time_ms = (win_idx * window_size) as f64 / sample_rate as f64 * 1000.0;
        let sum_sq: f64 = window.iter().map(|&s| (s as f64 / 32768.0).powi(2)).sum();
        let rms = (sum_sq / window.len() as f64).sqrt();

        // Onset strength = positive energy derivative
        let onset = (rms - prev_energy).max(0.0);
        energies.push(rms);

        if onset > max_onset {
            max_onset = onset;
            peak_time_ms = time_ms;
        }

        points.push(SignalPoint {
            time_ms,
            raw_value: onset,
            normalized_value: 0.0,
        });

        prev_energy = rms;
    }

    if points.is_empty() {
        return Err("No audio frames generated.".to_string());
    }

    // Normalize values
    let max_raw = points
        .iter()
        .map(|p| p.raw_value)
        .fold(0.0f64, |a, b| a.max(b));

    if max_raw > 1e-6 {
        for p in &mut points {
            p.normalized_value = p.raw_value / max_raw;
        }
    }

    // Confidence score
    let mean_onset: f64 = points.iter().map(|p| p.raw_value).sum::<f64>() / points.len() as f64;
    let confidence = if mean_onset > 1e-6 {
        (max_onset / mean_onset).min(10.0) / 10.0
    } else {
        0.0
    };

    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Audio".to_string());

    Ok(AnalysisResult {
        track_name: format!("Audio: {}", filename),
        peak_time_ms,
        max_intensity: max_onset,
        confidence,
        points,
    })
}

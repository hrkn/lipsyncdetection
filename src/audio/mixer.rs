use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tempfile::NamedTempFile;


pub struct MixPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink1: Option<Sink>,
    sink2: Option<Sink>,
    wav1_path: Option<PathBuf>,
    wav2_path: Option<PathBuf>,
    pub is_playing: bool,
    pub current_offset_ms: f64,
    pub volume1: f32,
    pub volume2: f32,
    pub mute1: bool,
    pub mute2: bool,
}

impl Default for MixPlayer {
    fn default() -> Self {
        let (stream, handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(_) => (None, None),
        };
        Self {
            _stream: stream,
            stream_handle: handle,
            sink1: None,
            sink2: None,
            wav1_path: None,
            wav2_path: None,
            is_playing: false,
            current_offset_ms: 0.0,
            volume1: 1.0,
            volume2: 1.0,
            mute1: false,
            mute2: false,
        }
    }
}

impl MixPlayer {
    pub fn prepare_track(&mut self, is_track2: bool, media_path: &Path, stream_index: usize) -> Result<(), String> {
        let temp_wav = NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;
        let temp_path = temp_wav.path().with_extension("wav");

        let mut cmd = Command::new(crate::utils::get_ffmpeg_path());
        cmd.args([
            "-y",
            "-ss",
            "0",
            "-t",
            "30", // Prepare first 30s for preview mix
            "-i",
            media_path.to_str().ok_or("Invalid path")?,
            "-map",
            &format!("0:{}", stream_index),
            "-ar",
            "44100",
            "-ac",
            "2",
            "-f",
            "wav",
            temp_path.to_str().ok_or("Invalid temp path")?,
        ]);

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to extract audio with ffmpeg: {}", e))?;

        if !status.success() {
            return Err("FFmpeg audio extraction failed.".to_string());
        }

        // Keep temp file path alive
        let persistent_path = temp_path.clone();
        // Prevent tempfile auto-delete on handle drop
        let _ = temp_wav.keep();

        if !is_track2 {
            self.wav1_path = Some(persistent_path);
        } else {
            self.wav2_path = Some(persistent_path);
        }

        Ok(())
    }

    pub fn play(&mut self, offset_ms: f64) -> Result<(), String> {
        self.stop();
        self.current_offset_ms = offset_ms;

        let handle = self
            .stream_handle
            .as_ref()
            .ok_or("No audio output device available.")?;

        if let Some(ref path1) = self.wav1_path {
            if let Ok(file) = File::open(path1) {
                if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
                    if let Ok(sink) = Sink::try_new(handle) {
                        let vol = if self.mute1 { 0.0 } else { self.volume1 };
                        sink.set_volume(vol);

                        if offset_ms < 0.0 {
                            let delay = Duration::from_secs_f64(-offset_ms / 1000.0);
                            sink.append(decoder.delay(delay));
                        } else {
                            sink.append(decoder);
                        }

                        self.sink1 = Some(sink);
                    }
                }
            }
        }

        if let Some(ref path2) = self.wav2_path {
            if let Ok(file) = File::open(path2) {
                if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
                    if let Ok(sink) = Sink::try_new(handle) {
                        let vol = if self.mute2 { 0.0 } else { self.volume2 };
                        sink.set_volume(vol);

                        if offset_ms > 0.0 {
                            let delay = Duration::from_secs_f64(offset_ms / 1000.0);
                            sink.append(decoder.delay(delay));
                        } else {
                            sink.append(decoder);
                        }

                        self.sink2 = Some(sink);
                    }
                }
            }
        }

        self.is_playing = true;
        Ok(())
    }

    pub fn pause(&mut self) {
        if let Some(ref s) = self.sink1 {
            s.pause();
        }
        if let Some(ref s) = self.sink2 {
            s.pause();
        }
        self.is_playing = false;
    }

    #[allow(dead_code)]
    pub fn resume(&mut self) {
        if let Some(ref s) = self.sink1 {
            s.play();
        }
        if let Some(ref s) = self.sink2 {
            s.play();
        }
        self.is_playing = true;
    }

    pub fn stop(&mut self) {
        if let Some(ref s) = self.sink1 {
            s.stop();
        }
        if let Some(ref s) = self.sink2 {
            s.stop();
        }
        self.sink1 = None;
        self.sink2 = None;
        self.is_playing = false;
    }

    pub fn update_volumes(&mut self) {
        if let Some(ref s) = self.sink1 {
            let vol = if self.mute1 { 0.0 } else { self.volume1 };
            s.set_volume(vol);
        }
        if let Some(ref s) = self.sink2 {
            let vol = if self.mute2 { 0.0 } else { self.volume2 };
            s.set_volume(vol);
        }
    }
}

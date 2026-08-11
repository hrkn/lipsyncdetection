use crate::analysis::{
    analyze_audio_track, analyze_video_track, compute_sync_offset, AnalysisResult, SyncResult,
};
use crate::audio::MixPlayer;
use crate::media::{probe_media_file, MediaFile, TrackType};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints, VLine};

pub struct SyncDetectorApp {
    loaded_files: Vec<MediaFile>,
    track1_selection: Option<(usize, usize)>, // (file_idx, track_idx)
    track2_selection: Option<(usize, usize)>, // (file_idx, track_idx)
    analysis1: Option<AnalysisResult>,
    analysis2: Option<AnalysisResult>,
    sync_result: Option<SyncResult>,
    manual_offset_ms: f64,
    status_message: String,
    is_analyzing: bool,
    mixer: MixPlayer,
}

impl Default for SyncDetectorApp {
    fn default() -> Self {
        Self {
            loaded_files: Vec::new(),
            track1_selection: None,
            track2_selection: None,
            analysis1: None,
            analysis2: None,
            sync_result: None,
            manual_offset_ms: 0.0,
            status_message: "メディアファイルを追加してください。".to_string(),
            is_analyzing: false,
            mixer: MixPlayer::default(),
        }
    }
}

fn setup_japanese_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let font_candidates = [
        r"C:\Windows\Fonts\meiryo.ttc",
        r"C:\Windows\Fonts\BIZ-UDGothicR.ttc",
        r"C:\Windows\Fonts\YuGothM.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
    ];

    for path in &font_candidates {
        if let Ok(font_bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "japanese_font".to_string(),
                egui::FontData::from_owned(font_bytes),
            );

            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "japanese_font".to_string());

            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("japanese_font".to_string());

            ctx.set_fonts(fonts);
            return;
        }
    }
}

impl SyncDetectorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_japanese_fonts(&cc.egui_ctx);
        Self::default()
    }

    fn open_file_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter("Media Files", &["mp4", "mkv", "mov", "avi", "webm", "wav", "mp3", "aac", "m4a", "flac"])
            .pick_files()
        {
            for path in paths {
                match probe_media_file(&path) {
                    Ok(media) => {
                        let file_idx = self.loaded_files.len();
                        if self.track1_selection.is_none() && !media.tracks.is_empty() {
                            self.track1_selection = Some((file_idx, 0));
                        } else if self.track2_selection.is_none() && !media.tracks.is_empty() {
                            self.track2_selection = Some((file_idx, 0));
                        }
                        self.loaded_files.push(media);
                        self.status_message = format!("ファイルを読み込みました: {:?}", path.file_name().unwrap_or_default());
                    }
                    Err(e) => {
                        self.status_message = format!("エラー: {}", e);
                    }
                }
            }
        }
    }

    fn run_auto_detection(&mut self) {
        let (f1_idx, t1_idx) = match self.track1_selection {
            Some(sel) => sel,
            None => {
                self.status_message = "トラック1が選択されていません。".to_string();
                return;
            }
        };

        let (f2_idx, t2_idx) = match self.track2_selection {
            Some(sel) => sel,
            None => {
                self.status_message = "トラック2が選択されていません。".to_string();
                return;
            }
        };

        let file1 = &self.loaded_files[f1_idx];
        let track1 = &file1.tracks[t1_idx];

        let file2 = &self.loaded_files[f2_idx];
        let track2 = &file2.tracks[t2_idx];

        self.status_message = "先頭5秒間の分析を実行中...".to_string();
        self.is_analyzing = true;

        let res1 = match track1.track_type {
            TrackType::Video => analyze_video_track(&file1.path, track1.stream_index, 5.0),
            TrackType::Audio => analyze_audio_track(&file1.path, track1.stream_index, 5.0),
        };

        let res2 = match track2.track_type {
            TrackType::Video => analyze_video_track(&file2.path, track2.stream_index, 5.0),
            TrackType::Audio => analyze_audio_track(&file2.path, track2.stream_index, 5.0),
        };

        self.is_analyzing = false;

        match (res1, res2) {
            (Ok(r1), Ok(r2)) => {
                let sync = compute_sync_offset(&r1, &r2);
                self.manual_offset_ms = sync.recommended_offset_ms;
                self.sync_result = Some(sync);
                self.analysis1 = Some(r1);
                self.analysis2 = Some(r2);
                self.status_message = "自動検出が完了しました！".to_string();

                // Prepare audio playback if tracks have audio stream available
                let _ = self.mixer.prepare_track(false, &file1.path, track1.stream_index);
                let _ = self.mixer.prepare_track(true, &file2.path, track2.stream_index);
            }
            (Err(e1), _) => {
                self.status_message = format!("トラック1の解析に失敗しました: {}", e1);
            }
            (_, Err(e2)) => {
                self.status_message = format!("トラック2の解析に失敗しました: {}", e2);
            }
        }
    }
}

impl eframe::App for SyncDetectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply custom dark theme visual style
        ctx.set_visuals(egui::Visuals::dark());

        egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("🎥🎵 Lip Sync & Media Sync Detector");
                ui.separator();
                if ui.button("📂 メディアファイルを開く").clicked() {
                    self.open_file_dialog();
                }
                if ui.button("🗑 クリア").clicked() {
                    self.loaded_files.clear();
                    self.track1_selection = None;
                    self.track2_selection = None;
                    self.analysis1 = None;
                    self.analysis2 = None;
                    self.sync_result = None;
                    self.mixer.stop();
                    self.status_message = "ファイルをクリアしました。".to_string();
                }
            });
            ui.add_space(6.0);
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("状態:");
                ui.colored_label(egui::Color32::KHAKI, &self.status_message);
            });
        });

        // Left Side Panel: File list & Track selector
        egui::SidePanel::left("left_track_selector")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("トラック選択");
                ui.separator();

                if self.loaded_files.is_empty() {
                    ui.label("「メディアファイルを開く」ボタンから比較したいファイルを追加してください。");
                } else {
                    ui.label("■ 基準トラック (Track 1):");
                    for (f_idx, file) in self.loaded_files.iter().enumerate() {
                        for (t_idx, track) in file.tracks.iter().enumerate() {
                            let icon = match track.track_type {
                                TrackType::Video => "📹",
                                TrackType::Audio => "🔊",
                            };
                            let label = format!("{} [{}] {}", icon, file.filename, track.detail);
                            let is_selected = self.track1_selection == Some((f_idx, t_idx));
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.track1_selection = Some((f_idx, t_idx));
                            }
                        }
                    }

                    ui.add_space(10.0);
                    ui.label("■ 対象トラック (Track 2):");
                    for (f_idx, file) in self.loaded_files.iter().enumerate() {
                        for (t_idx, track) in file.tracks.iter().enumerate() {
                            let icon = match track.track_type {
                                TrackType::Video => "📹",
                                TrackType::Audio => "🔊",
                            };
                            let label = format!("{} [{}] {}", icon, file.filename, track.detail);
                            let is_selected = self.track2_selection == Some((f_idx, t_idx));
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.track2_selection = Some((f_idx, t_idx));
                            }
                        }
                    }

                    ui.add_space(15.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 36.0],
                            egui::Button::new("⚡ 先頭5秒 自動同期検出").fill(egui::Color32::from_rgb(40, 100, 180)),
                        )
                        .clicked()
                    {
                        self.run_auto_detection();
                    }
                }
            });

        // Bottom Panel: Manual Adjustment & Audio Mix Player
        egui::TopBottomPanel::bottom("bottom_player_controls")
            .resizable(true)
            .default_height(180.0)
            .show(ctx, |ui| {
                ui.heading("手動遅延調整 & ミックス再生比較");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label("遅れ時間 (ミリ秒):");
                            let drag_res = ui.add(
                                egui::DragValue::new(&mut self.manual_offset_ms)
                                    .speed(1.0)
                                    .suffix(" ms"),
                            );

                            if drag_res.hovered() {
                                let (scroll_val, is_shift, is_ctrl) = ui.input(|i| {
                                    let delta = if i.raw_scroll_delta.y != 0.0 {
                                        i.raw_scroll_delta.y
                                    } else {
                                        i.raw_scroll_delta.x
                                    };
                                    (delta, i.modifiers.shift, i.modifiers.ctrl || i.modifiers.command)
                                });

                                if scroll_val != 0.0 {
                                    let step = if is_shift {
                                        100.0
                                    } else if is_ctrl {
                                        1.0
                                    } else {
                                        10.0
                                    };
                                    if scroll_val > 0.0 {
                                        self.manual_offset_ms += step;
                                    } else {
                                        self.manual_offset_ms -= step;
                                    }
                                }
                            }

                            if let Some(ref sync) = self.sync_result {
                                if ui.button("✨ 自動検出推奨値に合わせる").clicked() {
                                    self.manual_offset_ms = sync.recommended_offset_ms;
                                }
                            }
                        });
                    });

                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label("精密微調整 (ホイール/Shift+ホイール/Ctrl+ホイール対応):");
                            ui.horizontal(|ui| {
                                if ui.button("-100ms").clicked() { self.manual_offset_ms -= 100.0; }
                                if ui.button("-10ms").clicked() { self.manual_offset_ms -= 10.0; }
                                if ui.button("-1ms").clicked() { self.manual_offset_ms -= 1.0; }
                                if ui.button("+1ms").clicked() { self.manual_offset_ms += 1.0; }
                                if ui.button("+10ms").clicked() { self.manual_offset_ms += 10.0; }
                                if ui.button("+100ms").clicked() { self.manual_offset_ms += 100.0; }
                            });
                            let slider_res = ui.add(
                                egui::Slider::new(&mut self.manual_offset_ms, -5000.0..=5000.0)
                                    .text("オフセットスライダー (ms)"),
                            );

                            if slider_res.hovered() {
                                let (scroll_val, is_shift, is_ctrl) = ui.input(|i| {
                                    let delta = if i.raw_scroll_delta.y != 0.0 {
                                        i.raw_scroll_delta.y
                                    } else {
                                        i.raw_scroll_delta.x
                                    };
                                    (delta, i.modifiers.shift, i.modifiers.ctrl || i.modifiers.command)
                                });

                                if scroll_val != 0.0 {
                                    let step = if is_shift {
                                        100.0
                                    } else if is_ctrl {
                                        1.0
                                    } else {
                                        10.0
                                    };
                                    if scroll_val > 0.0 {
                                        self.manual_offset_ms += step;
                                    } else {
                                        self.manual_offset_ms -= step;
                                    }
                                    self.manual_offset_ms = self.manual_offset_ms.clamp(-5000.0, 5000.0);
                                }
                            }
                        });
                    });

                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label("ミックス再生コントロール:");
                            ui.horizontal(|ui| {
                                if self.mixer.is_playing {
                                    if ui.button("⏸ 一時停止").clicked() {
                                        self.mixer.pause();
                                    }
                                } else {
                                    if ui.button("▶ ミックス再生").clicked() {
                                        if let Err(e) = self.mixer.play(self.manual_offset_ms) {
                                            self.status_message = format!("再生エラー: {}", e);
                                        }
                                    }
                                }
                                if ui.button("⏹ 停止").clicked() {
                                    self.mixer.stop();
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.mixer.mute1, "Track 1 Mute");
                                ui.add(egui::Slider::new(&mut self.mixer.volume1, 0.0..=2.0).text("T1 Vol"));
                            });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.mixer.mute2, "Track 2 Mute");
                                ui.add(egui::Slider::new(&mut self.mixer.volume2, 0.0..=2.0).text("T2 Vol"));
                            });
                            self.mixer.update_volumes();
                        });
                    });
                });
            });

        // Central Panel: Signal graphs and detection result summary
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("検出結果 & 変化量波形グラフ (先頭 0.0s - 5.0s)");
            ui.separator();

            if let Some(ref sync) = self.sync_result {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("📍 Track 1 ピーク: {:.1} ms", sync.track1_peak_ms));
                        ui.separator();
                        ui.label(format!("📍 Track 2 ピーク: {:.1} ms", sync.track2_peak_ms));
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::LIGHT_GREEN,
                            format!("⏱ 推奨オフセット: {:+.1} ms (Track 2をシフト)", sync.recommended_offset_ms),
                        );
                        ui.separator();
                        ui.label(format!("🎯 ピーク検出信頼度: {:.1}%", sync.confidence_percentage));
                    });
                });
                ui.add_space(6.0);
            }

            if self.analysis1.is_none() && self.analysis2.is_none() {
                ui.centered_and_justified(|ui| {
                    ui.label("トラックを選択し、「先頭5秒 自動同期検出」を実行すると、ここに変化量グラフが表示されます。");
                });
            } else {
                Plot::new("signal_comparison_plot")
                    .view_aspect(2.5)
                    .legend(egui_plot::Legend::default())
                    .show(ui, |plot_ui| {
                        if let Some(ref a1) = self.analysis1 {
                            let points: PlotPoints = a1
                                .points
                                .iter()
                                .map(|p| [p.time_ms, p.normalized_value])
                                .collect();
                            plot_ui.line(Line::new(points).name(&a1.track_name).color(egui::Color32::LIGHT_BLUE));
                            plot_ui.vline(VLine::new(a1.peak_time_ms).color(egui::Color32::BLUE).name("T1 Peak"));
                        }

                        if let Some(ref a2) = self.analysis2 {
                            let points: PlotPoints = a2
                                .points
                                .iter()
                                .map(|p| [p.time_ms, p.normalized_value])
                                .collect();
                            plot_ui.line(Line::new(points).name(&a2.track_name).color(egui::Color32::GOLD));
                            plot_ui.vline(VLine::new(a2.peak_time_ms).color(egui::Color32::YELLOW).name("T2 Peak"));
                        }
                    });
            }
        });
    }
}

use super::video_detector::AnalysisResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub track1_peak_ms: f64,
    pub track2_peak_ms: f64,
    /// Offset (in milliseconds) to apply to Track 2 relative to Track 1 so that their peak points align.
    /// Recommended offset: Delta = T_Track1 - T_Track2.
    pub recommended_offset_ms: f64,
    pub confidence_percentage: f64,
    pub track1_name: String,
    pub track2_name: String,
}

pub fn compute_sync_offset(
    result1: &AnalysisResult,
    result2: &AnalysisResult,
) -> SyncResult {
    let t1 = result1.peak_time_ms;
    let t2 = result2.peak_time_ms;
    let recommended_offset_ms = t1 - t2;

    let avg_confidence = (result1.confidence + result2.confidence) / 2.0;
    let confidence_percentage = (avg_confidence * 100.0).clamp(0.0, 100.0);

    SyncResult {
        track1_peak_ms: t1,
        track2_peak_ms: t2,
        recommended_offset_ms,
        confidence_percentage,
        track1_name: result1.track_name.clone(),
        track2_name: result2.track_name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::video_detector::SignalPoint;

    #[test]
    fn test_compute_sync_offset() {
        let r1 = AnalysisResult {
            track_name: "Track A".to_string(),
            peak_time_ms: 1200.0,
            max_intensity: 0.9,
            confidence: 0.85,
            points: vec![SignalPoint {
                time_ms: 1200.0,
                raw_value: 0.9,
                normalized_value: 1.0,
            }],
        };

        let r2 = AnalysisResult {
            track_name: "Track B".to_string(),
            peak_time_ms: 1550.0,
            max_intensity: 0.8,
            confidence: 0.90,
            points: vec![SignalPoint {
                time_ms: 1550.0,
                raw_value: 0.8,
                normalized_value: 1.0,
            }],
        };

        let sync = compute_sync_offset(&r1, &r2);
        assert_eq!(sync.track1_peak_ms, 1200.0);
        assert_eq!(sync.track2_peak_ms, 1550.0);
        assert_eq!(sync.recommended_offset_ms, -350.0);
        assert!((sync.confidence_percentage - 87.5).abs() < 1e-3);
    }
}

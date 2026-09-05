//! Portable result viewers. Report values are displayed without recomputing inference.

use crate::{io::write_text, study::StudyReport};
use anyhow::{Result, bail};
use std::path::Path;

/// Renders an offline viewer with JSON embedded as inert, script-safe data.
/// Each analysis remains a separate dataset/track, including when evidence differs.
///
/// # Errors
/// Rejects empty/unsupported analyses, missing observations, or invalid intervals.
pub fn render_html(reports: &[StudyReport]) -> Result<String> {
    if reports.is_empty() {
        bail!("at least one validated analysis is required");
    }
    for report in reports {
        if report.schema_version != 2 || report.models.is_empty() {
            bail!("visualization requires a v0.2 analysis containing model observations");
        }
        if !crate::pilot::TRACKS.contains(&report.track.as_str())
            || !["synthetic_fixture", "live_provider"].contains(&report.evidence.as_str())
        {
            bail!("unknown analysis track or evidence type");
        }
        for model in report.models.values() {
            if model.repetitions == 0 || model.completed_cases_per_run == 0 {
                bail!("analysis contains no completed observations");
            }
            for interval in [&model.exact_text, &model.exact_words] {
                if interval.clusters == 0
                    || interval.resamples < 100
                    || interval.lower > interval.upper
                    || [interval.estimate, interval.lower, interval.upper]
                        .iter()
                        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                {
                    bail!("analysis contains an invalid recall interval");
                }
            }
        }
    }
    let payload = serde_json::to_string(reports)?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(include_str!("viewer.html").replacen("__BQB_REPORT_DATA__", &payload, 1))
}

/// Writes a self-contained HTML file; no external assets or network calls are used.
///
/// # Errors
/// Returns validation, serialization, or filesystem errors.
pub fn write_html(path: &Path, reports: &[StudyReport]) -> Result<()> {
    let html = render_html(reports)?;
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    write_text(path, &html)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> StudyReport {
        serde_json::from_str(include_str!("../docs/pilot/v0.2/canonical/analysis.json")).unwrap()
    }
    #[test]
    fn embeds_reports_without_external_assets_and_escapes_script_terminators() {
        let mut report = fixture();
        let attack = "</script><script>alert('unsafe')</script>&\u{2028}\u{2029}";
        report.examples[0].output = attack.into();
        report.notes.push(attack.into());
        let html = render_html(&[report]).unwrap();
        assert!(!html.contains(attack));
        assert!(html.contains("\\u003c/script\\u003e"));
        assert!(!html.contains("__BQB_REPORT_DATA__"));
        assert!(!html.contains("<script src="));
        assert!(html.contains("connect-src 'none'"));
        assert!(html.contains("id=\"report-data\""));
        let data = html
            .split("<script id=\"report-data\" type=\"application/json\">")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        let decoded: Vec<StudyReport> = serde_json::from_str(data).unwrap();
        assert_eq!(decoded[0].examples[0].output, attack);
    }
    #[test]
    fn rejects_invalid_analysis_instead_of_drawing_misleading_charts() {
        assert!(render_html(&[]).is_err());
        for mutation in 0..7 {
            let mut report = fixture();
            match mutation {
                0 => report.schema_version = 99,
                1 => report.models.clear(),
                2 => report.track = "mixed".into(),
                3 => report.evidence = "unknown".into(),
                4 => report.models.values_mut().next().unwrap().exact_text.upper = 2.0,
                5 => report.models.values_mut().next().unwrap().repetitions = 0,
                _ => {
                    report
                        .models
                        .values_mut()
                        .next()
                        .unwrap()
                        .exact_words
                        .clusters = 0;
                }
            }
            assert!(render_html(&[report]).is_err());
        }
    }
    #[test]
    fn bundles_tracks_without_pooling_and_writes_deterministically() {
        let report = fixture();
        let mut copy = fixture();
        copy.track = "copy_control".into();
        let reports = [report, copy];
        let first = render_html(&reports).unwrap();
        assert_eq!(first, render_html(&reports).unwrap());
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("nested/report.html");
        write_html(&path, &reports).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), first);
    }
}

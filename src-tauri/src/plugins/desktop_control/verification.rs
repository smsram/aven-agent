use serde::{Deserialize, Serialize};

/// Task completion outcome classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOutcome {
    Successful,
    Failed,
    Unverified,
    PartiallyCompleted,
    Cancelled,
}

impl TaskOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskOutcome::Successful => "successful",
            TaskOutcome::Failed => "failed",
            TaskOutcome::Unverified => "unverified",
            TaskOutcome::PartiallyCompleted => "partially_completed",
            TaskOutcome::Cancelled => "cancelled",
        }
    }
}

/// The category of evidence used to confirm state change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Uia,
    Visual,
    System,
    None,
}

/// Comprehensive verification result for a desktop action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionVerificationResult {
    pub action_type: String,
    pub verified: bool,
    pub evidence_kind: EvidenceKind,
    pub expected_state: Option<String>,
    pub actual_state: Option<String>,
    pub diagnostics: Option<String>,
    pub retry_count: usize,
    pub max_retries: usize,
    pub can_retry: bool,
}

impl ActionVerificationResult {
    pub fn success(
        action_type: &str,
        evidence: EvidenceKind,
        expected: Option<String>,
        actual: Option<String>,
    ) -> Self {
        Self {
            action_type: action_type.to_string(),
            verified: true,
            evidence_kind: evidence,
            expected_state: expected,
            actual_state: actual,
            diagnostics: None,
            retry_count: 0,
            max_retries: 2,
            can_retry: false,
        }
    }

    pub fn failure(
        action_type: &str,
        expected: Option<String>,
        actual: Option<String>,
        diagnostics: &str,
        retry_count: usize,
        max_retries: usize,
    ) -> Self {
        Self {
            action_type: action_type.to_string(),
            verified: false,
            evidence_kind: EvidenceKind::None,
            expected_state: expected,
            actual_state: actual,
            diagnostics: Some(diagnostics.to_string()),
            retry_count,
            max_retries,
            can_retry: retry_count < max_retries,
        }
    }

    pub fn unverified(action_type: &str, reason: &str) -> Self {
        Self {
            action_type: action_type.to_string(),
            verified: false,
            evidence_kind: EvidenceKind::None,
            expected_state: None,
            actual_state: None,
            diagnostics: Some(format!("Action executed but state could not be independently verified: {reason}")),
            retry_count: 0,
            max_retries: 2,
            can_retry: false,
        }
    }
}

/// Baseline state captured prior to executing an action
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionBaseline {
    pub window_id: Option<i64>,
    pub window_title: Option<String>,
    pub focused_control_name: Option<String>,
    pub focused_control_type: Option<String>,
    pub existing_text: Option<String>,
    pub caret_position: Option<usize>,
    pub scroll_vertical_percent: Option<f64>,
    pub screenshot_file: Option<String>,
}

/// Diagnoses why an action failed to produce the expected change
pub fn diagnose_failure(
    action: &str,
    baseline: &ActionBaseline,
    post_hwnd: Option<i64>,
    post_text: Option<&str>,
    has_focus: bool,
    is_read_only: bool,
) -> String {
    let mut reasons = Vec::new();

    if is_read_only {
        reasons.push("Target control is marked as Read-Only in UI Automation".to_string());
    }

    if !has_focus {
        reasons.push("Target control lost keyboard focus during or before execution".to_string());
    }

    if let (Some(b_hwnd), Some(p_hwnd)) = (baseline.window_id, post_hwnd) {
        if b_hwnd != p_hwnd {
            reasons.push(format!(
                "Active foreground window shifted from HWND {b_hwnd} to {p_hwnd}"
            ));
        }
    }

    if action == "type_text" || action == "keyboard_action" {
        if let (Some(pre), Some(post)) = (&baseline.existing_text, post_text) {
            if pre == post {
                reasons.push("Target text content did not change from pre-action baseline".to_string());
            }
        }
    }

    if reasons.is_empty() {
        format!("Action '{action}' did not produce verifiable UI state changes. Application may still be processing, dialog may be modal, or input was dropped.")
    } else {
        format!("Diagnosis for failed '{}': {}", action, reasons.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_success_and_failure() {
        let succ = ActionVerificationResult::success(
            "type_text",
            EvidenceKind::Uia,
            Some("Hello World".to_string()),
            Some("Hello World".to_string()),
        );
        assert!(succ.verified);
        assert_eq!(succ.evidence_kind, EvidenceKind::Uia);
        assert!(!succ.can_retry);

        let fail = ActionVerificationResult::failure(
            "type_text",
            Some("Hello World".to_string()),
            Some("".to_string()),
            "Input dropped",
            1,
            2,
        );
        assert!(!fail.verified);
        assert!(fail.can_retry);
        assert_eq!(fail.retry_count, 1);
    }

    #[test]
    fn test_diagnosis_generation() {
        let baseline = ActionBaseline {
            window_id: Some(100),
            existing_text: Some("Initial".to_string()),
            ..Default::default()
        };

        let diag = diagnose_failure("type_text", &baseline, Some(200), Some("Initial"), false, true);
        assert!(diag.contains("Read-Only"));
        assert!(diag.contains("lost keyboard focus"));
        assert!(diag.contains("shifted from HWND 100 to 200"));
    }
}

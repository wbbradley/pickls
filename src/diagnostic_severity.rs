use crate::prelude::*;

pub struct LintLsDiagnosticSeverity {
    pub severity: String,
}

impl From<LintLsDiagnosticSeverity> for DiagnosticSeverity {
    fn from(diag_sev: LintLsDiagnosticSeverity) -> Self {
        match diag_sev.severity.to_lowercase().as_str() {
            "error" => DiagnosticSeverity::ERROR,
            "warn" | "warning" => DiagnosticSeverity::WARNING,
            "hint" => DiagnosticSeverity::HINT,
            "note" => DiagnosticSeverity::INFORMATION,
            "info" => DiagnosticSeverity::INFORMATION,
            "information" => DiagnosticSeverity::INFORMATION,
            _ => DiagnosticSeverity::ERROR,
        }
    }
}

use crate::prelude::*;

pub struct PicklsDiagnostic {
    pub linter: String,
    pub filename: String,
    pub line: u32,
    pub start_column: Option<u32>,
    pub end_column: Option<u32>,
    pub severity: Option<PicklsDiagnosticSeverity>,
    pub description: Option<String>,
}

impl From<PicklsDiagnostic> for Diagnostic {
    fn from(diag: PicklsDiagnostic) -> Self {
        let line = diag.line.saturating_sub(1);
        let start_column = diag.start_column.unwrap_or(1).saturating_sub(1);
        let end_column = diag.end_column.unwrap_or(start_column + 1);

        let range = Range {
            start: Position {
                line,
                character: start_column,
            },
            end: Position {
                line,
                character: end_column,
            },
        };

        Self {
            range,
            severity: diag.severity.map(DiagnosticSeverity::from),
            code: None,
            code_description: None,
            source: Some(format!("[pickls/{}]", diag.linter)),
            message: diag.description.unwrap_or_else(|| "error".to_string()),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

use crate::prelude::*;

pub struct LintLsDiagnostic {
    pub line: u32,
    pub description: Option<String>,
}

impl From<LintLsDiagnostic> for Diagnostic {
    fn from(diagnostic: LintLsDiagnostic) -> Self {
        Self {
            range: Range {
                start: Position {
                    line: diagnostic.line.saturating_sub(1),
                    character: 0,
                },
                end: Position {
                    line: diagnostic.line.saturating_sub(1),
                    character: 0,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("lintls".to_string()),
            message: diagnostic
                .description
                .clone()
                .unwrap_or_else(|| "error".to_string()),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

// src/diagnostic.rs

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message:  String,
    pub file:     String,
    pub line:     usize,
    pub col:      usize,
    pub len:      usize,          // how many chars to underline (0 = just ^)
    pub hint:     Option<String>, // "help: add `;`"
    pub notes:    Vec<String>,    // secondary context lines
}

impl Diagnostic {
    pub fn error(msg: impl Into<String>, file: &str, line: usize, col: usize) -> Self {
        Self {
            severity: Severity::Error,
            message:  msg.into(),
            file:     file.to_string(),
            line, col,
            len:   0,
            hint:  None,
            notes: Vec::new(),
        }
    }

    pub fn warning(msg: impl Into<String>, file: &str, line: usize, col: usize) -> Self {
        Self { severity: Severity::Warning, ..Self::error(msg, file, line, col) }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_len(mut self, len: usize) -> Self {
        self.len = len;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Render rustc-style with source lines for display.
    pub fn render(&self, source: &str) -> String {
        let src_lines: Vec<&str> = source.lines().collect();
        let src_line  = src_lines.get(self.line.saturating_sub(1)).copied().unwrap_or("");
        let gutter    = self.line.to_string().len();
        let pad       = " ".repeat(gutter);
        let caret_off = " ".repeat(self.col.saturating_sub(1));
        let underline = if self.len > 1 {
            "^".repeat(self.len)
        } else {
            "^".to_string()
        };
        let help = self.hint.as_deref()
            .map(|h| format!(" help: {h}"))
            .unwrap_or_default();

        let severity_label = match self.severity {
            Severity::Error   => "error",
            Severity::Warning => "warning",
            Severity::Note    => "note",
        };

        let mut out = format!(
            "{severity_label}: {}\n \
            --> {}:{}:{}\n\
            {pad} |\n\
            {} | {src_line}\n\
            {pad} | {caret_off}{underline}{help}",
            self.message, self.file, self.line, self.col, self.line,
        );

        for note in &self.notes {
            out.push_str(&format!("\n{pad} = note: {note}"));
        }

        out
    }
}

/// Shared bag passed through every compiler stage.
/// Each stage pushes into it; compiler.rs reads it at the end.
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, d: Diagnostic) {
        self.diagnostics.push(d);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    /// Print all diagnostics to stderr, using source for highlighting.
    pub fn emit_all(&self, source: &str) {
        for d in &self.diagnostics {
            eprintln!("{}\n", d.render(source));
        }
        if self.has_errors() {
            let n = self.error_count();
            eprintln!("aborting due to {} error{}", n, if n == 1 { "" } else { "s" });
        }
    }
}
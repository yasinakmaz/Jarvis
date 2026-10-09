//! Kapı bulgularının toplanması ve raporlanması.

use anyhow::bail;

/// Bir kapının ürettiği bulgular. Hata varsa kapı kırmızıdır; uyarı ve notlar yazdırılır.
#[derive(Debug, Default)]
pub struct Findings {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub notes: Vec<String>,
}

impl Findings {
    pub fn error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn note(&mut self, message: impl Into<String>) {
        self.notes.push(message.into());
    }

    pub fn merge(&mut self, other: Self) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.notes.extend(other.notes);
    }

    /// Notları ve uyarıları yazdırır; hata varsa hepsini listeleyen bir hata döner.
    pub fn finish(self, gate: &str) -> anyhow::Result<()> {
        for note in &self.notes {
            eprintln!("    not: {note}");
        }
        for warning in &self.warnings {
            eprintln!("    uyarı: {warning}");
        }
        if self.errors.is_empty() {
            return Ok(());
        }
        for error in &self.errors {
            eprintln!("    HATA: {error}");
        }
        bail!("{gate}: {} ihlal", self.errors.len())
    }
}

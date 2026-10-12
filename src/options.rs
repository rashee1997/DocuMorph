use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfStandard {
    Standard,
    PdfA1a, // ISO 19005-1 compliance
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseTransform {
    Original,
    Lowercase,
    Uppercase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputDestination {
    SourceDirectory,
    Custom(PathBuf),
}

#[derive(Debug, Clone)]
pub struct NamingOptions {
    pub pattern: String,
    pub prefix: String,
    pub suffix: String,
    pub case_transform: CaseTransform,
    pub safe_collision: bool,
}

impl Default for NamingOptions {
    fn default() -> Self {
        NamingOptions {
            pattern: "{name}".to_string(),
            prefix: String::new(),
            suffix: String::new(),
            case_transform: CaseTransform::Original,
            safe_collision: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversionConfig {
    pub format: PdfStandard,
    pub destination: OutputDestination,
    pub naming: NamingOptions,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        ConversionConfig {
            format: PdfStandard::Standard,
            destination: OutputDestination::SourceDirectory,
            naming: NamingOptions::default(),
        }
    }
}

/// Renders a destination filename given the source stem, index, total items, and naming options
pub fn render_filename(
    stem: &str,
    index: usize,
    _total: usize,
    options: &NamingOptions,
) -> String {
    let now = std::time::SystemTime::now();
    let dt = format_iso_date(now);

    let num_str = format!("{:02}", index + 1);

    let mut result = options.pattern.clone();
    if result.is_empty() {
        result = "{name}".to_string();
    }

    result = result.replace("{name}", stem);
    result = result.replace("{date}", &dt);
    result = result.replace("{num}", &num_str);

    // Apply prefix and suffix
    if !options.prefix.is_empty() {
        result = format!("{}{}", options.prefix, result);
    }
    if !options.suffix.is_empty() {
        result = format!("{}{}", result, options.suffix);
    }

    // Ensure valid extension
    if !result.to_lowercase().ends_with(".pdf") {
        result.push_str(".pdf");
    }

    // Apply case transformation
    match options.case_transform {
        CaseTransform::Original => {}
        CaseTransform::Lowercase => {
            result = result.to_lowercase();
        }
        CaseTransform::Uppercase => {
            result = result.to_uppercase();
        }
    }

    result
}

/// Generates safe sequential output path if file already exists and safe_collision is true
pub fn resolve_target_path(
    source_path: &Path,
    index: usize,
    total: usize,
    config: &ConversionConfig,
) -> PathBuf {
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");

    let filename = render_filename(stem, index, total, &config.naming);

    let target_dir = match &config.destination {
        OutputDestination::SourceDirectory => {
            source_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
        }
        OutputDestination::Custom(custom) => custom.clone(),
    };

    let initial = target_dir.join(&filename);
    if !config.naming.safe_collision || !initial.exists() {
        return initial;
    }

    // Generate safe sequential copy: "filename (1).pdf", "filename (2).pdf"
    let base_stem = initial
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(stem);

    let mut counter = 1;
    loop {
        let candidate = target_dir.join(format!("{} ({}).pdf", base_stem, counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

fn format_iso_date(system_time: std::time::SystemTime) -> String {
    // Basic date formatting without heavy external dependencies
    let dur = system_time
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = secs / 86400;

    // Approximate civil date calculation
    let mut year = 1970i32;
    let mut day_count = days as i32;

    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if day_count < days_in_year {
            break;
        }
        day_count -= days_in_year;
        year += 1;
    }

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = [
        31,
        if is_leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 1;
    for &d in &month_days {
        if day_count < d {
            break;
        }
        day_count -= d;
        month += 1;
    }
    let day = day_count + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_filename_basic() {
        let opts = NamingOptions::default();
        let name = render_filename("sample", 0, 1, &opts);
        assert_eq!(name, "sample.pdf");
    }

    #[test]
    fn test_render_filename_prefix_suffix() {
        let mut opts = NamingOptions::default();
        opts.prefix = "V1_".to_string();
        opts.suffix = "_FINAL".to_string();
        let name = render_filename("report", 0, 1, &opts);
        assert_eq!(name, "V1_report_FINAL.pdf");
    }

    #[test]
    fn test_render_filename_case_transform() {
        let mut opts = NamingOptions::default();
        opts.case_transform = CaseTransform::Uppercase;
        let name = render_filename("my_document", 0, 1, &opts);
        assert_eq!(name, "MY_DOCUMENT.PDF");
    }

    #[test]
    fn test_render_filename_numbering() {
        let mut opts = NamingOptions::default();
        opts.pattern = "{name}_part{num}".to_string();
        let name = render_filename("batch", 2, 5, &opts);
        assert_eq!(name, "batch_part03.pdf");
    }
}

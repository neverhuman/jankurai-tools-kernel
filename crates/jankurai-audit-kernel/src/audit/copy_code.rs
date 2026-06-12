use serde::{Deserialize, Serialize};

pub const COPY_CODE_SCHEMA_VERSION: &str = "1.1.0";
pub const DEFAULT_MIN_LINES: usize = 10;
pub const DEFAULT_MIN_TOKENS: usize = 100;
pub const DEFAULT_MAX_FINDINGS: usize = 50;

pub const DEFAULT_JSON_PATH: &str = "target/jankurai/copy-code.json";
pub const DEFAULT_MD_PATH: &str = "target/jankurai/copy-code.md";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeOptions {
    pub min_lines: usize,
    pub min_tokens: usize,
    pub max_findings: usize,
    pub include_tests: bool,
    pub strict: bool,
}

impl Default for CopyCodeOptions {
    fn default() -> Self {
        Self {
            min_lines: DEFAULT_MIN_LINES,
            min_tokens: DEFAULT_MIN_TOKENS,
            max_findings: DEFAULT_MAX_FINDINGS,
            include_tests: false,
            strict: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodePolicy {
    pub active_source_only: bool,
    pub include_tests: bool,
    pub min_lines: usize,
    pub min_tokens: usize,
    pub max_findings: usize,
    pub strict: bool,
    pub excluded_roots: Vec<String>,
    pub warning_only_roots: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CopyCodeSummary {
    pub files_scanned: usize,
    pub files_considered: usize,
    pub active_files: usize,
    pub exact_file_classes: usize,
    pub hard_classes: usize,
    pub warning_classes: usize,
    pub hard_instances: usize,
    pub warning_instances: usize,
    pub duplicate_lines: usize,
    pub duplicate_tokens: usize,
    pub duplicate_bytes: usize,
    pub total_redundant_lines: usize,
    pub total_redundant_tokens: usize,
    pub total_redundant_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeInstance {
    pub path: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyCodeKind {
    ExactFile,
    ExactUnitSameName,
    ExactUnitDifferentName,
    TokenBlock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyCodeSeverity {
    Hard,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressedInfo {
    pub by: String,
    pub owner: Option<String>,
    pub reason: Option<String>,
    pub expires: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeClass {
    pub id: String,
    pub kind: CopyCodeKind,
    pub severity: CopyCodeSeverity,
    pub confidence: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_name: Option<String>,
    pub duplicate_lines: usize,
    pub duplicate_tokens: usize,
    pub duplicate_bytes: usize,
    pub instance_count: usize,
    pub total_redundant_lines: usize,
    pub total_redundant_tokens: usize,
    pub total_redundant_bytes: usize,
    pub effective_severity: CopyCodeSeverity,
    pub hard_fail: bool,
    pub fingerprint: String,
    pub reason: String,
    pub recommended_action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppressed: Option<SuppressedInfo>,
    pub instances: Vec<CopyCodeInstance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopyCodeReport {
    pub schema_version: String,
    pub generated_by: String,
    pub generated_at: String,
    pub repo: String,
    pub auditor_version: String,
    pub status: String,
    pub policy: CopyCodePolicy,
    pub summary: CopyCodeSummary,
    pub classes: Vec<CopyCodeClass>,
    pub notes: Vec<String>,
}

impl CopyCodeReport {
    pub fn empty() -> Self {
        Self {
            schema_version: COPY_CODE_SCHEMA_VERSION.into(),
            generated_by: "jankurai copy-code".into(),
            generated_at: String::new(),
            repo: String::new(),
            auditor_version: crate::model::AUDITOR_VERSION.into(),
            status: "skipped".into(),
            policy: CopyCodePolicy {
                active_source_only: true,
                include_tests: false,
                min_lines: DEFAULT_MIN_LINES,
                min_tokens: DEFAULT_MIN_TOKENS,
                max_findings: DEFAULT_MAX_FINDINGS,
                strict: false,
                excluded_roots: vec![],
                warning_only_roots: vec![],
            },
            summary: CopyCodeSummary::default(),
            classes: vec![],
            notes: vec![],
        }
    }
}

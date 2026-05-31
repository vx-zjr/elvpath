use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OperatingSystem {
    Windows,
    Macos,
    Linux,
}

impl OperatingSystem {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "macos" => Self::Macos,
            _ => Self::Linux,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalizedText {
    pub en: String,
    pub zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub schema_version: u32,
    pub categories: Vec<CatalogCategory>,
    pub items: Vec<CatalogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogCategory {
    pub id: String,
    pub name: LocalizedText,
    pub description: LocalizedText,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub id: String,
    pub category: String,
    pub name: LocalizedText,
    pub summary: LocalizedText,
    pub homepage: String,
    pub supported_platforms: Vec<OperatingSystem>,
    pub tags: Vec<String>,
    pub versions: Vec<VersionOption>,
    pub sources: Vec<Source>,
    pub detection: Vec<DetectionRule>,
    pub dependencies: Vec<String>,
    pub install_steps: Vec<InstallStepTemplate>,
    pub config_steps: Vec<ConfigChangeTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionOption {
    pub id: String,
    pub label: String,
    #[serde(default, rename = "default")]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub kind: SourceKind,
    pub url: String,
    #[serde(default)]
    pub notes: Option<LocalizedText>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Official,
    PackageManager,
    Mirror,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionRule {
    pub platforms: Vec<OperatingSystem>,
    pub kind: DetectionKind,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub source_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionKind {
    PathCommand,
    PackageManager,
    AppBundle,
    Registry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallStepTemplate {
    pub id: String,
    pub platforms: Vec<OperatingSystem>,
    pub kind: InstallStepKind,
    pub title: LocalizedText,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub mirror_url: Option<String>,
    #[serde(default)]
    pub checksum: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub source: SourceKind,
    pub permissions: StepPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallStepKind {
    Download,
    Command,
    PackageManager,
    Configure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepPermissions {
    #[serde(default)]
    pub requires_admin: bool,
    #[serde(default)]
    pub reason: Option<LocalizedText>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangeTemplate {
    pub target: String,
    pub action: ConfigAction,
    pub value: String,
    pub backup_hint: LocalizedText,
    pub rollback: LocalizedText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigAction {
    Append,
    Set,
    WriteFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanSelection {
    pub item_id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkMode {
    Official,
    CustomProxy,
    ChinaMirror,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkProfile {
    pub mode: NetworkMode,
    pub proxy_url: Option<String>,
    pub mirror_region: Option<String>,
}

impl Default for NetworkProfile {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Official,
            proxy_url: None,
            mirror_region: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPlan {
    pub id: String,
    pub platform: OperatingSystem,
    pub network: NetworkProfile,
    pub items: Vec<PlannedItem>,
    pub steps: Vec<PlannedStep>,
    pub requires_admin: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedItem {
    pub item_id: String,
    pub name: LocalizedText,
    pub version: String,
    pub dependency: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedStep {
    pub id: String,
    pub item_id: String,
    pub title: LocalizedText,
    pub kind: InstallStepKind,
    pub source: SourceKind,
    pub url: Option<String>,
    pub checksum: Option<String>,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub permissions: StepPermissions,
    pub config_changes: Vec<ConfigChangeTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DetectionStatus {
    Installed,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    pub item_id: String,
    pub status: DetectionStatus,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub satisfies_requested: bool,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRefreshResult {
    pub catalog: Catalog,
    pub refreshed_online: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRequest {
    pub step: PlannedStep,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    #[serde(default)]
    pub network_profile: Option<NetworkProfile>,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStatus {
    DryRun,
    Completed,
    Failed,
    NeedsPrivilege,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    pub step_id: String,
    pub status: ExecutionStatus,
    pub message: String,
    pub log_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackResult {
    pub target: String,
    pub message: String,
}

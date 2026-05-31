use std::process::Command;

use crate::models::{CatalogItem, DetectionResult, DetectionStatus, OperatingSystem};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput>;
}

pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let output = Command::new(program).args(args).output()?;
        Ok(CommandOutput {
            status: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

pub fn detect_item(
    item: &CatalogItem,
    platform: OperatingSystem,
    runner: &dyn CommandRunner,
) -> DetectionResult {
    for rule in item
        .detection
        .iter()
        .filter(|rule| rule.platforms.contains(&platform))
    {
        let output = runner.run(&rule.program, &rule.args);
        match output {
            Ok(output) if output.status == 0 => {
                let combined = format!("{}\n{}", output.stdout, output.stderr);
                return DetectionResult {
                    item_id: item.id.clone(),
                    status: DetectionStatus::Installed,
                    version: extract_version(&combined),
                    path: Some(rule.program.clone()),
                    source: rule
                        .source_hint
                        .clone()
                        .or_else(|| Some("PATH".to_string())),
                    satisfies_requested: true,
                    conflicts: Vec::new(),
                };
            }
            Ok(_) | Err(_) => continue,
        }
    }

    DetectionResult {
        item_id: item.id.clone(),
        status: DetectionStatus::Missing,
        version: None,
        path: None,
        source: None,
        satisfies_requested: false,
        conflicts: Vec::new(),
    }
}

pub fn scan_catalog(
    items: &[CatalogItem],
    platform: OperatingSystem,
    runner: &dyn CommandRunner,
) -> Vec<DetectionResult> {
    items
        .iter()
        .filter(|item| item.supported_platforms.contains(&platform))
        .map(|item| detect_item(item, platform.clone(), runner))
        .collect()
}

fn extract_version(output: &str) -> Option<String> {
    output
        .split_whitespace()
        .map(|part| {
            part.trim_matches(|ch: char| {
                matches!(ch, 'v' | 'V' | ',' | ';' | ':' | '(' | ')' | '[' | ']')
            })
        })
        .find(|part| {
            let has_digit = part.chars().any(|ch| ch.is_ascii_digit());
            let has_separator = part.contains('.') || part.contains('-');
            has_digit && has_separator
        })
        .map(|part| part.to_string())
}

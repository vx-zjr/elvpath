use std::collections::HashMap;

use elvpath_lib::catalog::load_embedded_catalog;
use elvpath_lib::detection::{detect_item, CommandOutput, CommandRunner};
use elvpath_lib::models::{DetectionStatus, OperatingSystem};

#[derive(Default)]
struct FakeRunner {
    outputs: HashMap<String, CommandOutput>,
}

impl FakeRunner {
    fn with(mut self, command: &str, output: CommandOutput) -> Self {
        self.outputs.insert(command.to_string(), output);
        self
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<CommandOutput> {
        let key = if args.is_empty() {
            program.to_string()
        } else {
            format!("{} {}", program, args.join(" "))
        };
        self.outputs
            .get(&key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing fake output for {key}"))
    }
}

#[test]
fn detect_item_reads_version_from_path_command() {
    let catalog = load_embedded_catalog().expect("catalog loads");
    let rust = catalog.items.iter().find(|item| item.id == "rust").unwrap();
    let runner = FakeRunner::default().with(
        "rustc --version",
        CommandOutput {
            status: 0,
            stdout: "rustc 1.76.0 (07dca489a 2024-02-04)\n".into(),
            stderr: String::new(),
        },
    );

    let result = detect_item(rust, OperatingSystem::Windows, &runner);

    assert_eq!(result.status, DetectionStatus::Installed);
    assert_eq!(result.version.as_deref(), Some("1.76.0"));
    assert_eq!(result.path.as_deref(), Some("rustc"));
}

#[test]
fn detect_item_reports_missing_when_command_fails() {
    let catalog = load_embedded_catalog().expect("catalog loads");
    let node = catalog
        .items
        .iter()
        .find(|item| item.id == "nodejs")
        .unwrap();
    let runner = FakeRunner::default().with(
        "node --version",
        CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: "not found".into(),
        },
    );

    let result = detect_item(node, OperatingSystem::Linux, &runner);

    assert_eq!(result.status, DetectionStatus::Missing);
    assert!(result.version.is_none());
}

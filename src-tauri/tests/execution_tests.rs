use std::cell::RefCell;

use elvpath_lib::catalog::load_embedded_catalog;
use elvpath_lib::execution::{
    execute_install_step_with_runner, execute_install_step_with_runner_and_observer, ProcessOutput,
    ProcessRunner,
};
use elvpath_lib::models::{
    ExecutionRequest, ExecutionStatus, NetworkMode, NetworkProfile, OperatingSystem, PlanSelection,
    PlannedStep,
};
use elvpath_lib::planner::build_install_plan;

#[derive(Default)]
struct FakeRunner {
    calls: RefCell<Vec<(String, Vec<String>)>>,
    fail: bool,
}

impl ProcessRunner for FakeRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<ProcessOutput> {
        self.calls
            .borrow_mut()
            .push((program.to_string(), args.to_vec()));
        if self.fail {
            return Err(anyhow::anyhow!("program not found"));
        }
        Ok(ProcessOutput {
            status: 0,
            stdout: "ok".into(),
            stderr: String::new(),
        })
    }
}

#[test]
fn windows_language_steps_use_powershell_install_backend() {
    for item_id in [
        "rust", "nodejs", "python", "java", "go", "dotnet", "php", "ruby",
    ] {
        let step = planned_windows_step(item_id);
        let runner = FakeRunner::default();

        let result = execute_install_step_with_runner(
            ExecutionRequest {
                step,
                dry_run: false,
                network_profile: None,
            },
            OperatingSystem::Windows,
            &runner,
        )
        .expect("execution result");

        assert_eq!(result.status, ExecutionStatus::Completed, "{item_id}");
        let calls = runner.calls.borrow();
        let (program, args) = calls.first().expect("one process call");
        assert_eq!(program, "powershell.exe", "{item_id}");
        assert!(
            args.iter().any(|arg| arg.contains("Invoke-WebRequest")),
            "expected download script for {item_id}: {args:?}"
        );
    }
}

#[test]
fn windows_archive_installers_use_fresh_zip_staging_instead_of_expand_archive() {
    for item_id in ["java", "go"] {
        let script =
            captured_windows_script(planned_windows_step(item_id), NetworkProfile::default());

        assert!(
            script.contains("[Guid]::NewGuid()"),
            "expected unique extraction staging directory for {item_id}: {script}"
        );
        assert!(
            script.contains("[System.IO.Compression.ZipFile]::ExtractToDirectory"),
            "expected .NET zip extraction for {item_id}: {script}"
        );
        assert!(
            !script.contains("Expand-Archive"),
            "PowerShell Expand-Archive should not be used for {item_id}: {script}"
        );
    }
}

#[test]
fn windows_language_downloads_use_curl_fallback_and_custom_proxy() {
    let script = captured_windows_script(
        planned_windows_step("rust"),
        NetworkProfile {
            mode: NetworkMode::CustomProxy,
            proxy_url: Some("http://127.0.0.1:7890".into()),
            mirror_region: None,
        },
    );

    assert!(script.contains("curl.exe"), "{script}");
    assert!(script.contains("Invoke-WebRequest"), "{script}");
    assert!(
        script.contains("$env:HTTP_PROXY = 'http://127.0.0.1:7890'"),
        "{script}"
    );
    assert!(
        script.contains("$env:HTTPS_PROXY = 'http://127.0.0.1:7890'"),
        "{script}"
    );
    assert!(script.contains("--proxy"), "{script}");
}

#[test]
fn completed_process_output_includes_command_stdout_for_terminal_display() {
    let mut step = planned_windows_step("rust");
    step.item_id = "unknown-tool".into();
    step.command = Some("cmd".into());
    step.args = vec!["/C".into(), "echo installing".into()];
    step.url = None;
    let runner = FakeRunner {
        fail: false,
        ..FakeRunner::default()
    };

    let result = execute_install_step_with_runner(
        ExecutionRequest {
            step,
            dry_run: false,
            network_profile: None,
        },
        OperatingSystem::Windows,
        &runner,
    )
    .expect("execution result");

    assert_eq!(result.status, ExecutionStatus::Completed);
    assert!(result.message.contains("ok"));
}

#[test]
fn execution_observer_receives_process_output_for_terminal_streaming() {
    let mut step = planned_windows_step("rust");
    step.item_id = "unknown-tool".into();
    step.command = Some("cmd".into());
    step.args = vec!["/C".into(), "echo installing".into()];
    step.url = None;
    let runner = FakeRunner::default();
    let mut observed = Vec::new();

    let result = execute_install_step_with_runner_and_observer(
        ExecutionRequest {
            step,
            dry_run: false,
            network_profile: None,
        },
        OperatingSystem::Windows,
        &runner,
        &mut |line| observed.push(line.to_string()),
    )
    .expect("execution result");

    assert_eq!(result.status, ExecutionStatus::Completed);
    assert!(observed.iter().any(|line| line.contains("ok")));
}

#[test]
fn missing_generic_program_returns_failed_result_instead_of_error() {
    let mut step = planned_windows_step("rust");
    step.item_id = "unknown-tool".into();
    step.command = Some("definitely-missing-program".into());
    step.url = None;
    let runner = FakeRunner {
        fail: true,
        ..FakeRunner::default()
    };

    let result = execute_install_step_with_runner(
        ExecutionRequest {
            step,
            dry_run: false,
            network_profile: None,
        },
        OperatingSystem::Windows,
        &runner,
    )
    .expect("missing command is converted to failed execution result");

    assert_eq!(result.status, ExecutionStatus::Failed);
    assert!(result.message.contains("Program not found"));
}

fn planned_windows_step(item_id: &str) -> PlannedStep {
    let catalog = load_embedded_catalog().expect("catalog loads");
    let plan = build_install_plan(
        &catalog,
        &[PlanSelection {
            item_id: item_id.into(),
            version: None,
        }],
        &NetworkProfile::default(),
        OperatingSystem::Windows,
    )
    .expect("plan builds");
    plan.steps
        .into_iter()
        .find(|step| step.item_id == item_id)
        .expect("planned step")
}

fn captured_windows_script(step: PlannedStep, network_profile: NetworkProfile) -> String {
    let runner = FakeRunner::default();
    let result = execute_install_step_with_runner(
        ExecutionRequest {
            step,
            dry_run: false,
            network_profile: Some(network_profile),
        },
        OperatingSystem::Windows,
        &runner,
    )
    .expect("execution result");

    assert_eq!(result.status, ExecutionStatus::Completed);
    let script = runner
        .calls
        .borrow()
        .first()
        .and_then(|(_, args)| args.last().cloned())
        .expect("captured powershell script");
    script
}

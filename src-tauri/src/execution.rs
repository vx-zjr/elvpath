use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use crate::models::{
    ConfigChangeTemplate, ExecutionRequest, ExecutionResult, ExecutionStatus, NetworkMode,
    NetworkProfile, OperatingSystem, PlannedStep, RollbackResult,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub trait ProcessRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<ProcessOutput>;

    fn run_with_observer(
        &self,
        program: &str,
        args: &[String],
        observer: &mut dyn FnMut(&str),
    ) -> anyhow::Result<ProcessOutput> {
        let output = self.run(program, args)?;
        for line in output.stdout.lines().chain(output.stderr.lines()) {
            observer(line);
        }
        Ok(output)
    }
}

pub struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, program: &str, args: &[String]) -> anyhow::Result<ProcessOutput> {
        run_system_process(program, args, None)
    }

    fn run_with_observer(
        &self,
        program: &str,
        args: &[String],
        observer: &mut dyn FnMut(&str),
    ) -> anyhow::Result<ProcessOutput> {
        run_system_process(program, args, Some(observer))
    }
}

pub fn execute_install_step(request: ExecutionRequest) -> anyhow::Result<ExecutionResult> {
    let runner = SystemProcessRunner;
    execute_install_step_with_runner(request, OperatingSystem::current(), &runner)
}

pub fn execute_install_step_with_runner(
    request: ExecutionRequest,
    platform: OperatingSystem,
    runner: &dyn ProcessRunner,
) -> anyhow::Result<ExecutionResult> {
    let mut noop = |_line: &str| {};
    execute_install_step_with_runner_and_observer(request, platform, runner, &mut noop)
}

pub fn execute_install_step_with_runner_and_observer(
    request: ExecutionRequest,
    platform: OperatingSystem,
    runner: &dyn ProcessRunner,
    observer: &mut dyn FnMut(&str),
) -> anyhow::Result<ExecutionResult> {
    let step = request.step;
    let network_profile = request.network_profile.unwrap_or_default();
    if request.dry_run {
        let preview = command_preview(step.command.as_deref(), &step.args, step.url.as_deref());
        observer(&preview);
        return Ok(ExecutionResult {
            step_id: step.id,
            status: ExecutionStatus::DryRun,
            message: preview,
            log_path: None,
        });
    }

    if platform == OperatingSystem::Windows {
        if let Some(script) = windows_language_installer_script(&step, &network_profile) {
            let args = vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-Command".to_string(),
                script,
            ];
            return Ok(run_process_as_result(
                &step.id,
                runner,
                "powershell.exe",
                &args,
                observer,
            ));
        }
    }

    if step.permissions.requires_admin && !is_elevated_context() {
        return Ok(ExecutionResult {
            step_id: step.id,
            status: ExecutionStatus::NeedsPrivilege,
            message: "This step requires administrator privileges. Re-run through the platform elevation prompt.".to_string(),
            log_path: None,
        });
    }

    let Some(command) = step.command else {
        return Ok(ExecutionResult {
            step_id: step.id,
            status: ExecutionStatus::Failed,
            message: "No executable command is defined for this step.".to_string(),
            log_path: None,
        });
    };

    Ok(run_process_as_result(
        &step.id, runner, &command, &step.args, observer,
    ))
}

pub fn rollback_config_change(change: ConfigChangeTemplate) -> RollbackResult {
    RollbackResult {
        target: change.target,
        message: change.rollback.en,
    }
}

fn run_process_as_result(
    step_id: &str,
    runner: &dyn ProcessRunner,
    program: &str,
    args: &[String],
    observer: &mut dyn FnMut(&str),
) -> ExecutionResult {
    match runner.run_with_observer(program, args, observer) {
        Ok(output) => {
            let status = if output.status == 0 {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Failed
            };
            let message = format!("{}\n{}", output.stdout, output.stderr)
                .trim()
                .to_string();
            ExecutionResult {
                step_id: step_id.to_string(),
                status,
                message,
                log_path: None,
            }
        }
        Err(error) => ExecutionResult {
            step_id: step_id.to_string(),
            status: ExecutionStatus::Failed,
            message: format!("Program not found or could not be started: {program}. {error}"),
            log_path: None,
        },
    }
}

fn run_system_process(
    program: &str,
    args: &[String],
    mut observer: Option<&mut dyn FnMut(&str)>,
) -> anyhow::Result<ProcessOutput> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let (sender, receiver) = mpsc::channel::<(&'static str, String)>();
    let stdout_handle = child
        .stdout
        .take()
        .map(|stdout| spawn_pipe_reader(stdout, "stdout", sender.clone()));
    let stderr_handle = child
        .stderr
        .take()
        .map(|stderr| spawn_pipe_reader(stderr, "stderr", sender.clone()));
    drop(sender);

    let mut stdout = String::new();
    let mut stderr = String::new();
    let status = loop {
        while let Ok((stream, line)) = receiver.try_recv() {
            append_observed_line(stream, line, &mut stdout, &mut stderr, &mut observer);
        }

        if let Some(status) = child.try_wait()? {
            break status.code().unwrap_or(1);
        }

        match receiver.recv_timeout(Duration::from_millis(80)) {
            Ok((stream, line)) => {
                append_observed_line(stream, line, &mut stdout, &mut stderr, &mut observer);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let status = child.wait()?;
                break status.code().unwrap_or(1);
            }
        }
    };

    for (stream, line) in receiver.try_iter() {
        append_observed_line(stream, line, &mut stdout, &mut stderr, &mut observer);
    }

    if let Some(handle) = stdout_handle {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }

    Ok(ProcessOutput {
        status,
        stdout,
        stderr,
    })
}

fn spawn_pipe_reader<R: Read + Send + 'static>(
    pipe: R,
    stream: &'static str,
    sender: Sender<(&'static str, String)>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        for line in BufReader::new(pipe).lines() {
            match line {
                Ok(line) => {
                    let _ = sender.send((stream, line));
                }
                Err(error) => {
                    let _ =
                        sender.send((stream, format!("failed to read process output: {error}")));
                    break;
                }
            }
        }
    })
}

fn append_observed_line(
    stream: &str,
    line: String,
    stdout: &mut String,
    stderr: &mut String,
    observer: &mut Option<&mut dyn FnMut(&str)>,
) {
    if let Some(observer) = observer.as_deref_mut() {
        observer(&line);
    }

    let target = if stream == "stderr" { stderr } else { stdout };
    target.push_str(&line);
    target.push('\n');
}

fn command_preview(command: Option<&str>, args: &[String], url: Option<&str>) -> String {
    match (command, url) {
        (Some(command), Some(url)) => {
            format!(
                "Dry run: download {url}, then run `{command} {}`",
                args.join(" ")
            )
        }
        (Some(command), None) => format!("Dry run: run `{command} {}`", args.join(" ")),
        (None, Some(url)) => format!("Dry run: download {url}"),
        (None, None) => "Dry run: configuration-only step".to_string(),
    }
}

fn windows_language_installer_script(
    step: &PlannedStep,
    network_profile: &NetworkProfile,
) -> Option<String> {
    let body = match step.item_id.as_str() {
        "rust" => {
            r#"
$file = Join-Path $downloadDir 'rustup-init.exe'
Download-File 'https://win.rustup.rs/x86_64' $file $null
& $file -y --default-toolchain stable
"#
        }
        "nodejs" => {
            r#"
$index = Invoke-ElvPathRest 'https://nodejs.org/dist/index.json' $null
$release = $index | Where-Object { $_.lts } | Select-Object -First 1
$version = $release.version
$fileName = "node-$version-win-x64.zip"
$url = "https://nodejs.org/dist/$version/$fileName"
$file = Join-Path $downloadDir $fileName
$target = Join-Path $toolsDir 'node'
Download-File $url $file $null
$staging = New-ZipStaging $file
$root = Get-ChildItem -LiteralPath $staging -Directory | Select-Object -First 1
if (-not $root) { throw "Node.js archive did not contain a root directory." }
Move-FreshDirectory $root.FullName $target
Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
Add-UserPath $target
"#
        }
        "python" => {
            r#"
$url = 'https://www.python.org/ftp/python/3.12.0/python-3.12.0-amd64.exe'
$file = Join-Path $downloadDir 'python-3.12.0-amd64.exe'
Download-File $url $file $null
Start-Process -FilePath $file -ArgumentList @('/quiet','InstallAllUsers=0','PrependPath=1','Include_test=0') -Wait
"#
        }
        "java" => {
            r#"
$url = 'https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jdk/hotspot/normal/eclipse'
$file = Join-Path $downloadDir 'temurin-jdk-21.zip'
$target = Join-Path $toolsDir 'jdk-21'
Download-File $url $file $null
$staging = New-ZipStaging $file
$root = Get-ChildItem -LiteralPath $staging -Directory | Select-Object -First 1
if (-not $root) { throw "Temurin archive did not contain a root directory." }
Move-FreshDirectory $root.FullName $target
Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
[Environment]::SetEnvironmentVariable('JAVA_HOME', $target, 'User')
Add-UserPath (Join-Path $target 'bin')
"#
        }
        "go" => {
            r#"
$releases = Invoke-ElvPathRest 'https://go.dev/dl/?mode=json' $null
$release = $releases | Select-Object -First 1
$asset = $release.files | Where-Object { $_.os -eq 'windows' -and $_.arch -eq 'amd64' -and $_.kind -eq 'archive' } | Select-Object -First 1
if (-not $asset) { throw 'No official Go windows amd64 archive found.' }
$url = "https://go.dev/dl/$($asset.filename)"
$file = Join-Path $downloadDir $asset.filename
$target = Join-Path $toolsDir 'go'
Download-File $url $file $null
$staging = New-ZipStaging $file
$root = Join-Path $staging 'go'
if (-not (Test-Path -LiteralPath $root)) { throw "Go archive did not contain a go directory." }
Move-FreshDirectory $root $target
Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
[Environment]::SetEnvironmentVariable('GOROOT', $target, 'User')
Add-UserPath (Join-Path $target 'bin')
"#
        }
        "dotnet" => {
            r#"
$scriptFile = Join-Path $downloadDir 'dotnet-install.ps1'
$target = Join-Path $env:LOCALAPPDATA 'Microsoft\dotnet'
Download-File 'https://dot.net/v1/dotnet-install.ps1' $scriptFile $null
& $scriptFile -Channel 8.0 -InstallDir $target
Add-UserPath $target
"#
        }
        "php" => {
            r#"
$url = 'https://windows.php.net/downloads/releases/latest/php-8.3-nts-Win32-vs16-x64-latest.zip'
$file = Join-Path $downloadDir 'php-8.3-nts-Win32-vs16-x64-latest.zip'
$target = Join-Path $toolsDir 'php'
Download-File $url $file $null
Remove-Item -LiteralPath $target -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $target | Out-Null
$staging = New-ZipStaging $file
Get-ChildItem -LiteralPath $staging -Force | Move-Item -Destination $target
Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
Add-UserPath $target
"#
        }
        "ruby" => {
            r#"
$headers = @{ 'User-Agent' = 'ElvPath' }
$release = Invoke-ElvPathRest 'https://api.github.com/repos/oneclick/rubyinstaller2/releases/latest' $headers
$asset = $release.assets | Where-Object { $_.name -match '^rubyinstaller-devkit-.*-x64\.exe$' } | Select-Object -First 1
if (-not $asset) { throw 'No RubyInstaller x64 DevKit installer found.' }
$file = Join-Path $downloadDir $asset.name
Download-File $asset.browser_download_url $file $headers
Start-Process -FilePath $file -ArgumentList @('/verysilent','/tasks=modpath') -Wait
"#
        }
        _ => return None,
    };

    Some(format!("{}\n{}", powershell_prelude(network_profile), body))
}

fn powershell_prelude(network_profile: &NetworkProfile) -> String {
    let proxy_setup = match (
        &network_profile.mode,
        network_profile.proxy_url.as_deref().map(str::trim),
    ) {
        (NetworkMode::CustomProxy, Some(proxy_url)) if !proxy_url.is_empty() => {
            let proxy_url = escape_powershell_single_quoted(proxy_url);
            format!("$env:HTTP_PROXY = '{proxy_url}'\n$env:HTTPS_PROXY = '{proxy_url}'\n")
        }
        _ => String::new(),
    };

    format!(
        r#"$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$downloadDir = Join-Path $env:LOCALAPPDATA 'ElvPath\downloads'
$toolsDir = Join-Path $env:LOCALAPPDATA 'ElvPath\tools'
New-Item -ItemType Directory -Force -Path $downloadDir,$toolsDir | Out-Null
{proxy_setup}function Write-ElvPath([string]$Message) {{
  Write-Host "[elvpath] $Message"
}}
function Download-File([string]$Url, [string]$OutFile, [hashtable]$Headers) {{
  Write-ElvPath "download $Url"
  if (Test-Path -LiteralPath $OutFile) {{
    Remove-Item -LiteralPath $OutFile -Force -ErrorAction SilentlyContinue
  }}
  if (Get-Command curl.exe -ErrorAction SilentlyContinue) {{
    $curlArgs = @('-L','--fail','--retry','3','--retry-delay','2','--connect-timeout','30','-o',$OutFile)
    if ($env:HTTPS_PROXY) {{
      $curlArgs += @('--proxy', $env:HTTPS_PROXY)
    }}
    if ($Headers) {{
      foreach ($key in $Headers.Keys) {{
        $curlArgs += @('-H', "${{key}}: $($Headers[$key])")
      }}
    }}
    $curlArgs += $Url
    & curl.exe @curlArgs
    if ($LASTEXITCODE -ne 0) {{
      throw "curl.exe failed with exit code $LASTEXITCODE"
    }}
    return
  }}
  $params = @{{ Uri = $Url; OutFile = $OutFile; UseBasicParsing = $true }}
  if ($env:HTTPS_PROXY) {{
    $params.Proxy = $env:HTTPS_PROXY
  }}
  if ($Headers) {{
    $params.Headers = $Headers
  }}
  Invoke-WebRequest @params
}}
function Invoke-ElvPathRest([string]$Url, [hashtable]$Headers) {{
  Write-ElvPath "fetch metadata $Url"
  $params = @{{ Uri = $Url; UseBasicParsing = $true }}
  if ($env:HTTPS_PROXY) {{
    $params.Proxy = $env:HTTPS_PROXY
  }}
  if ($Headers) {{
    $params.Headers = $Headers
  }}
  Invoke-RestMethod @params
}}
function New-ZipStaging([string]$ZipFile) {{
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  $staging = Join-Path $downloadDir ([Guid]::NewGuid().ToString())
  New-Item -ItemType Directory -Force -Path $staging | Out-Null
  Write-ElvPath "extract $ZipFile"
  [System.IO.Compression.ZipFile]::ExtractToDirectory($ZipFile, $staging)
  return $staging
}}
function Move-FreshDirectory([string]$Source, [string]$Destination) {{
  if (Test-Path -LiteralPath $Destination) {{
    Remove-Item -LiteralPath $Destination -Recurse -Force
  }}
  $parent = Split-Path -Parent $Destination
  New-Item -ItemType Directory -Force -Path $parent | Out-Null
  Move-Item -LiteralPath $Source -Destination $Destination
}}
function Add-UserPath([string]$PathToAdd) {{
  $current = [Environment]::GetEnvironmentVariable('Path', 'User')
  if ([string]::IsNullOrWhiteSpace($current)) {{
    $parts = @()
  }} else {{
    $parts = $current -split ';' | Where-Object {{ -not [string]::IsNullOrWhiteSpace($_) }}
  }}
  $exists = $parts | Where-Object {{ $_ -ieq $PathToAdd }} | Select-Object -First 1
  if (-not $exists) {{
    $next = ($parts + $PathToAdd) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $next, 'User')
  }}
  if (($env:Path -split ';') -notcontains $PathToAdd) {{
    $env:Path = "$env:Path;$PathToAdd"
  }}
}}
"#
    )
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn is_elevated_context() -> bool {
    #[cfg(target_family = "unix")]
    {
        Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .is_some_and(|uid| uid.trim() == "0")
    }
    #[cfg(not(target_family = "unix"))]
    {
        false
    }
}

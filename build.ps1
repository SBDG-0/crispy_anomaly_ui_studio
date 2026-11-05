param([switch]$Test,[string]$OutputName='Anomaly UI Studio.exe')
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
if (Test-Path -LiteralPath "$PSScriptRoot\.toolchain\cargo\bin\cargo.exe") {
    $env:RUSTUP_HOME = "$PSScriptRoot\.toolchain\rustup"
    $env:CARGO_HOME = "$PSScriptRoot\.toolchain\cargo"
    $env:PATH = "$PSScriptRoot\.toolchain\cargo\bin;$PSScriptRoot\.toolchain\w64devkit\bin;$env:PATH"
    $env:RUSTFLAGS = '-C link-self-contained=yes'
}
if ($Test) {
    cargo test --locked
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
New-Item -ItemType Directory -Force -Path "$PSScriptRoot\dist" | Out-Null
Copy-Item -LiteralPath "$PSScriptRoot\target\release\anomaly-ui-editor.exe" -Destination (Join-Path "$PSScriptRoot\dist" $OutputName)
Copy-Item -LiteralPath "$PSScriptRoot\README.md" -Destination "$PSScriptRoot\dist\README.md"

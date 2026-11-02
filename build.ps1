param([switch]$Release, [switch]$Test)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $env:RUSTUP_HOME = Join-Path $PSScriptRoot '.tools\rustup'
    $env:CARGO_HOME = Join-Path $PSScriptRoot '.tools\cargo'
    $env:PATH = (Join-Path $PSScriptRoot '.tools\w64devkit\bin') + ';' + $env:PATH
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $PSScriptRoot '.tools\w64devkit\bin\gcc.exe'
    $rustSupport = Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\lib\self-contained'
    $env:RUSTFLAGS = '-Clink-self-contained=yes -Lnative=' + $rustSupport
    $cargo = Join-Path $env:CARGO_HOME 'bin\cargo.exe'
    if ($Test) { & $cargo test --offline } elseif ($Release) { & $cargo build --release } else { & $cargo build }
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
} finally { Pop-Location }

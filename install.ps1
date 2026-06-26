$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$UserBin = "$env:USERPROFILE\.cargo\bin"

Write-Host "Building mltv..."
cargo build --manifest-path (Join-Path $ScriptDir "Cargo.toml") 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Host "mltv build failed" -ForegroundColor Red; exit 1 }

$MltvBinary = Join-Path $ScriptDir "target\debug\mltv.exe"

Write-Host "Building mpm..."
& $MltvBinary deploy (Join-Path $ScriptDir "mpm.mltv") -o (Join-Path $ScriptDir "mpm.exe") 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Host "mpm build failed" -ForegroundColor Red; exit 1 }

New-Item -ItemType Directory -Path $UserBin -Force | Out-Null
Copy-Item $MltvBinary (Join-Path $UserBin "mltv.exe") -Force
Copy-Item (Join-Path $ScriptDir "mpm.exe") (Join-Path $UserBin "mpm.exe") -Force
Write-Host "Installed mltv and mpm to $UserBin"

$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$UserBin*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$UserBin", "User")
    Write-Host "Added $UserBin to user PATH"
}

Write-Host ""
Write-Host "mltv and mpm installed! Restart your terminal, then:"
Write-Host "  mltv deploy myfile.mltv -o myprogram"
Write-Host "  mpm install mltv-lang/sample-lib"

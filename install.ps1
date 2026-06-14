param([switch]$NoDestruct)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ExtDir = Join-Path $ScriptDir "vscode-mltv"

# --- Build ---
Write-Host "Building mltv..."
$null = cargo build --release --manifest-path (Join-Path $ScriptDir "Cargo.toml")
if ($LASTEXITCODE -ne 0) { exit 1 }

$Binary = Join-Path $ScriptDir "target\release\mltv.exe"

Write-Host "Building mpm..."
$null = & $Binary deploy (Join-Path $ScriptDir "mpm.mltv") -o (Join-Path $ScriptDir "mpm")
$MpmBinary = Join-Path $ScriptDir "mpm.exe"

# --- Install binaries ---
$UserBin = "$env:USERPROFILE\.cargo\bin"
if (!(Test-Path $UserBin)) { New-Item -ItemType Directory -Path $UserBin -Force | Out-Null }

Copy-Item $Binary (Join-Path $UserBin "mltv.exe") -Force
Copy-Item $MpmBinary (Join-Path $UserBin "mpm.exe") -Force
Write-Host "Installed mltv and mpm to $UserBin"

# Add to PATH if missing
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$UserBin*") {
    [Environment]::SetEnvironmentVariable("PATH", "$userPath;$UserBin", "User")
    Write-Host "Added $UserBin to user PATH"
}

# --- Install VS Code extension ---
$VscodeExt = "$env:USERPROFILE\.vscode\extensions"
if (Test-Path $VscodeExt) {
    $Dest = Join-Path $VscodeExt "molotov-language-0.1.0"
    if (Test-Path $Dest) { Remove-Item -Recurse -Force $Dest }
    Copy-Item -Recurse $ExtDir $Dest
    Write-Host "Installed VS Code extension to $Dest"
}

$VscodeOssExt = "$env:USERPROFILE\.vscode-oss\extensions"
if (Test-Path $VscodeOssExt) {
    $Dest2 = Join-Path $VscodeOssExt "molotov-language-0.1.0"
    if (Test-Path $Dest2) { Remove-Item -Recurse -Force $Dest2 }
    Copy-Item -Recurse $ExtDir $Dest2
    Write-Host "Installed VS Code extension to $Dest2"
}

Write-Host ""
Write-Host "mltv and mpm installed! Restart your terminal, then:"
Write-Host "  mltv deploy myfile.mltv -o myprogram"
Write-Host "  mpm install mltv-lang/sample-lib"

# --- Self-destruct ---
if (-not $NoDestruct) {
    $ScriptPath = $MyInvocation.MyCommand.Path
    $OtherScript = Join-Path (Split-Path -Parent $ScriptPath) "install.sh"
    Write-Host ""
    Write-Host "Self-destructing in 3 seconds..."
    Start-Job -ScriptBlock {
        param($self, $other)
        Start-Sleep -Seconds 3
        Remove-Item -Force $self -ErrorAction SilentlyContinue
        Remove-Item -Force $other -ErrorAction SilentlyContinue
    } -ArgumentList $ScriptPath, $OtherScript | Out-Null
}

# Đóng gói VieEdit cho Windows.
#   .\build.ps1                    build + đóng gói
#   .\build.ps1 -SkipTests         bỏ qua kiểm thử
#   .\build.ps1 -BundleFFmpeg      kèm ffmpeg.exe vào bản phát hành (zip rất lớn)
#   .\build.ps1 -Toolchain stable-x86_64-pc-windows-gnu
param(
    [switch]$SkipTests,
    [switch]$BundleFFmpeg,
    [string]$Toolchain = ""
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Root

function Say($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Warn($msg) { Write-Host "    $msg" -ForegroundColor Yellow }
function Fail($msg) { Write-Host "Lỗi: $msg" -ForegroundColor Red; exit 1 }

function RunCargo {
    param([string[]]$CargoArgs)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    if ($Toolchain) {
        & cargo "+$Toolchain" @CargoArgs
    } else {
        & cargo @CargoArgs
    }
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0) { Fail "cargo $($CargoArgs -join ' ') thất bại (mã $code)" }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail "chưa có cargo. Cài Rust tại https://rustup.rs"
}
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Warn "chưa có ffmpeg trong PATH. Ứng dụng vẫn chạy nhưng không xem trước / xuất được video."
    Warn "Cài bằng:  winget install Gyan.FFmpeg"
}

if (-not $SkipTests) {
    Say "Chạy kiểm thử"
    RunCargo @("test", "--release")
}

Say "Biên dịch bản phát hành"
RunCargo @("build", "--release")

$targetDir = "target\release"
if (-not (Test-Path (Join-Path $targetDir "viedit.exe"))) {
    Fail "không thấy viedit.exe trong $targetDir"
}

$dist = Join-Path $Root "dist\viedit-windows-x64"
if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Copy-Item (Join-Path $targetDir "viedit.exe") $dist
Copy-Item "assets" $dist -Recurse
Remove-Item (Join-Path $dist "assets\library\README.md") -ErrorAction SilentlyContinue
Copy-Item "README.md", "LICENSE" $dist

# Kèm FFmpeg nếu được yêu cầu, để bản phát hành chạy độc lập
if ($BundleFFmpeg) {
    $ff = Get-Command ffmpeg -ErrorAction SilentlyContinue
    if ($ff) {
        $ffDir = Split-Path $ff.Source
        $binDir = Join-Path $dist "ffmpeg\bin"
        New-Item -ItemType Directory -Force -Path $binDir | Out-Null
        Copy-Item (Join-Path $ffDir "ffmpeg.exe") $binDir -ErrorAction SilentlyContinue
        Copy-Item (Join-Path $ffDir "ffprobe.exe") $binDir -ErrorAction SilentlyContinue
        Warn "đã kèm FFmpeg từ $ffDir"
    } else {
        Warn "không tìm thấy ffmpeg để kèm theo"
    }
}

Say "Đóng gói ZIP"
$zip = Join-Path $Root "dist\viedit-windows-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$dist\*" -DestinationPath $zip

$size = "{0:N1} MB" -f ((Get-Item $zip).Length / 1MB)
Say "Xong: $zip ($size)"
Write-Host ""
Write-Host "Chạy thử:  $dist\viedit.exe"

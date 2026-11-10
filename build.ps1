# Đóng gói VieEdit cho Windows.
#   .\build.ps1              build + đóng gói
#   .\build.ps1 -SkipTests   bỏ qua kiểm thử
param(
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Root

function Say($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }
function Fail($msg) { Write-Host "Lỗi: $msg" -ForegroundColor Red; exit 1 }

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Fail "chưa có cargo. Cài Rust tại https://rustup.rs"
}
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "Cảnh báo: chưa có ffmpeg trong PATH. Ứng dụng vẫn chạy nhưng không xem trước / xuất được video." -ForegroundColor Yellow
    Write-Host "         Cài bằng:  winget install Gyan.FFmpeg" -ForegroundColor Yellow
}

if (-not $SkipTests) {
    Say "Chạy kiểm thử"
    cargo test --release
    if ($LASTEXITCODE -ne 0) { Fail "kiểm thử thất bại" }
}

Say "Biên dịch bản phát hành"
cargo build --release
if ($LASTEXITCODE -ne 0) { Fail "biên dịch thất bại" }

$dist = Join-Path $Root "dist\viedit-windows-x64"
if (Test-Path $dist) { Remove-Item $dist -Recurse -Force }
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Copy-Item "target\release\viedit.exe" $dist
Copy-Item "assets" $dist -Recurse
Remove-Item (Join-Path $dist "assets\library\README.md") -ErrorAction SilentlyContinue
Copy-Item "README.md", "LICENSE" $dist

# Kèm FFmpeg nếu tìm thấy, để bản phát hành chạy độc lập
$ff = Get-Command ffmpeg -ErrorAction SilentlyContinue
if ($ff) {
    $ffDir = Split-Path $ff.Source
    $binDir = Join-Path $dist "ffmpeg\bin"
    New-Item -ItemType Directory -Force -Path $binDir | Out-Null
    Copy-Item (Join-Path $ffDir "ffmpeg.exe") $binDir -ErrorAction SilentlyContinue
    Copy-Item (Join-Path $ffDir "ffprobe.exe") $binDir -ErrorAction SilentlyContinue
}

Say "Đóng gói ZIP"
$zip = Join-Path $Root "dist\viedit-windows-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$dist\*" -DestinationPath $zip

$size = "{0:N1} MB" -f ((Get-Item $zip).Length / 1MB)
Say "Xong: $zip ($size)"
Write-Host ""
Write-Host "Chạy thử:  $dist\viedit.exe"

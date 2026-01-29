# 发布新版本脚本
# 用法: .\scripts\release.ps1 -Version "0.2.0"

param(
    [Parameter(Mandatory=$true)]
    [string]$Version
)

# 验证版本号格式
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "版本号格式错误，请使用 x.x.x 格式（如 0.2.0）"
    exit 1
}

Write-Host "准备发布版本 v$Version..." -ForegroundColor Cyan

# 更新 package.json 版本
$packageJson = Get-Content "package.json" -Raw | ConvertFrom-Json
$packageJson.version = $Version
$packageJson | ConvertTo-Json -Depth 10 | Set-Content "package.json" -Encoding UTF8

# 更新 tauri.conf.json 版本
$tauriConf = Get-Content "src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
$tauriConf.version = $Version
$tauriConf | ConvertTo-Json -Depth 10 | Set-Content "src-tauri\tauri.conf.json" -Encoding UTF8

# 更新 Cargo.toml 版本
$cargoToml = Get-Content "src-tauri\Cargo.toml" -Raw
$cargoToml = $cargoToml -replace 'version = "\d+\.\d+\.\d+"', "version = `"$Version`""
Set-Content "src-tauri\Cargo.toml" $cargoToml -Encoding UTF8

# 更新 App.vue 中的版本号
$appVue = Get-Content "src\App.vue" -Raw
$appVue = $appVue -replace "const currentVersion = '[^']+';", "const currentVersion = '$Version';"
Set-Content "src\App.vue" $appVue -Encoding UTF8

Write-Host "版本号已更新到 v$Version" -ForegroundColor Green

# 提交更改
Write-Host "提交版本更改..." -ForegroundColor Cyan
git add .
git commit -m "chore: bump version to v$Version"

# 创建标签
Write-Host "创建标签 v$Version..." -ForegroundColor Cyan
git tag -a "v$Version" -m "Release v$Version"

Write-Host ""
Write-Host "完成！请执行以下命令推送到 GitHub 触发自动构建：" -ForegroundColor Green
Write-Host "  git push origin main --tags" -ForegroundColor Yellow

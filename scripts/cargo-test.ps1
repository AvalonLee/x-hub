# cargo test 包装：修复 Windows 上测试 exe 无法启动的问题（STATUS_ENTRYPOINT_NOT_FOUND）。
#
# 根因：tauri/wry 导入的 comctl32!TaskDialogIndirect 只存在于 Common-Controls v6 程序集，
# 而 cargo 的测试 exe 不经 tauri-build 的 manifest 嵌入（后者只作用于 bin 目标），
# 缺激活上下文时 loader 用 comctl32 5.82 解析导入，进程启动即死。
# 处理：cargo test --no-run 后给每个测试 exe 嵌入 src-tauri/windows/app.manifest（幂等），
# 然后**直接运行这些 exe**——不能再把控制权交回 cargo：它会因 exe 被外部修改而重链，
# 把刚嵌进去的 manifest 冲掉（详见文末第 3 步的实测记录）。
param(
  [string[]]$TestArgs = @()
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'src-tauri\windows\app.manifest'
$cargoToml = Join-Path $root 'src-tauri\Cargo.toml'

# 定位 mt.exe（Windows SDK 自带；有 Rust msvc 工具链必然有 SDK）
$mt = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\mt.exe' -ErrorAction SilentlyContinue |
  Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $mt) { throw '未找到 Windows SDK mt.exe' }

# 1) 构建测试产物（不运行），从 cargo JSON 输出提取测试 exe 路径
$artifacts = cargo test --manifest-path $cargoToml --no-run --message-format=json @TestArgs 2>$null |
  ForEach-Object {
    try { $j = $_ | ConvertFrom-Json } catch { return }
    if ($j.reason -eq 'compiler-artifact' -and $j.executable -and $j.profile.test) { $j.executable }
  } | Sort-Object -Unique
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

# 2) 给每个测试 exe 嵌入激活上下文（对已嵌入的 exe 覆盖 RT_MANIFEST，幂等）
foreach ($exe in $artifacts) {
  & $mt -nologo -manifest $manifest -outputresource:"$exe;#1" | Out-Null
  if ($LASTEXITCODE -ne 0) { throw "manifest 嵌入失败: $exe" }
}
Write-Host ("已为 {0} 个测试 exe 嵌入 Common-Controls v6 manifest" -f @($artifacts).Count)

# 3) 运行测试：**直接跑刚嵌过 manifest 的测试 exe，不要经过 cargo**。
#    实测（2026-09-22）：`cargo test` 会检测到 exe 被 mt 改过（mtime 变了）→ 判定产物过期 →
#    **重新链接** → 把嵌好的 RT_MANIFEST 覆盖回默认 → 又缺激活上下文，测试进程
#    STATUS_ENTRYPOINT_NOT_FOUND 启动即死（与源码无关，纯构建链路问题）。
#    直接执行 exe 绕开重链，等价于 `cargo test` 的运行阶段。
#    TestArgs 里的 cargo 选择器（--lib / --bins / --test …）已在第 1 步消费；
#    只有 `--` 之后的 harness 参数（--nocapture / --test-threads …）才转发给 exe。
$harnessArgs = @()
$afterDash = $false
foreach ($a in $TestArgs) {
  if ($a -eq '--') { $afterDash = $true; continue }
  if ($afterDash) { $harnessArgs += $a }
}
# 默认单线程：`commands::tests` 有两个平台额度用例共享**进程内全局轮询游标**
# （`chat::PLATFORM_RR`，见「平台免费额度」一节），并行跑会互相推游标，导致
# `legacy_session_with_concrete_platform_name_still_load_balances` 的 `assert_ne!` 偶发失败
# （实测 2026-09-22：多线程 238/239，单线程 239/239 —— 与源码无关的用例间竞态）。
# 要并行自己传：`npm run tauri:test -- -- --test-threads=8`。
if ($harnessArgs.Count -eq 0) { $harnessArgs = @('--test-threads=1') }
$failed = 0
foreach ($exe in $artifacts) {
  Write-Host ("--- 运行测试 exe：{0} ---" -f $exe)
  & $exe @harnessArgs
  if ($LASTEXITCODE -ne 0) { $failed = $LASTEXITCODE }
}
exit $failed

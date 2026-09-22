# x-hub 本地开发环境搭建与验证指南

> 面向**二次开发**的动手即用手册：环境依赖梳理 → 一次性搭建 → 日常启动 → 验证清单 → 排错。
>
> | 项 | 值 |
> | --- | --- |
> | 仓库 | `AvalonLee/x-hub`（本地路径 `E:/Download/xhub`） |
> | 分支 | `master`（唯一分支，`origin/HEAD -> origin/master`，即默认开发分支） |
> | 版本 | `0.6.5` |
> | 基线 commit | `d957223`（`docs(release): v0.6.5 发布说明并入本轮审查修复条目…`） |
> | 技术栈 | Tauri 2 + Vue 3 + TypeScript 6 + Rust + 本地 SQLite |
> | 文档日期 | 2026-09-22 |

---

## 1. TL;DR 快速开始

> 假设**环境已就绪**（Node、Rust、MSVC、WebView2 均满足，见第 2、4 节）时的最短路径。

```bash
cd E:/Download/xhub

# 1) 安装前端依赖（有 package-lock.json，优先 ci 保证可复现）
npm ci

# 2) 前端类型检查 + 构建（跑完即退，用于验证环境）—— 约 15 秒
npm run build

# 3) 日常开发二选一：
npm run dev          # 仅调前端：Vite 开发服务器 http://localhost:1420（浏览器预览，无桌面能力）
npm run tauri:dev    # 整机桌面开发窗口（首次会编译全部 Rust 依赖；本机实测首次全量编译约 2–5 分钟）
```

- **本机当前状态（2026-09-22 深夜 · 最终实测）**：**Rust 1.98.1 与 MSVC 14.44.35207（+ Windows SDK 10.0.26100.0）均已安装**，桌面链路条件**齐备且已全部跑通**。前端链路（`npm ci` / `npm run build` / `npm run dev`）**已跑通**；Rust 侧**无需任何 `RUSTFLAGS`、无需改一行源码**：`cargo check` **`EXIT 0`**（19.96s）、`cargo build` **`EXIT 0`**（3m12s）→ 链接出 `src-tauri/target/debug/app.exe`（**27,945,984 B**），该二进制此前已实测可正常启动（数据库 / 托盘 / 全局快捷键 / 扩展反向代理均初始化，**无 panic**）。✅ **先前「必须加 `RUSTFLAGS='--cfg has_std'`」的说法已作废** —— 那是 `target/` 缓存污染造成的假象，定向清理即修复（详见 7.9 / 7.10 / 7.11）。
- **⚠️ 本机环境缺陷（QA 已证伪「沙箱特有」说法；非沙箱特有）**：本机存在一个**进程级缺陷**——以 `Stdio::piped()` 为**子进程的 stdin** 创建管道时**稳定失败**，报 `os error 231`（`ERROR_PIPE_BUSY`，「所有的管道范例都在使用中」）。它直接导致：
  - `autocfg`（`indexmap 1.9.3` 的 `build.rs`）无法用管道把探针源码喂给 `rustc` → 未发出 `cargo:rustc-cfg=has_std` → `indexmap 1.9.3` 缺 `has_std` → `schemars 0.8.22` 报 `error[E0107]` → **CI 原样命令 `cargo check` 失败（EXIT 101）**；
  - `tauri-cli` 派生 `npm run dev` 失败 → `npm run tauri:dev` panic `failed to run \`npm run dev\``，无法启动。
  **⚠️ 归因更正**：早期曾把该失败判为「**本机环境缺陷 / 最可疑火绒（Huorong）HIPS**」，并据此设计了 A～D 四条绕过路径 —— **该归因与四条路径均已作废**（详见 §7.12）。真实成因有两个，**都不在本机环境**：① **`target/` 构建缓存被污染**（修法：`cargo clean -p indexmap`，见 7.9）；② WorkBuddy 沙箱注入 `tsbx.dll`，导致子进程 stdin 管道创建失败（**出该进程树即消失，机器与火绒均无问题**，见 7.10）。另有一条独立坑：脚本里若把变量命名为 `RC`，会破坏 `rc.exe` 查找（见 7.11）。
- **前端开发不需要 Rust**：只改 `src/` 下的 Vue/TS，用 `npm run dev` + `npm run build` 即可闭环（`npm run build` 的 `vue-tsc` 会做全量类型检查）。

---

## 2. 环境依赖总表

> 「本机现状」列为 2026-09-22 在开发机（Windows，Git Bash 沙箱）实测结果。

| 组件 | 版本要求 | 本机现状 | 是否必需 | 获取方式 |
| --- | --- | --- | --- | --- |
| **Node.js** | `^20.19.0 \|\| >=22.12.0`（Vite 8 硬性要求）；CI 用 20 | ✅ v22.22.2 | **必需**（前端全链路） | winget / 官网 LTS；本机已含 workbuddy 托管版 |
| **npm** | 随 Node（10.x） | ✅ 10.9.7 | **必需** | 随 Node 安装 |
| **Rust 工具链**（rustc/cargo/rustup） | `rust-version = 1.77.2`，实际用 stable | ✅ **1.98.1**（`rustc`/`cargo` 均 `1.98.1`；host `x86_64-pc-windows-msvc`） | **必需**（仅桌面 `tauri:dev`/`tauri:build`/`tauri:test`） | 已装；可执行文件在 `~/.cargo/bin`（**不在 Bash 会话 PATH，需自行 `export PATH="$HOME/.cargo/bin:$PATH"`**，见 4.2） |
| **MSVC 构建工具**（VS Build Tools + Windows SDK） | VS 2022 Build Tools，MSVC v143 + Win10/11 SDK | ✅ **MSVC 14.44.35207** + **Windows SDK 10.0.26100.0**（VS 2022 **BuildTools**：`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`） | **必需**（Rust `*-pc-windows-msvc` 目标链接所需） | 已装（见 4.3） |
| **WebView2 Runtime** | 任意现代版（Win11 自带） | ✅ 153.0.4234.48 | **必需**（Tauri 渲染内核，仅运行时） | Win10/11 自带 / [官方引导](https://developer.microsoft.com/microsoft-edge/webview2/) |
| **PowerShell 7（pwsh）** | 7.x | ✅ **7.6.6**（实测 `pwsh -v` → `7.6.6`；路径 `%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe`，Store 版） | **可选**（仅 `npm run tauri:test` 需要） | 已装（Store 版）；缺失时装 MSI / winget（见 4.5） |
| **git** | 任意现代版 | ✅ 2.55.0.windows.3 | 必需（版本控制） | 已安装 |
| **gh（GitHub CLI）** | 任意 | ✅ 已安装且已登录 | 可选（推送凭据 + 发版辅助） | 已安装 |
| 磁盘 | ≥ 5 GB 空闲（含 MSVC 体积） | ✅ E: 432G / C: 271G | — | — |

### 语言 / 运行时版本要求（源码核实）

| 项 | 来源 | 值 |
| --- | --- | --- |
| Node.js | `node_modules/vite/package.json` → `engines`（实测） | `^20.19.0 \|\| >=22.12.0` |
| Node.js（文档口径） | `README.md` 环境要求 | 「Node.js 18+」（**偏旧**，实际按 Vite 8 的 20.19+/22.12+ 为准） |
| Node.js（CI 口径） | `.github/workflows/ci.yml` | `node-version: 20` |
| TypeScript | `package.json` `devDependencies` | `~6.0.2`（实测装上 6.0.3） |
| Rust | `src-tauri/Cargo.toml` → `rust-version` | `1.77.2` |
| Rust edition | `src-tauri/Cargo.toml` → `edition` | `2021` |
| Rust（CI 口径） | `.github/workflows/ci.yml` | `dtolnay/rust-toolchain@stable` |
| Vite | `package.json` | `^8.2.0`（实测 8.2.0） |

> ⚠️ **Node 版本结论**：本机 v22.22.2 满足 Vite 8 要求（`>=22.12.0`）。若换机，请装 **Node 20.19+ 或 22.12+**；README 写的「18+」对 Vite 8 已不够。

### 包管理器

- **前端：npm**（仓库根有 `package-lock.json` → npm 是约定包管理器）。**不使用 pnpm / yarn**，本机也未安装。
- **Rust：cargo**（自带于 rustup 工具链）。
- 优先 `npm ci`（严格按 lockfile，可复现）；仅当 `ci` 因 lockfile 与环境不匹配失败时才回退 `npm install`。

### 数据库

- **本地 SQLite，无需单独安装数据库服务**。`src-tauri/Cargo.toml` 用 `rusqlite = { version = "0.37", features = ["bundled", "backup"] }`，`bundled` 特性表示**编译期自带 SQLite 源码**，运行时不需要系统 libsqlite。
- **数据落盘位置**（由 `src-tauri/src/paths.rs` 解析，启动时确定一次）：
  - **标准版（默认）**：`%APPDATA%\x-hub\`
    - 数据库：`%APPDATA%\x-hub\app.db`（WAL 模式）
    - 配置：`%APPDATA%\x-hub\app.json`
    - 日志：`%APPDATA%\x-hub\logs\x-hub.log`
    - 数据路径引导文件：`%APPDATA%\x-hub\data_path.json`（记录用户改过的数据根）
    - AI 密钥回退文件：`%APPDATA%\x-hub\chat_keys.json`（**仅当系统钥匙串不可用时才生成，明文 JSON**，见 3.2）
  - **便携版**（`exe` 同目录放一个空文件 `portable`）：数据固定为 `exe\data\`（不支持改路径）。

---

## 3. 第三方服务与外部依赖说明

> 以下全部**从源码核实**（非照抄 README）。结论按三类划分，明确哪些阻塞开发、哪些不阻塞。

### 3.1 开发 / 构建**必需**

| 依赖 | 说明 | 证据 |
| --- | --- | --- |
| Node.js + npm | 前端构建、类型检查、Vite dev server | `package.json` scripts |
| Rust stable + MSVC | 编译 `src-tauri`（桌面壳 + 全部后端逻辑） | `src-tauri/Cargo.toml` |
| WebView2 Runtime | Tauri 运行时渲染内核（仅 Windows 桌面运行时用） | `tauri.conf.json` / `README.md` |
| 本地 SQLite | `rusqlite` **bundled**，编译期自带，**无需安装服务** | `Cargo.toml` `features = ["bundled"]` |

**开发/构建不需要**任何 API Key、不需要联网访问业务后端即可完成：`npm ci` → `npm run build` → `cargo check` → 打开桌面窗口。

### 3.2 运行时**可选**（需用户自配 Key / 联网；缺失时自动降级，**不阻塞开发**）

| 能力 | 外部服务 / 地址（源码核实） | Key / 配置位置 | 缺失时的降级 |
| --- | --- | --- | --- |
| **天气** | Open-Meteo：`https://api.open-meteo.com/v1/forecast`；地理编码 `https://geocoding-api.open-meteo.com/v1/search`（`src-tauri/src/online.rs`） | 无 Key，仅需联网；城市可设（`config.weather_city`），内置中国城市表离线可命中 | 离线时天气卡片不刷新 |
| **IP 定位** | `http://ip-api.com/json/`（`online.rs`） | 无 Key | 定位失败则要求手动选城市 |
| **连通性探活** | `https://www.baidu.com`（`online.rs::check_connectivity`） | 无 Key | 判定为离线 |
| **名言金句** | hitokoto：`https://v1.hitokoto.cn/`（`online.rs`） | 无 Key；`config.quote_source` 可选 `online`/本地 | **自动回退本地语料** `src/utils/quotes.ts` |
| **AI 对话** | OpenAI 兼容 SSE（默认预置 `https://api.deepseek.com/v1`，可改 OpenAI / Ollama / one-api 等，`config.rs::default_chat_models`） | API Key **优先存系统钥匙串（keyring）**，界面脱敏；**钥匙串不可用时会回退写数据根下 `chat_keys.json`（明文 JSON）**（`chat.rs::save_api_key`） | 不配 Key 则 AI 面板不可用，其余功能不受影响 |
| **扩展市场** | 平台服务端 `https://x-hub.xfactor.top/api/v1/market/registry`（`config.rs`，v0.6.1 起经服务端代理，客户端不再直连 COS） | 无需 Key，Ed25519 验签 | 拉不到清单则仅显示本地已装扩展 |
| **应用自动更新** | 平台服务端 `https://x-hub.xfactor.top/api/v1/app/update`（`config.rs`，升级包分发在腾讯云 COS，客户端只认服务端清单） | 无需 Key，Ed25519 分离签名验签 | 检查失败静默跳过，不阻塞使用 |
| **扩展 service 后端运行时** | 系统 Node 优先；缺失时**按需下载内置 Node** `https://nodejs.org/dist/v24.9.0/node-v24.9.0-win-x64.zip`（`src-tauri/src/runtime.rs`） | 无 Key | 系统有 Node 即用系统 Node；扩展 service 不可用时自动降级 |
| **系统级依赖** | WebView2；Windows 注册表（`winreg` 开机自启 Run 键）；系统钥匙串（`keyring`） | — | 平台相关功能在非 Windows 上 `#[cfg]` 隔离 |

> 说明：`keyring` 依赖已启用 `windows-native` 等特性（`Cargo.toml`），API Key 默认走系统凭据管理器（Windows Credential Manager），**不含任何硬编码密钥**；**仅当钥匙串写入失败时**才回退到数据根下的明文 `chat_keys.json`（见上表与 7.6，`chat.rs` 有该兜底分支）。以上服务全部是「有则更好」的运行时能力，**本地开发与构建不阻塞**。

### 3.3 **仅发布 / 运维流程**需要

| 脚本 | 用途 | 依赖 |
| --- | --- | --- |
| `scripts/publish-release.ps1` / `scripts/upload-release.ps1` | 发布应用升级包到分发源 | pwsh 7 + **rclone**（`winget install --id Rclone.Rclone`）+ COS 凭据（默认通道 `-Target cos`；`-Target sftp` 为备选） |
| `scripts/publish-extension.ps1` / `scripts/upload-market.ps1` | 发布扩展 / 上传市场清单 | pwsh 7 + **rclone** + 平台服务端凭据 |
| `scripts/pub-sign.mjs` | Ed25519 签名 | Node |
| `scripts/verify-runtime.ps1` | 运行时/发布产物校验 | pwsh |
| `scripts/server/*` | 自托管分发服务端 | 独立环境 |
| `scripts/cargo-test.ps1`（经 `npm run tauri:test`） | Rust 单元测试包装（Windows 必须走） | **pwsh 7** + Windows SDK `mt.exe` |

> **日常开发（改代码 + 本地跑起来）不需要以上任何脚本**；它们只在「发版 / 分发 / CI 之外的手工运维」时用。

### 3.4 平台构建依赖说明

- **MSVC 是硬依赖**：Rust 默认 target 为 `x86_64-pc-windows-msvc`，链接 `link.exe`、生成资源与 manifest 都依赖 MSVC 工具链 + Windows SDK。缺它会报 `link.exe not found` 或 `msvc` 相关错误。**本机已安装并实测可用**（`MSVC 14.44.35207` + `Windows SDK 10.0.26100.0`；实测 `rustc` 能直接产出可执行文件、`cargo build` 能链接出 27 MB 的桌面二进制，见附录 A）。
- **关于 PATH 上的 `link.exe` 遮蔽**：Git 自带 `…/PortableGit/…/usr/bin/link.exe`（GNU coreutils 的硬链接工具）确实在同名上遮蔽了 MSVC 链接器，但 **rustc 自带的 MSVC 探测（经 `cc`/`vswhere`）会解析出 MSVC `link.exe` 的绝对路径再调用**，因此**未出现**任何链接器报错、**无需**手工前置 MSVC 路径或设置 `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`。
- `scripts/cargo-test.ps1` 还会用 **Windows SDK 的 `mt.exe`**（`C:\Program Files (x86)\Windows Kits\10\bin\*\x64\mt.exe`）给测试 exe 嵌入 Common-Controls v6 manifest——所以跑 `npm run tauri:test` 需要「pwsh 7 + Windows SDK」两者齐备。

### 3.5 CI 对照（最小验证门禁）

`.github/workflows/ci.yml` 在 `windows-latest` 上执行：

```yaml
- Setup Node.js:  node-version: 20
- Setup Rust:     dtolnay/rust-toolchain@stable
- npm ci                          # 安装前端依赖（可复现）
- npm run build                   # vue-tsc 类型检查 + vite build
- cargo check --manifest-path src-tauri/Cargo.toml   # Rust 编译检查（不产出二进制）
```

> **把这三条命令当作「最小验证门禁」**：本地改完代码，依次跑 `npm ci`（首次/依赖变动时）→ `npm run build` → `cargo check --manifest-path src-tauri/Cargo.toml`，全绿即达到 CI 的通过标准。
>
> 注意：CI **只跑 `cargo check`**（编译检查，不链接、不打包），所以 CI 环境靠 `windows-latest` 自带的 MSVC。**本机 Rust + MSVC 已齐备，且该命令已实测 `EXIT 0`**（2026-09-22 22:34，19.96s）。⚠️ **若你在 WorkBuddy 内执行却报 `schemars` E0107，那是两件事叠加**：WorkBuddy 注入层让 `autocfg` 探针失败（见 7.10），而这份错误结果又被写进 `target/` 缓存（见 7.9）→ **请改用普通 CMD 执行，或先 `cargo clean -p indexmap`**。**不要**再用 `RUSTFLAGS` 绕过。

### 3.6 脚本对 PowerShell 7 的依赖

- `npm run tauri:test` → `pwsh -NoProfile -File scripts/cargo-test.ps1`：**需要 PowerShell 7（`pwsh`）**，Windows 自带的 5.1（`powershell.exe`）**不满足**该命令（命令名是 `pwsh`）。
- 其余 `scripts/*.ps1` 属发布/运维脚本（见 3.3），非日常开发必需。

---

## 4. 一次性环境搭建步骤

> **本机现已全部满足**：4.2（Rust）、4.3（MSVC）**均已安装**（本轮实测确认），仅需按下文「验证」命令校验；4.1 / 4.4 / 4.5 亦已就绪。下列各节的**安装命令保留作换机参考**（换机时 4.2 / 4.3 需手动执行、含管理员 UAC 提权）。

### 4.1 Node.js 与 npm（本机已满足）

```bash
node -v    # 期望：v22.22.2（或任意满足 ^20.19.0 || >=22.12.0 的版本）
npm -v     # 期望：10.9.7
```

若需新装：`winget install --id OpenJS.NodeJS.LTS`（或官网 https://nodejs.org/ 下载 LTS）。

### 4.2 Rust 工具链安装（**本机已安装，仅需校验**）

> **本机状态（2026-09-22 实测）**：已安装且可用，**无需再装**。`rustc 1.98.1 (48a229cea 2026-09-01)` / `cargo 1.98.1 (797e8a9bc 2026-08-05)`；`rustup show` → 默认 toolchain `stable-x86_64-pc-windows-msvc`，host `x86_64-pc-windows-msvc`。
> ⚠️ **实操要点**：可执行文件在 `~/.cargo/bin`（`rustc.exe`/`cargo.exe`/`rustup.exe`…），但**该目录不在 Git Bash 会话的 PATH 上**——Bash 里直接 `rustc -V` 会 `command not found`。**Bash 命令前需先** `export PATH="$HOME/.cargo/bin:/usr/bin:/bin:$PATH"`。
> 下列安装命令保留作**换机参考**。

**途径 A：winget（推荐）**

```powershell
winget install --id Rustlang.Rustup
# 新开一个终端，让 PATH 生效，然后：
rustup default stable-msvc
```

**途径 B：官方安装器**

1. 下载并运行 `rustup-init.exe`：<https://win.rustup.rs/x86_64>
2. 交互界面选 `1) Proceed with standard installation (default - just press enter)`（默认 host 即 `x86_64-pc-windows-msvc`）
3. 若已装 rustup 但 host 不对，执行：`rustup default stable-msvc`

**验证**（新开终端）：

```powershell
rustc -V                   # 期望：rustc 1.7x.x（>= 1.77.2）
cargo -V                   # 期望：cargo 1.7x.x
rustup show                # 期望：default host 为 x86_64-pc-windows-msvc
cargo --version --verbose  # 查看 host 目标
```

**本机实测输出**（Bash，已补 PATH）：

```
$ rustc -V
rustc 1.98.1 (48a229cea 2026-09-01)
$ cargo -V
cargo 1.98.1 (797e8a9bc 2026-08-05)
$ rustup show
Default host: x86_64-pc-windows-msvc
installed toolchains / active toolchain:
  stable-x86_64-pc-windows-msvc (active, default)
$ which rustc
/c/Users/avalo/.cargo/bin/rustc
```

- **体积 / 耗时量级**：rustup 本体约 10 MB；首次 `cargo build` 会拉取并编译约数百个 crate。**本机实测首次全量编译约 2–5 分钟**（`cargo check` `2m20s`、`cargo build` `3m07s`；生成 `src-tauri/target/` 约 6 GB）。

### 4.3 MSVC 构建工具安装（**本机已安装，仅需校验**）

> **本机状态（2026-09-22 实测）**：已安装，**无需再装**。Visual Studio 2022 **Build Tools**（`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`），`VC/Tools/MSVC/**14.44.35207**`（含 `bin/Hostx64/x64/link.exe`）；Windows SDK `C:\Program Files (x86)\Windows Kits\10`，`Include/10.0.26100.0`，`bin/` 下有 `10.0.26100.0`（含 `mt.exe`，另有若干旧版本）。实测 `cargo build` 可正常链接（见 6.5 / 附录 A）。下列安装命令保留作**换机参考**。

**途径 A：winget（一条命令，仍会弹 UAC）**

```powershell
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

**途径 B：手动（图形安装器）**

1. 下载 **Visual Studio 2022 Build Tools**：<https://visualstudio.microsoft.com/visual-cpp-build-tools/>
2. 运行安装器 → 「工作负载」勾选 **「使用 C++ 的桌面开发」（Desktop development with C++）**
3. 确保右侧「安装详细信息」包含：
   - **MSVC v143 - VS 2022 C++ x64/x86 生成工具**
   - **Windows 10 SDK**（或 Windows 11 SDK）
   - 默认勾选的 **C++ CMake 工具**（`--includeRecommended` 会带上）
4. 安装（**需管理员权限**；**体积约 3–7 GB**，视勾选项而定；**耗时约 10–30 分钟**）

**验证**（新开「x64 Native Tools Command Prompt for VS 2022」，或普通终端确认目录存在）：

```powershell
# 确认 SDK 的 mt.exe（cargo-test.ps1 需要）
Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\mt.exe' | Select-Object -First 1

# 在「x64 Native Tools」命令提示符里：
cl        # 期望：打印 MSVC 编译器版本（Microsoft (R) C/C++ ...）
```

**本机实测输出**（目录探测）：

```
VC/Tools/MSVC/                      -> 14.44.35207
…/14.44.35207/bin/Hostx64/x64/link.exe   -> 存在
Windows Kits/10/Include/            -> 10.0.26100.0
Windows Kits/10/bin/                -> 10.0.14393.0 … 10.0.26100.0（含 mt.exe）
```

**本机已装好**：`cargo build --manifest-path src-tauri/Cargo.toml` 实测可正常编译并链接（见 6.5 / 附录 A）。

### 4.4 WebView2 Runtime（本机已装）

```powershell
ls "C:\Program Files (x86)\Microsoft\EdgeWebView\Application"
# 本机实测：153.0.4234.48

# 或用 winget 列出：
winget list --id Microsoft.EdgeWebView2Runtime
```

Win10/11 通常自带；缺失时从官方安装：<https://developer.microsoft.com/microsoft-edge/webview2/>

### 4.5 可选：PowerShell 7（仅 `npm run tauri:test` 需要；**本机已装 7.6.6，无需再装**）

```powershell
pwsh -v    # 本机实测：7.6.6（路径 %LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe，Store 版）
# 仅当目标机缺装时才需要（任选其一）：
winget install --id Microsoft.PowerShell
```

### 4.6 git 推送凭据配置

> **背景（实测）**：`~/.ssh/id_ed25519` 公钥**未在 GitHub 注册**（`~/.ssh/config` 已把 `github.com` 映射到 `ssh.github.com:443`，但 SSH 拉取仍报 `Permission denied (publickey)`）。仓库已用 **HTTPS** 克隆成功。若要 `git push`，二选一：

**方案 A：改用 HTTPS + 凭据管理器（推荐，本机已基本就绪）**

本机已满足以下条件（实测）：

- remote 为 HTTPS：`origin  https://github.com/AvalonLee/x-hub.git`
- 已装 GitHub CLI `gh`（路径 `C:\Program Files\GitHub CLI\gh`）
- `gh auth status` 显示：**已登录 `AvalonLee`（仓库 owner）**，协议 https，Token 含 `repo` + `workflow` scope
- git 凭据助手为 `git-credential-manager`；`git ls-remote origin HEAD` 实测**成功**（EXIT=0）

因此 HTTPS 推送**开箱即用**。若某台机器没配好：

```bash
gh auth login                                   # 交互登录 GitHub
gh auth setup-git                               # 让 git 使用 gh 作为 HTTPS 凭据助手
git remote set-url origin https://github.com/AvalonLee/x-hub.git
git ls-remote origin HEAD                       # 验证读权限
```

**方案 B：注册 SSH 公钥**

```bash
cat ~/.ssh/id_ed25519.pub     # 复制输出
```

把公钥内容粘贴到 <https://github.com/settings/keys>（New SSH key）。然后：

```bash
ssh -T git@github.com                        # 期望：Hi AvalonLee! You've successfully authenticated...
git remote set-url origin git@github.com:AvalonLee/x-hub.git
git ls-remote origin HEAD                    # 验证
```

> ⚠️ 本机 `~/.ssh/config` 已把 `github.com` 指向 `ssh.github.com:443`——这是为受限网络（22 端口被封）准备的，属正常配置，无需改动。

---

## 5. 日常开发启动流程

| 目标 | 命令 | 说明 |
| --- | --- | --- |
| 仅前端调试 | `npm run dev` | Vite dev server，`http://localhost:1420`。浏览器预览，**无 Tauri 能力**（`window.__TAURI_INTERNALS__` 不存在，代码以 `isTauri()` 守卫降级）。改 `src/**` 热更新。 |
| 整机桌面开发 | `npm run tauri:dev` | 启动完整桌面窗口（真实 Rust 后端 + SQLite + WebView2）。`beforeDevCommand` 会先自动跑 `npm run dev`。**首次编译全部 Rust 依赖，本机实测约 3 分钟**（`cargo check` 19.96s、`cargo build` 3m12s），之后增量秒级。⚠️ **请在普通 CMD / 终端里执行** —— 在 WorkBuddy 内跑会因注入层导致 tauri-cli 派生 `npm run dev` 失败（见 7.10，**非本机缺陷**）。亦可用 `cargo build` + 直接运行 `app.exe` 替代验证（见 6.5）。 |
| 类型检查 + 构建（验证用） | `npm run build` | `vue-tsc -b && vite build`，跑完即退。**日常改完代码的推荐验证手段**。 |
| 桌面安装包 | `npm run tauri:build` | 产物见第 8 节。 |
| Rust 单元测试 | `npm run tauri:test` | Windows **必须**走此包装脚本（见第 9 节 FAQ），需 pwsh 7 + Windows SDK。 |

> ⚠️ **纪律（来自 `AGENTS.md` 约定，踩过坑）**：**不要在前台跑 watch 类命令**（`npm run dev` / `tauri:dev` 会占死终端直到超时）。验证一律用跑完即退的 `npm run build`；确需起 dev server 时**后台分离启动 → 轮询端口就绪 → 测完必杀 → 复查端口释放**。启动前先查 `1420` 是否被占用。

---

## 6. 验证清单

> 分层验证，每项给出命令 + 期望结果 + 失败时怎么查。

### 6.1 环境校验

```bash
node -v                                   # v22.22.2 或 >=22.12.0
npm -v                                    # 10.x
rustc -V && cargo -V && rustup show       # 已装 Rust，host = x86_64-pc-windows-msvc
pwsh -v                                   # 可选；PowerShell 7.x
ls "C:\Program Files (x86)\Microsoft\EdgeWebView\Application"   # 有版本号目录
```

**失败排查**：命令 not found → PATH 未生效，重开终端；`rustup show` host 非 msvc → `rustup default stable-msvc`。

### 6.2 前端：依赖安装

```bash
cd E:/Download/xhub
npm ci
```

**期望**：`added N packages`，退出码 0。
**本机实测**：`added 303 packages in 2m`，`EXIT_CODE=0`。

**失败排查**：网络慢 → 换源（见 FAQ 6.6）；lockfile 冲突 → 回退 `npm install` 并检查 `package-lock.json` 是否被改动。

### 6.3 前端：类型检查 + 构建

```bash
npm run build
```

**期望**：`prebuild` 两脚本通过 → `vue-tsc -b` 无类型错误 → `vite build` 输出 `✓ built in ...`，退出码 0，生成 `dist/`。
**本机实测**：

```
[settings-index] 已是最新（46 项 / 13 个分区）
[settings-css] 前缀口径正常（157 条顶层规则）
✓ 3628 modules transformed.
✓ built in 2.34s
EXIT_CODE=0   DURATION_SEC=15
```

**失败排查**：`vue-tsc` 报类型错误 → 按行号修 TS/Vue；`prebuild` 失败 → 单独跑 `npm run gen:settings-index` / `npm run check:settings-css` 看具体报错。

### 6.4 Rust：编译检查（本机已具备 Rust + MSVC）

```bash
# 沙箱/Git Bash 需先补 PATH
export PATH="$HOME/.cargo/bin:/usr/bin:/bin:$PATH"
cargo check --manifest-path src-tauri/Cargo.toml
```

**期望**：`Finished dev profile [unoptimized + debuginfo] target(s)`，退出码 0。

**本机实测（两种情形，均如实记录）**：

1. **直接跑（CI 原样命令）→ 本机失败（含关闭沙箱）**：`EXIT_CODE=101`，约 `123s`，报 `error[E0107]: struct takes 3 generic arguments but 2 generic arguments were supplied`，位置 `…/schemars-0.8.22/src/lib.rs:12`（`pub type Map<K, V> = indexmap::IndexMap<K, V>;`）→ `could not compile \`schemars\` (lib)`。
   - **根因（最终定论）**：**不是本机环境缺陷，与火绒无关**。真实成因有二 —— ① WorkBuddy 沙箱注入 `tsbx.dll`，使 `indexmap 1.9.3` 的 `build.rs` 里 `autocfg` 的 stdin 管道探针失败（`autocfg-1.5.1/src/lib.rs:327`），未发出 `cargo:rustc-cfg=has_std`；② 这份「缺 `has_std`」的产物**被 cargo 指纹缓存**，此后即使换到干净终端也不会重跑，于是 `indexmap` 的 `#[cfg(has_std)] IndexMap<K, V, S = RandomState>` 变体被裁掉 → `schemars` 的 `IndexMap<K, V>` 报 E0107。**修法**：`cargo clean -p indexmap --manifest-path src-tauri/Cargo.toml`，实测随即通过。详见 7.9 / 7.10。
2. ~~加 `--cfg has_std` 绕过 → 通过~~ **（已作废，请勿使用）**：早期实测该 hack 确实能让 `cargo check` 通过，但它只是**掩盖**了被污染的缓存、并非修复，还会改变缓存指纹并触发全量重编。**正确做法：`cargo clean -p indexmap` 后原样重跑**（实测 `EXIT 0`，见 7.9）。

**失败排查**：
- `link.exe not found` / `msvc` 相关 → MSVC 未装或未在 PATH（见 4.3）。**本机未出现**。
- `schemars` 的 `E0107` / `autocfg could not probe for \`std\`` → 见 7.9：**真因是 `target/` 缓存污染**，修法 `cargo clean -p indexmap --manifest-path src-tauri/Cargo.toml`（**不需要** `RUSTFLAGS`）。
- `cargo`/`rustc` not found → PATH 未补 `~/.cargo/bin`（见 7.2）。

### 6.5 桌面窗口冒烟（本机已具备 Rust + MSVC）

**方式 A（官方）：`npm run tauri:dev`**

```bash
export PATH="$HOME/.cargo/bin:/usr/bin:/bin:$PATH"
npm run tauri:dev
```

**期望**：编译完成后弹出无边框窗口（标题 `x-hub`），工作台正常渲染，托盘图标出现。
**本机实测**：**无法启动**——tauri-cli 在 `Running BeforeDevCommand (\`npm run dev\`)` 之后立即 panic。⚠️ QA 在**关闭沙箱**后重跑该命令，结果**完全相同**（rc=127，约 7s）：

```
thread '<unnamed>' panicked at crates\tauri-cli\src\dev.rs:207:31:
failed to run `npm run dev`
```

根因与 6.4 同类：**子进程 stdin 管道创建失败**（`os error 231`），tauri-cli 无法派生 `npm run dev`。**非 Rust/MSVC 问题、非本机缺陷、与火绒无关** —— 这是 **WorkBuddy 注入层（`tsbx.dll`）** 造成的，**在普通 CMD / 终端里执行即正常**。详见 7.10。

**方式 B（本机可行的等效验证）：`cargo build` + 直接运行二进制**

```bash
export PATH="$HOME/.cargo/bin:/usr/bin:/bin:$PATH"
# （已作废）本机不需要 RUSTFLAGS；旧写法 export RUSTFLAGS='--cfg has_std' 只会掩盖被污染的缓存
cargo build --manifest-path src-tauri/Cargo.toml       # 全量编译 + 链接 → 产出 app.exe
./src-tauri/target/debug/app.exe                       # 直接运行桌面二进制
```

**本机实测**：`cargo build` → `Finished dev profile [unoptimized + debuginfo] target(s) in 3m 07s`，`EXIT=0`，产出 `src-tauri/target/debug/app.exe`（**27,945,984 字节 ≈ 27 MB**，已完整链接）；运行 `app.exe` 后进程存活、启动日志完整（数据库初始化 / 系统托盘 / 全局快捷键 `Ctrl+Shift+Space`、`` Ctrl+` `` / 扩展反向代理 `127.0.0.1:5752` / 窗口状态恢复 `1400x900`，**无 panic**）。**桌面链路「编译 → 链接 → 运行」实证可用**；仅「tauri-cli 自动编排窗口」这一层受沙箱所限无法复现。

**⚠️ 补充（2026-09-22 深夜实测，5 次复现）**：**不要从 WorkBuddy 的工具进程里直接启动 `app.exe`**。进程能起来，但会**卡死在启动中期的 WebView2 窗口预创建**上：日志停在 `剪贴板格式监听已注册` 之后（即 `clipboard::init_overlay_window` → `WebviewWindowBuilder::build()`），主窗口 `Responding=False`、无 `msedgewebview2` 子进程、**无 panic / 无 ERROR / 无崩溃转储**，把 `%LOCALAPPDATA%\x-hub\EBWebView` 改名换全新 profile 也**无效**。同一二进制**换启动方式即完全正常**：

```powershell
# ✅ 可行：借 explorer 代理启动（进程挂到用户会话下，脱离工具进程树）
explorer.exe "E:\Download\xhub\src-tauri\target\debug\app.exe"
# ✅ 等价可行：普通 CMD 里执行，或资源管理器里双击 app.exe
```

**判定依据**：explorer 代理启动的会话日志跑到 `外部网页窗口已创建（隐藏常驻）: web-1` + `x-hub 启动完成`，随后 `初始化数据加载完成` / `扩展注册表扫描完成` / 市场清单刷新（6 个扩展）全部正常；工具直启的 4 次会话**全部**卡在同一位置。
**归因**：与 7.10 同源（WorkBuddy 注入层/作业对象干扰子进程创建 WebView2 环境），**非本机缺陷、与本项目代码无关** —— 卡点在既有的剪贴板浮层预创建逻辑里；时间线亦佐证：`src-tauri/src/web_window.rs` 写于 23:33，而 20:40 那次会话就已卡在该位置。

**失败排查**：白屏 → 见 FAQ 6.3；编译报错 → 检查 MSVC；窗口不弹 → 看 `%APPDATA%\x-hub\logs\x-hub.log`。

### 6.6 测试脚本（可选，需 pwsh 7 + Windows SDK）

```bash
npm run tauri:test
```

**期望**：全绿，输出 `已为 N 个测试 exe 嵌入 Common-Controls v6 manifest` + 每个 exe 一行 `test result: ok. …`。
**本机状态**：✅ **已验证通过（2026-09-22 深夜）：`239 passed; 0 failed`，脚本退出码 0**（此前记录为「未执行/未验证」）。前置条件（**pwsh 7.6.6** + **Windows SDK `mt.exe`**，后者位于 `…\Windows Kits\10\bin\10.0.26100.0\x64\`）均已在磁盘核实具备。**注意**：请在**普通 CMD / 终端**里运行；不要在脚本里使用名为 `RC` 的变量（会破坏 `rc.exe` 查找，见 7.11）；若曾在 WorkBuddy 内跑过构建，先执行 `cargo clean -p indexmap`（见 7.9）。**不需要** `RUSTFLAGS`。
**脚本行为（2026-09-22 起改了第 3 步）**：不再调用 `cargo test` 来运行测试，而是**直接执行**第 2 步已嵌好 manifest 的测试 exe —— 交给 cargo 会被它重链、把 manifest 冲掉，测试进程随即 `STATUS_ENTRYPOINT_NOT_FOUND` 启动即死（见 7.13）；同时默认加 `--test-threads=1`（既有用例间竞态，见 7.13）。
**失败排查**：`pwsh` 不存在 → 见 FAQ 7.7；`未找到 Windows SDK mt.exe` → 装 Windows SDK（见 4.3）；测试 exe 报 `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)` → 见 7.13。

---

## 7. 常见问题排查（FAQ）

### 7.1 `link.exe not found` / `cargo` 链接器报错 / `msvc` 相关错误

**根因**：缺 MSVC 工具链（Windows SDK + VS Build Tools）。
**处理**：按 **4.3** 安装 VS 2022 Build Tools（勾「使用 C++ 的桌面开发」+ MSVC v143 + Windows SDK），**重开终端**后重试。
**自查**：`get-command cl.exe`（在 x64 Native Tools 提示符里应能找到）。
**本机状态（2026-09-22 实测）**：⚠️ **本机不适用**——MSVC 已装且链接正常（见 4.3 / 6.5）。注意 Git 自带 `…/PortableGit/…/usr/bin/link.exe`（coreutils 硬链接工具）会遮蔽 PATH 上的同名 `link.exe`，但 **rustc 自带 MSVC 探测（`cc`/`vswhere`）按绝对路径调用 MSVC `link.exe`**，**未出现**该报错，**无需**手工前置 MSVC 路径或设 `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`。

### 7.2 `rustc` / `cargo` 找不到

**根因**：Rust 未装，或装完 PATH 未刷新。
**处理**：按 **4.2** 安装；确认 `%USERPROFILE%\.cargo\bin` 在 PATH；重开终端再 `rustc -V`。

### 7.3 `tauri:dev` 白屏 / 窗口空白

**可能原因与处理**：

1. **WebView2 缺失** → 装 WebView2 Runtime（4.4），`ls "C:\Program Files (x86)\Microsoft\EdgeWebView\Application"` 应存在。
2. **前端 dev server 没起来** → `beforeDevCommand` 是 `npm run dev`，若 1420 被占用会失败（见 7.5）。先确认 `http://localhost:1420` 能访问。
3. **构建产物过期** → 删除 `dist/` 后 `npm run build` 重试。
4. **看日志** → `%APPDATA%\x-hub\logs\x-hub.log`（Info 级，含所有命令入口成功/失败记录）。

### 7.4 npm 安装慢 / 超时

换国内镜像源：

```bash
npm config set registry https://registry.npmmirror.com
npm config get registry        # 确认已生效
npm ci
```

> 本机当前源：`https://registry.npmjs.org/`。换源后如需还原：`npm config set registry https://registry.npmjs.org/`。

### 7.5 Vite 端口 1420 被占用

`vite.config.ts` 设了 `port: 1420` + `strictPort: true`——**端口被占会直接失败**（不会自动换端口）。

```bash
# 查占用
netstat -ano | findstr :1420
# 结束占用进程（把 <PID> 换成上面最后一列）
taskkill /PID <PID> /F
```

**自查（本机实测）**：`netstat -ano | grep 1420` 若只剩 `TIME_WAIT`（无 `LISTENING`）表示端口已释放，可直接启动。

### 7.6 数据 / 日志 / 配置在哪

| 内容 | 路径 |
| --- | --- |
| 数据根（标准版） | `%APPDATA%\x-hub` |
| 数据根（便携版） | `exe\data`（exe 同目录需有 `portable` 空文件） |
| 数据库 | `%APPDATA%\x-hub\app.db` |
| 配置 | `%APPDATA%\x-hub\app.json` |
| **日志** | `%APPDATA%\x-hub\logs\x-hub.log` |
| AI 密钥回退文件 | `%APPDATA%\x-hub\chat_keys.json`（仅当系统钥匙串不可用时生成，**明文**） |
| 图标 | `%APPDATA%\x-hub\icons\` |
| 剪贴板图片 | `%APPDATA%\x-hub\clipboard\images\` |
| 备份包 | `x-hub-backup-<时间戳>.zip` |
| 更新包暂存 | `%APPDATA%\x-hub\updates\` |

> 可直接 `explorer %APPDATA%\x-hub` 打开数据目录。

### 7.7 `npm run tauri:test` 报 `pwsh` 不存在

**根因**：`package.json` 中 `tauri:test` = `pwsh -NoProfile -File scripts/cargo-test.ps1`，命令名是 `pwsh`（**PowerShell 7**）；Windows 自带的 `powershell.exe`（5.1）不叫这个名字，故 5.1 环境会报「找不到 pwsh」。
**本机状态**：⚠️ **不适用**——本机已装 PowerShell **7.6.6**（实测 `pwsh -v` → `7.6.6`，位于 `%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe`，Store 版），此报错不会出现。
**处理（仅当目标机缺装）**：`winget install --id Microsoft.PowerShell`（见 4.5），然后 `pwsh -v` 确认。
**补充**：直接 `cargo test` 在 Windows 上**会失败**（测试 exe 启动即死 `STATUS_ENTRYPOINT_NOT_FOUND` 0xC0000139）——必须走 `npm run tauri:test` 嵌 manifest（原理见 `AGENTS.md`「cargo test」条与 `scripts/cargo-test.ps1` 注释）。

### 7.8 `git push` 报 `Permission denied (publickey)`

**根因**：SSH 公钥未在 GitHub 注册。
**处理**：用 **4.6 方案 A**（HTTPS + gh 凭据助手，本机已就绪）或方案 B（注册 SSH 公钥）。

### 7.9 `schemars` E0107 / `autocfg could not probe for \`std\`` —— 真因是 `target/` 缓存污染（**已修复**）

> ✅ **判定更正（2026-09-22 深夜 · 实测落地）**：本节早期版本（以及附录 A / B / C）曾判定为「**本机环境缺陷**，**最可疑火绒（Huorong）HIPS**，换终端也无法解决」——**该判定已被证伪，特此作废**。
> **本机没有任何环境缺陷；火绒无需任何放行、排除或退出操作。** 真因是 **`target/` 构建缓存被污染**，用一条 `cargo clean -p indexmap` 定向清理即彻底修复（下方有决定性证据）。

**现象**

- `cargo check --manifest-path src-tauri/Cargo.toml` → `error[E0107]: struct takes 3 generic arguments but 2 generic arguments were supplied`（`schemars-0.8.22/src/lib.rs:12`：`pub type Map<K, V> = indexmap::IndexMap<K, V>;`），随后 `could not compile \`schemars\` (lib)`。
- 对应构建脚本 stderr：`warning: autocfg could not probe for \`std\``。

**真因（两个独立事实，先前被混为一谈）**

1. **触发源** —— `indexmap 1.9.3` 的 `build.rs` 用 `autocfg` 派生 `rustc` 探针子进程（`autocfg-1.5.1/src/lib.rs:327`：`command.arg("-").stdin(Stdio::piped())`，把探针源码经 **stdin 管道** 喂给子进程）。**在 WorkBuddy 沙箱内**执行 cargo 时，为子进程创建 stdin 管道稳定失败（`os error 231` = `ERROR_PIPE_BUSY`）→ 探针失败 → 未发出 `cargo:rustc-cfg=has_std`。（详见 7.10：这只是**触发**，不是"本机缺陷"。）
2. **放大源（真正的坑）** —— 这个「缺 `has_std`」的构建脚本产物**已被 cargo 指纹缓存**。此后**即使换到完全正常的终端**，cargo 仍认为 `indexmap` 产物未过期、**从不重跑** build script，于是 `has_std` 永久缺失 → `indexmap` 的 `#[cfg(has_std)] IndexMap<K, V, S = RandomState>` 变体被裁掉、只剩无默认参数的同名类型 → `schemars` 的 `IndexMap<K, V>` 报 E0107。**这才是"换了干净终端还报同样的错"的原因。**

**决定性证据（区分「环境缺陷」与「缓存污染」）**

| 观察 | 指向的结论 |
| --- | --- |
| 干净环境里 `cargo check` 的日志**只出现 `Compiling schemars`，全程没有 `Compiling indexmap`** | 缺陷不在环境，而在于 **`indexmap` 的 build script 被缓存复用、从未重跑** |
| 进程树外（无 `tsbx.dll` 注入）实测 `stdin=pipe` **OK**，但 E0107 **照旧复现** | E0107 与该管道缺陷**无因果** → **缓存污染**成立 |
| `cargo clean -p indexmap` 之后，日志出现 **`Compiling indexmap v1.9.3`**，紧接着 `Compiling schemars v0.8.22` **编译通过** | 修复有效，且**反证**缓存是唯一原因 |

**修复（一次性，两条命令）**

```bash
cd E:/Download/xhub
cargo clean -p indexmap --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml       # 应看到 Compiling indexmap v1.9.3 → 通过
```

> 兜底：若因故仍不通过，再执行 `cargo clean --manifest-path src-tauri/Cargo.toml` 全量重建（**必须带 `--manifest-path`**，否则会在仓库根找不到 `Cargo.toml` 而空跑）。registry 已缓存，不会重新下载。

**已实测结果（2026-09-22 22:34 / 22:40，`RUSTFLAGS` 全程未设置）**

| 命令 | 结果 |
| --- | --- |
| `cargo check --manifest-path src-tauri/Cargo.toml` | ✅ **`EXIT 0`**，19.96s，`Finished \`dev\` profile [unoptimized + debuginfo] target(s)` |
| `cargo build --manifest-path src-tauri/Cargo.toml` | ✅ **`EXIT 0`**，3m12s → `src-tauri/target/debug/app.exe`（**27,945,984 B**） |

**⚠️ 跨项目通用教训（务必记住）**

**不要在 WorkBuddy 沙箱内跑 `cargo` / `tauri` 构建。** 沙箱 shim（`tsbx.dll`）会破坏子进程 stdin 管道 → build script 探针**静默失败**（只打 warning、不报错）→ **错误结果被写进 `target/` 指纹缓存** → 之后即使在干净终端重建，cargo 也会**复用这份坏缓存**，形成「换了环境还是同样的错」的假象。

> **排查心法**：凡遇到「干净环境仍复现**同一个 build script 失败**」，**先 `cargo clean`**（或定向 `cargo clean -p <crate>`）**再下判断**，不要一上来就怀疑安全软件、防火墙或硬件。

### 7.10 子进程 stdin 管道创建失败（`os error 231`）—— **只在 WorkBuddy 进程树内出现**

> **一句话结论**：这是 **WorkBuddy 自带沙箱 shim（`tsbx.dll` 注入）** 造成的，**不是你的机器、不是火绒、不是 Rust/MSVC**。影响面**仅限**「在 WorkBuddy 内跑 cargo / tauri」。

**证据（进程内 / 进程外对照，均为自建探针实测）**

| 测试项 | WorkBuddy 内 | 进程树外（普通 CMD） |
| --- | --- | --- |
| `autocfg` 式探针（`Command::new(rustc).stdin(Stdio::piped())`） | ❌ `os error 231`（**20/20 稳定**） | ✅ **OK** |
| Win32 `CreatePipe` 连续 300 次 | `fail=0`（管道资源**未耗尽**） | — |
| Win32 `CreateNamedPipeW`（固定名 ×10） | `fail=0` | — |
| Node 子进程 6 种 stdio 组合（含 `stdin:'pipe'`） | **全部 OK** | — |
| `stdout` / `stderr` 设为管道 | **完全正常** | — |
| `GetModuleHandleW("tsbx.dll")` 自检 | **`true`（已注入）** | **`false`** |
| `explorer.exe` / `AppInit_DLLs` | **无注入** / `LoadAppInit_DLLs=0` → **非全机注入** | — |

**结论**：起因是 **WorkBuddy 派生的进程被注入 `tsbx.dll`**（路径 `…\WorkBuddy\resources\app.asar.unpacked\cli\vendor\sandbox\5.6.10\tsbx.dll`）。一旦离开该进程树，缺陷**完全消失**。由于全依赖树里**只有 `autocfg` 用 `.stdin(Stdio::piped())`**（`cc` 编译 SQLite 走 `.output()` / `stdin=null`，`embed-resource` 走继承句柄），症状才**只**表现为 `indexmap 1.9.3` → `schemars 0.8.22`。

**对日常开发的硬性约束**

| 操作 | 建议执行位置 |
| --- | --- |
| `cargo check` / `cargo build` / `cargo test` / `cargo clean` | **普通 CMD / 终端**（或 VS 开发者命令提示符） |
| `npm run tauri:dev` / `tauri:build` / `tauri:test` | **普通 CMD / 终端** |
| `npm ci` / `npm run build` / `npm run dev`（纯前端，不碰 Rust 构建脚本） | WorkBuddy 内**即可** |
| 读代码、改前端、查日志、跑 git | WorkBuddy 内**即可** |

### 7.11 `tauri-winres` 报 `Are you sure you have RC.EXE in your $PATH or ${RC_$TARGET} or $RC is set?`

**现象**：`cargo check` / `cargo build` 编译到 `app` 构建脚本时 panic：

```text
thread 'main' panicked at tauri-winres-0.3.6/src/lib.rs:543:14:
Failed("Are you sure you have RC.EXE in your $PATH or ${RC_$TARGET} or $RC is set?")
```

**真因（已确证）：环境变量 `RC` 被赋成了非法值。**

`embed-resource 3.0.11` 的查找链（读源码核实：`src/lib.rs:641-645` + `src/windows_msvc.rs:33-43`）：

```text
RC_<target>               →  RC_x86_64-pc-windows-msvc
RC_<target 的 - 换成 _>   →  RC_x86_64_pc_windows_msvc
RC                        →  ← 本次命中的就是这一级
PATH 上的 rc.exe
以上全落空 → 才走「注册表 KitsRoot10 + 版本目录扫描」→ 仍无 → 才抛这条 panic
```

只要 `RC` **存在**（哪怕值是 `1`），就会**短路**掉后面的 PATH 与注册表发现逻辑，直接 `Command::new("1")` → spawn 失败 → 抛出上面这条 panic。

**本次踩坑的具体经过**：脚本里用 `set RC=1` / `set RC=%ERRORLEVEL%` 当「退出码暂存变量」——**变量名与 embed-resource 读取的环境变量同名**。于是出现「SDK 明明装好了、`rc.exe` 也明明白白躺在磁盘上，却报 `rc.exe` 找不到」的诡异现象。

**诊断要点**

- 报错里的 `Failed(...)` 来自 `tauri-winres/src/lib.rs:543` 的 `.unwrap()`，对应 embed-resource 的 **`Command::status()` 返回 `Err`**——即「**路径根本不存在 / 无法启动**」，**不是**「`rc.exe` 跑起来后编译失败」（后者文案是 `RC.EXE failed to compile specified resource file`）。
  → **看到这条就优先怀疑 `RC*` 变量，而不是怀疑没装 SDK。**
- 自查命令：`echo %RC%` / `echo %RC_x86_64_pc_windows_msvc%`。本机实测这两者**均未持久化设置**（用户级 / 系统级环境变量、`~/.cargo/config.toml`、项目内 `.cargo/config.toml` **全部为空**）。

**正确做法**

1. **不要**在任何会调用 cargo 的脚本里把变量命名为 `RC`（同理避开 `RC_*` / `TARGET` / `HOST` / `OUT_DIR` / `INCLUDE` / `PATH`）。退出码请用 `CHECKRC` 之类的名字。
2. **什么都不额外配置也能跑**：`rc.exe` 不在 PATH 上**不是问题**——embed-resource 会从注册表 `KitsRoot10` 定位 SDK 根，再扫描 `bin\<version>\x64\rc.exe`。本机实测该发现逻辑**能正确命中** `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe`，且该路径可成功 spawn。
3. **若想让查找彻底确定化（可选，非必需）**，任选其一：
   - **（推荐）** 用开始菜单的 **「x64 Native Tools Command Prompt for VS 2022」** 跑 cargo / tauri；它会自动把 `rc.exe`、`mt.exe`、`cl.exe` 都准备好（`npm run tauri:test` 需要 `mt.exe`）。
   - **（最小）** `setx RC_x86_64_pc_windows_msvc "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe"`，然后**新开终端**生效。
     **回滚**：`setx RC_x86_64_pc_windows_msvc ""`，或到「系统属性 → 环境变量」里删除该项。

**本机现状（已逐项核实）**

| 项 | 状态 |
| --- | --- |
| `rc.exe` | ✅ 存在：`C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\rc.exe`（**仅此一个版本目录里有**；`10.0.14393.0` / `15063.0` / `16299.0` / `17134.0` 的 `x64\rc.exe` **均缺失**） |
| `bin\x64\rc.exe`（旧式布局兜底） | ❌ **不存在** → 所以 `find_windows_kits_tool` 那一级兜底实际无效，真正生效的是「注册表 + 版本目录扫描」那一级 |
| `mt.exe` | ✅ 同在 `…\10.0.26100.0\x64\`（`npm run tauri:test` 需要） |
| PATH | ❌ **不含** Windows Kits 的 bin 目录 |
| `RC` / `RC_x86_64_pc_windows_msvc` | ❌ **均未持久化设置** |

**实测结论**：`cargo check`（22:34）与 `cargo build`（22:40）**都真实调用了 `rc.exe`**（证据：`src-tauri/target/debug/build/app-*/out/resource.lib` 随这两次构建被**重新生成**），**无需任何额外配置**。

### 7.12 已作废的处置路径（保留备查，**日常无需使用**）

早期版本为「绕过本机缺陷」设计了四条路径。**现根因已查清（见 7.9 / 7.10 / 7.11），四条路径全部不再需要**，此处仅留索引，避免旧笔记误导：

| 旧路径 | 原用途 | 现状 |
| --- | --- | --- |
| A 修环境（放行火绒 HIPS） | 消除 stdin 管道拦截 | ❌ **作废** —— 火绒无辜，**请不要**去改火绒设置 |
| B 窄修：`[build-dependencies]` 加 `indexmap = { version = "1", features = ["std"] }` | 跳过 `autocfg` 探针 | ❌ **不需要**（且属源码改动，本仓库 `Cargo.toml` 已保持**零改动**） |
| C 用 `--config '{"build":{"beforeDevCommand":""}}'` 清空 `beforeDevCommand` | 绕过 tauri-cli 派生 `npm run dev` | ❌ **不需要**（该 panic 同属 7.10 的注入层现象，出沙箱即消失） |
| D 全局 `RUSTFLAGS='--cfg has_std'` | 应急 hack | ❌ **不需要**；它会改变缓存指纹、触发全量重编，**请勿使用** |

> 若确需查这些路径的技术细节（`indexmap` 探针机制、`CARGO_FEATURE_STD` 环境变量版、`autocfg` 无开关等调研结论），见**附录 A 四～六轮**；那些结论本身仍然成立，只是**已无使用必要**。

### 7.13 `npm run tauri:test` 报测试 exe `STATUS_ENTRYPOINT_NOT_FOUND`（0xc0000139）—— cargo 重链冲掉了 manifest（**已修复**）

**现象**：脚本打印 `已为 N 个测试 exe 嵌入 Common-Controls v6 manifest` 之后立刻失败：

```
Running unittests src\lib.rs (…\target\debug\deps\app_lib-….exe)
error: test failed, to rerun pass `--lib`
Caused by:
  process didn't exit successfully: `…app_lib-….exe` (exit code: 0xc0000139, STATUS_ENTRYPOINT_NOT_FOUND)
```

**根因（两步，均与源码无关）**：

1. tauri/wry 导入 `comctl32!TaskDialogIndirect`，它只存在于 Common-Controls **v6** 程序集，而 cargo 的**测试 exe 不经 tauri-build 的 manifest 嵌入**（后者只作用于 bin 目标）→ 缺激活上下文的进程在**加载期**就死（不是断言失败、没有任何测试输出）。
2. 脚本原本的处置是「`--no-run` 构建 → `mt.exe` 逐 exe 嵌 manifest → 再交给 `cargo test` 运行」，但**第 3 步的 cargo 检测到 exe 被 mt 改过（mtime 变化）→ 判定产物过期 → 重新链接 → 嵌好的 `RT_MANIFEST` 被覆盖回默认**，于是又落回第 1 条。旧注释里「指纹未变不会重链」的假设是错的——**这是本机此前根本跑不了这个脚本的真正原因**（2026-09-22 实测复现）。

**修复（`scripts/cargo-test.ps1` 第 3 步）**：不再把控制权交回 cargo，改为**直接执行**第 2 步嵌过 manifest 的测试 exe（等价于 `cargo test` 的运行阶段，只是绕开 cargo 的产物检查）。第 1 步已消费 cargo 选择器参数（`--lib` / `--bins` / `--test …`），只有 `--` 之后的 harness 参数（`--nocapture` / `--test-threads …`）才转发给 exe。

**顺带修掉的用例间竞态**：脚本现在**默认 `--test-threads=1`**。`commands::tests` 有两个平台额度用例共享**进程内全局轮询游标**（`chat::PLATFORM_RR`），并行跑会互相推游标，使 `legacy_session_with_concrete_platform_name_still_load_balances` 的 `assert_ne!` 偶发失败（实测：多线程 `238 passed; 1 failed`，单线程 `239 passed; 0 failed`）。想并行自己传：`npm run tauri:test -- -- --test-threads=8`。

**手工等效命令**（不想用脚本时，注意**别在最后一步用 cargo**）：

```powershell
# 1) 构建测试产物并取 exe 路径
cargo test --lib --no-run --message-format=json
# 2) 给 exe 嵌激活上下文（幂等，可重复执行）
& "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\mt.exe" -nologo `
    -manifest src-tauri\windows\app.manifest -outputresource:"<上一步的 exe 路径>;#1"
# 3) 直接运行该 exe（不是 cargo test）
& "<上一步的 exe 路径>" --test-threads=1
```

---

## 8. 构建产物路径

| 产物 | 路径 |
| --- | --- |
| 前端构建产物 | `dist/`（`index.html` + `assets/`） |
| Rust 编译中间产物 | `src-tauri/target/` |
| **开发可执行文件（debug）** | `src-tauri/target/debug/app.exe`（**本轮实测**：`cargo build` 直接产出；Cargo 包名为 `app`，≈ 27 MB） |
| `tauri build` 产出的可执行文件 | `src-tauri/target/release/x-hub.exe`（由 tauri-cli 按 `mainBinaryName = "x-hub"` 重命名） |
| **桌面安装包** | `src-tauri/target/release/bundle/`（**当前配置下不会生成**——`bundle.active = false`，且发版流程显式用 `--no-bundle`；详见下方注） |

> 注：当前 `tauri.conf.json` 里 `bundle.active = false` / `targets: []`，本地 `tauri:build` 默认**不产安装包**，只出可执行文件 `src-tauri/target/release/x-hub.exe`（`mainBinaryName = "x-hub"`，故产物名是 `x-hub.exe`）。
>
> ⚠️ **产物名注意**：`mainBinaryName = "x-hub"` 仅经 **tauri-cli**（`tauri build` / `tauri dev`）路径生效——CLI 会把 Cargo 产物 `app` 重命名 / 指向为 `x-hub`。**直接 `cargo build` 的产物名是 `app.exe`**（Cargo 包名），本轮实测即产出该文件。
>
> ⚠️ **更正（读 `.github/workflows/release.yml` 核实）**：正式发版**并不**「覆盖 bundle 配置」——release.yml 的命令是 `npm run tauri:build -- --no-bundle`，随后把 `src-tauri/target/release/x-hub.exe` 直接压成标准版 / 便携版 zip 上传 GitHub Release。**该流程从不生成 `bundle/` 安装包（NSIS/MSI）**。因此 `src-tauri/target/release/bundle/` 在本仓库现有配置下**实际不会被生成**（README「安装与运行」小节里「产物在 …/bundle/」属过时口径）。
>
> ⚠️ **`target/` 体积与缓存可复用性（运维提示）**：`src-tauri/target/` 目前约 **6.1 GB**。**关键点：`RUSTFLAGS` 或特性集一变，cargo 即视为不同指纹，会触发全量重建、旧缓存不可复用**。本仓库**现已不使用任何 `RUSTFLAGS`**，缓存可正常复用；若你曾用过 `RUSTFLAGS='--cfg has_std'` 的旧绕过（**已作废，见 7.12**），那些缓存会因指纹不同而整包重编一次，属一次性代价。
> **清理建议**：`cargo clean --manifest-path src-tauri/Cargo.toml`（**代价**：删掉约 6 GB，且**再次构建需重新下载/编译全部依赖**，本机首次约 2–5 分钟）——请权衡后再执行；另注意本机批量删除可能被安全软件 / 宿主拦截，`cargo clean` 偶发被终止（可重试或手动清理）。**何时该 clean**：① 切换 / 去掉了 `RUSTFLAGS`（缓存已失效）；② 磁盘紧张；③ 排查「诡异增量编译」问题；④ **遇到「干净环境仍复现同一个 build script 失败」—— 这是 `target/` 缓存被污染的最典型症状，先 `cargo clean -p <crate>` 再下判断（见 7.9）**。**何时别 clean**：`RUSTFLAGS` 未变、且马上要再次 `cargo build`（今次清理只会白白重编）。

---

## 9. 二次开发上手提示

### 9.1 关键目录导览

| 路径 | 内容 |
| --- | --- |
| `src/main.ts` · `src/App.vue` | 前端入口与窗口壳（按窗口 `label` 路由：主窗/便签浮窗/倒计时浮窗/剪贴板浮层/扩展浮窗/AI 窗/悬浮球/通知） |
| `src/index/index.vue` | 首页：侧栏导航 + 视图协调 + 三轴主题 |
| `src/api/tauri.ts` | 所有 Tauri `invoke` 调用的类型安全封装（190+ 命令）+ 模型/配置类型 |
| `src/stores/workbench.ts` | 响应式状态（`reactive()` + `readonly()` 自定义 store，**无 Pinia**） |
| `src/components/` | 功能组件（工作台卡片/速记/速达/搜索/待办/设置/倒计时/AI/扩展中心/更新弹窗…） |
| `src/composables/` · `src/utils/` | 组合式函数 / 工具（文件分类、时间、提示音、天气码映射、本地名言语料…） |
| `src-tauri/src/lib.rs` | 应用构建：数据库/托盘/快捷键/窗口状态/数据迁移/命令注册 + 定时检查更新 |
| `src-tauri/src/commands.rs` | Tauri 命令实现 |
| `src-tauri/src/db.rs` · `models.rs` | SQLite 迁移与模型 |
| `src-tauri/src/config.rs` | 配置持久化（主题/窗口/快捷键/AI 模型/更新源…）+ `DEFAULT_SERVER_URL` 等服务端地址常量 |
| `src-tauri/src/repo/` | 数据访问层（resource/note/todo/todo_tag/sticky/detached_sticky/snippet/tag/countdown/chat/clipboard） |
| `src-tauri/src/online.rs` | 天气 / 名言 / IP 定位 / 连通性探活（全部走后端 reqwest） |
| `src-tauri/src/chat.rs` · `market.rs` · `updater.rs` | AI SSE 对话 / 扩展市场 / 应用自动更新 |

### 9.2 必须同步更新的文档约定

改代码时按项目约定同步以下文档（详见 `AGENTS.md`）：

- **`CONTEXT.md`**：领域术语表——新增/改动易混淆概念时按「术语 + 说明 + `_Avoid_`」格式登记，口径一致。
- **`docs/adr/`**：重大架构决策写 ADR（现有 `0001`–`0010`，如 `0006-agent-runtime-selection.md`）。
- **`AGENTS.md`**：文件头/结构/关键约定/命令速查，跟随代码更新。
- **`RELEASE_NOTES.md`**：发版时在**顶部**新增 `# vX.Y.Z 发布说明` 章节（累积式，勿覆盖历史）。
- **版本号单一来源**：发版从 README 向下同步 `README.md` 徽章 → `package.json` → `src-tauri/tauri.conf.json` → `src-tauri/Cargo.toml` → `AGENTS.md` 头部。

### 9.3 分支策略

- 远端**只有 `master` 一个分支**（`origin/HEAD -> origin/master`），`master` 即默认开发分支。
- 本地当前在 `master`，与 `origin/master` 同步，工作区干净（基线 `d957223`）。
- 二次开发建议：**在本地新建特性分支**（如 `feature/xxx`）再合并回 `master`，或按团队约定走 PR 流程；推送前先完成第 4.6 节的凭据配置。

---

## 附录 A：本机实测证据（2026-09-22；含桌面链路二轮 + 四轮 + 五轮 + 六轮复核）

> ⚠️ **本附录是按时间顺序的原始实测记录。其中关于「本机环境缺陷 / 最可疑火绒（Huorong）HIPS」的归因，已在终局被推翻**（见 §7.9 判定更正、§7.12 已作废路径）。原文保留仅供追溯，**不作为结论** —— **请勿据此去修改火绒、防火墙或系统设置**。

> 说明：**未标注**的行是同日前段的前端链路实测；标 **「二轮复核」** 的为补齐 Rust/MSVC 桌面链路的证据；标 **「四轮」** 的为处置方案（窄修 A/B、依赖链与版本比对、`tauri dev --config` 绕过）新增证据；标 **「五轮」** 的为 **QA 在真实工程上**对路径 B 的独立复核（修正了「`[dependencies]` 做法」）；标 **「六轮」** 的为**工程师本轮**补做的「**无 `RUSTFLAGS`** 的路径 B（`[build-dependencies]`）+ 路径 C（`--config`）**组合桌面回归**」及 `autocfg` / `CARGO_FEATURE_STD` 调研证据。同一项以后者为准。

| 检查项 | 命令 | 实测结果 |
| --- | --- | --- |
| 前端依赖安装 | `npm ci` | ✅ `added 303 packages in 2m`，`EXIT_CODE=0` |
| 前端类型检查 + 构建 | `npm run build` | ✅ `EXIT_CODE=0`，`DURATION_SEC=15`，`✓ built in 2.34s`，产出 `dist/` |
| Vite dev server | `node node_modules/vite/bin/vite.js` + `curl` | ✅ HTTP `200`，`main.ts` `200`，`ready in 2171 ms`；测后已杀，端口 1420 无 `LISTENING` 残留 |
| Node / npm | `node -v` / `npm -v` | ✅ `v22.22.2` / `10.9.7` |
| Vite Node 要求 | `vite/package.json.engines` | `^20.19.0 \|\| >=22.12.0`（已满足） |
| Rust 工具链（一轮） | `rustc -V` / `cargo -V` / `rustup` | ❌ 均 not found；`~/.cargo`、`~/.rustup` 不存在 |
| MSVC / Windows SDK（一轮） | 目录探测 | ❌ `Microsoft Visual Studio`、`Windows Kits` 均不存在 |
| WebView2 | 目录探测 | ✅ `153.0.4234.48` |
| PowerShell 7 | `pwsh -v` | ✅ **7.6.6**（`%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe`，Store 版；原记录「未安装」有误，经 QA 复核修正） |
| git 远端 | `git remote -v` | ✅ `origin https://github.com/AvalonLee/x-hub.git` |
| HTTPS 读权限 | `git ls-remote origin HEAD` | ✅ `d957223...`，`EXIT=0` |
| gh 登录 | `gh auth status` | ✅ 已登录 `AvalonLee`（owner），HTTPS，scope 含 `repo`/`workflow` |
| SSH 公钥 | `~/.ssh/id_ed25519.pub` | ⚠️ 存在但**未在 GitHub 注册** |
| **Rust 工具链（二轮复核）** | `rustc -V` / `cargo -V` / `rustup show`（Bash 补 PATH 后） | ✅ `rustc 1.98.1 (48a229cea 2026-09-01)` / `cargo 1.98.1 (797e8a9bc 2026-08-05)`；默认 toolchain `stable-x86_64-pc-windows-msvc`，host `x86_64-pc-windows-msvc` |
| **MSVC / Windows SDK（二轮复核）** | 目录探测 | ✅ `VC/Tools/MSVC/14.44.35207`（含 `bin/Hostx64/x64/link.exe`）；Windows SDK `10.0.26100.0`（`bin/` 下含 `mt.exe`，另有旧版本） |
| **`cargo check`（CI 原样命令）** | `cargo check --manifest-path src-tauri/Cargo.toml` | ❌ **`EXIT_CODE=101`**：`error[E0107]` @ `schemars-0.8.22/src/lib.rs:12`，`could not compile \`schemars\` (lib)`；构建脚本 stderr `warning: autocfg could not probe for \`std\``。**QA 复核：`dangerouslyDisableSandbox` 关闭沙箱后重跑仍 `EXIT=101`、同一错误（88s）→ 当时判为「非沙箱特有」**（⚠️ **该归因已作废**：真因是 `target/` 缓存污染 + WorkBuddy 注入层，见 §7.9 / §7.10） |
| **`cargo check`（绕过）** | `RUSTFLAGS='--cfg has_std' cargo check …` | ✅ `EXIT_CODE=0`，`DURATION_SEC=142`（`Finished dev profile … in 2m 20s`），**警告数 0** |
| **工具链自证 + 根因复现** | `rustc --edition 2021 spawntest.rs -o spawntest.exe` 后运行 | ✅ 编译链接出 `268,800 B` 的真实 `.exe`（证明 rustc+MSVC 链接正常）；程序内 `Command::new("rustc").arg("-V").output()`（stdout+stderr 管道）→ **OK**，而 `…stdin(Stdio::piped())…spawn()` → `ERR: 所有的管道范例都在使用中。 (os error 231)`（**复现根因——失败的是子进程 stdin 管道**） |
| **桌面链接** | `RUSTFLAGS='--cfg has_std' cargo build --manifest-path src-tauri/Cargo.toml` | ✅ `EXIT_CODE=0`，`DURATION_SEC=189`（3m07s）→ 产出 `src-tauri/target/debug/app.exe`（**27,945,984 B ≈ 27 MB**） |
| **桌面运行** | `./src-tauri/target/debug/app.exe`（运行 12s 后取证据并杀进程） | ✅ 进程存活；启动日志完整：`x-hub 启动` / `数据库初始化完成` / `扩展反向代理已启动: 127.0.0.1:5752` / `系统托盘初始化完成` / 注册快捷键 `Ctrl+Shift+Space`、`` Ctrl+` `` / `恢复窗口状态: 1400x900`；**无 panic**；已生成 `%APPDATA%\x-hub\app.db` |
| **清理复查** | `tasklist` / `netstat -ano`（杀进程后） | ✅ `app.exe` 无残留；端口 `1420`、`5752` 均无 `LISTENING` |
| **`npm run tauri:dev`** | `npm run tauri:dev`（沙箱内两次 + **关闭沙箱一次**） | ❌ tauri-cli panic：`failed to run \`npm run dev\``（`dev.rs:207:31`，rc=127，7s）——**关闭沙箱后同样复现（⚠️ 该行归因已作废：真因是 WorkBuddy 注入层 + 缓存污染，见 §7.9 / §7.10）** |
| **`cargo` 窄修 A/B（四轮，最小复现）** | 临时 crate（`schemars="=0.8.22"` + `features=["preserve_order"]`）：对照 vs 额外加 `indexmap={version="1.9",features=["std"]}` | **对照：`EXIT=101`**（schemars E0107；`indexmap-*/stderr`=`warning: autocfg could not probe for \`std\``）；**加 `indexmap/std`：`EXIT=0`（22.7s），stderr 为空、输出 `cargo:rustc-cfg=has_std`**（探针被完全跳过）。⚠️ 注：此为**最小复现**（indexmap 在普通依赖上下文），**不能代表本仓库**——见下一行。 |
| **`cargo` 窄修（五轮，真实工程·QA 独立）** | 真实 `src-tauri/Cargo.toml`：分别把 `indexmap={version="1",features=["std"]}` 放 `[dependencies]` / `[build-dependencies]`；`CARGO_TARGET_DIR`=临时目录、`RUSTFLAGS` 置空；测后完整还原 | **`[dependencies]`：`EXIT=101`**（同 schemars E0107；`indexmap-*/stderr` 仍 `autocfg could not probe`；`cargo tree -e features` 有 **2 个** indexmap 单元、**build-dep 那个无 `std`**）→ **原稿做法无效**；**`[build-dependencies]`：`EXIT=0`（1m17s）、stderr 为空、output=`cargo:rustc-cfg=has_std`、只剩 1 个 indexmap 单元（features 含 `std`）** → **修正做法有效** |
| **indexmap 版本比对** | 读 `indexmap` 1.9.1 / 1.9.2 / 1.9.3 的 `build.rs` 并 sha256 比对（1.8.2 / 1.9.0 本地未留存） | **QA 复核：`1.9.1 / 1.9.2 / 1.9.3` 三者 sha256 完全一致（`558b4d0b…`）**，都走 `autocfg` 探针（仅当 `CARGO_FEATURE_STD` 有值才跳过）→ `cargo update --precise` 换 1.9.x **无法规避**；`indexmap 2.14.0` **无 `build.rs`**（不探针），但 `schemars 0.8.22` 的 `[dependencies.indexmap] version = "1.2"`（`^1.2`）**只接受 1.x**。**（`1.8.2 / 1.9.0` 未独立复核，原稿「五者逐字相同」未逐版验证）** |
| **`has_std` 影响面** | 全 `~/.cargo/registry` 检索 `has_std`（`**/src/lib.rs` 与 `**/build.rs`；QA 另做全目录 `grep -rl`） | **仅 `indexmap-1.9.3`** 读 `cfg(has_std)`（QA 另见 `brotli-8.0.4` 命中，但为函数名 `has_stdlib`、非 cfg，已排除）→ 全局 `--cfg has_std` **虽注入所有 crate**，但**唯一读取方**只有 indexmap-1.9.3，故实际副作用≈可忽略。**（原稿「影响面≈定向」措辞易误导：注入是全局的，只是无人读）** |
| **`tauri dev` 绕过（四轮，已验证可行）** | 先 `npm run dev`（1420 就绪）→ `RUSTFLAGS='--cfg has_std' npx tauri dev --config '{"build":{"beforeDevCommand":""}}'` | ✅ 进入 `Running DevCommand (\`cargo run --no-default-features --color always --\`)`；编译 493 单元 → 拉起 `app.exe`（进程存活）→ **6 个 `msedgewebview2.exe`** → 应用日志 `x-hub 启动完成`、`已是最新版本（v0.6.5）`；**成功跳过**了派生 `npm run dev` 的失败步骤 |
| **`tauri dev` 绕过·残留问题** | 同上，读应用日志 | ⚠️ 一个**辅助隐藏窗口** webview 创建失败：`failed to create webview: WebView2 error … HRESULT(0x80070578) "没有注册类"`（发生在预创建隐藏窗阶段，**非致命**，主流程仍 `启动完成`）；主窗口渲染**未目视确认**（无截图） |
| **构建中途警告** | 同上，观察 `target` 写入 | ⚠️ `warning: error copying object file … to incremental directory … 拒绝访问。 (os error 5)`（**非致命**；疑与本机拦截/宿主同源） |
| **绕过后的清理** | `taskkill` + `netstat -ano` | ✅ `app.exe` / `cargo` 无残留；`node.exe` 回到基线（仅 7440 / 25812）；端口 `1420` / `8175` / `5752` 无 `LISTENING`（残留的 `msedgewebview2.exe` 经查为**其它应用**所有、杀掉即重生，**未强杀**） |
| **路径 B+C 组合桌面回归（六轮，无 RUSTFLAGS）** | 真实工程：`[build-dependencies]` 加 `indexmap={version="1",features=["std"]}` → 先 `npm run dev`（1420 就绪、`curl` 200）→ **不设任何 `RUSTFLAGS`** 跑 `npx tauri dev --config '{"build":{"beforeDevCommand":""}}' --no-watch` | ✅ **`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 3m 14s` → `Running \`target\debug\app.exe\``；全程 `RUSTFLAGS=[<unset>]`；无 E0107**；`app.exe` 存活 + **6 个 `msedgewebview2.exe`**；启动日志完整（`========== x-hub 启动 ==========` / `数据库初始化完成` / `系统托盘初始化完成` / 快捷键 `Ctrl+Shift+Space`、`` Ctrl+` `` / `悬浮球窗口已创建` / `剪贴板监听线程启动` / 反代 `127.0.0.1:3395`）——**证明无 RUSTFLAGS 亦可跑起桌面** |
| **indexmap 构建脚本落盘（六轮）** | 读 `src-tauri/target/debug/build/indexmap-d94c8e92c6c11fc0/{output,stderr}`（当前构建的 run 目录，时间戳与本次一致） | `output` = `cargo:rustc-cfg=has_std` + `cargo:rerun-if-changed=build.rs`；`stderr` **为空**（= 探针被跳过、直接声明）；同轮**无任何** indexmap 构建脚本产出 `autocfg could not probe for \`std\``（仍留 `19:26`/`19:30` 的旧告警目录，系前几轮陈旧产物，非本次） |
| **清单还原证明（六轮）** | `md5sum`（还原前后对比） + `git status --short` / `git diff --stat` | ✅ `src-tauri/Cargo.toml` = `d57c77738960e2dc885ef16b3cba855b`、`src-tauri/Cargo.lock` = `3117aa8ead388ac6b36001e7509a0149`（**恢复后与备份逐字节一致**）；`git status --short` = **仅 `?? docs/local-dev-setup.md`**；`git diff --stat` **空** |
| **`autocfg` 有无跳过开关（六轮）** | 通读 `~/.cargo/registry/src/…/autocfg-1.5.1/src/lib.rs` | ❌ **无** `AUTOCFG_*` 之类开关；只读 `OUT_DIR` / `TARGET` / `RUSTC` / `RUSTFLAGS` / `CARGO_ENCODED_RUSTFLAGS`（`rustflags()` L579–618）；探针固定 `command.arg("-").stdin(Stdio::piped())`（L327），**外部无法改为非 stdin 方式**；探针失败仅打 `warning: autocfg could not probe for \`std\``（L217）、不抛错 |
| **`CARGO_FEATURE_STD` 透传（六轮）** | 最小 crate（`build.rs` 打印 `env::var_os("CARGO_FEATURE_STD")`）：不设 env vs `CARGO_FEATURE_STD=std` | 不设 → `CARGO_FEATURE_STD=None`；设置 → `Some("std")` → **cargo 确会把外部该变量透传给 build script**（即 `indexmap#184` 记录的环境变量版「路径 B」；但为**全局作用域 + 不落盘**，**不优于 B**，仅作备选） |
| **组合绕过后的清理（六轮）** | `taskkill` / `netstat -ano` / `tasklist` | ✅ `app.exe`、`cargo` 无残留；端口 `1420` / `5752` / `8175` / `3395` 无 `LISTENING`；`node` 回到基线（7440 / 25812） |

---

## 附录 B：QA 独立复核记录（2026-09-22，未采信工程师自述）

> ⚠️ **本附录是按时间顺序的原始实测记录。其中关于「本机环境缺陷 / 最可疑火绒（Huorong）HIPS」的归因，已在终局被推翻**（见 §7.9 判定更正、§7.12 已作废路径）。原文保留仅供追溯，**不作为结论** —— **请勿据此去修改火绒、防火墙或系统设置**。

> 由 QA 独立重跑 / 读源码取证，用于证伪上文的自述结论。凡未亲自复现的一律标为「未验证」。

| 复核项 | 独立执行的操作 | 实测结果 |
| --- | --- | --- |
| 依赖真实落盘 | 读 `node_modules/.package-lock.json` 的 `packages` 条目数 | **303**（与「added 303 packages」一致）；`node_modules` 顶层 178 项 |
| `npm ci` | 未重跑 | 未重跑（沙箱 bulk-delete 保护会拦截对 `node_modules` 的清理；以「目录真实存在 + 303 条目 + 构建可用」间接佐证） |
| 构建（独立重跑） | `mv dist dist_qa_bak && npm run build` | **EXIT 0，15s**，`✓ built in 2.65s`；产出 `dist/`（`index.html` + `assets/` 221 文件 + `favicon.svg`） |
| 构建注意 | 在 `dist/` 已存在时直接重跑 `npm run build` | vite `prepare-out-dir` 会清空 `dist/`；**本机沙箱的批量删除保护会拦截**（>100 文件），需先把 `dist/` 移走/清空才能跑通 |
| Dev server | `node node_modules/vite/bin/vite.js`，再 `curl --noproxy '*' http://127.0.0.1:1420/` | **HTTP 200**（4110 B，`<title>x-hub 个人效率工作台</title>`）；`/src/main.ts` **200**；日志 `VITE v8.2.0 ready in 1610 ms` |
| 端口清理 | `netstat -ano`（测前 / 运行时 / 杀进程后） | 运行时 `0.0.0.0:1420 LISTENING`；杀进程后**仅剩 `TIME_WAIT`，无 `LISTENING`** |
| PowerShell 7 | `pwsh -v` | ✅ **7.6.6**（原文档记「未安装」，**已更正**） |
| Vite engines | `node_modules/vite/package.json.engines` | `^20.19.0 \|\| >=22.12.0`（属实） |
| Rust / MSVC | `rustc -V` / `cargo -V` / `rustup`；目录探测 | 三者 not found；`~/.cargo`、`~/.rustup` 不存在；`Microsoft Visual Studio`、`Windows Kits` 目录不存在（属实） |
| WebView2 | 目录探测 | `153.0.4234.48`（属实） |
| 数据 / 日志路径 | 读 `paths.rs` / `config.rs` / `lib.rs` / `db.rs` / `updater.rs` | 与 §2 / §7.6 一致；另补记明文回退文件 `chat_keys.json` |
| 发布流程 | 读 `.github/workflows/release.yml` | 用 `tauri build --no-bundle`，**不产 `bundle/`**（§8 已更正） |
| CI 门禁 | 读 `.github/workflows/ci.yml` | Node 20 → `npm ci` → `npm run build` → `cargo check --manifest-path`，与 §3.5 一致 |
| 推送读权限 | `git ls-remote origin HEAD` | EXIT 0（`d957223…`）——**仅证明读；写（push）未实测** |

**未验证项（诚实标注）**：

- `git push` **写权限**：未执行真实 push（`ls-remote` 只证读成功；`gh` token scope 含 `repo`/`workflow` 仅作旁证），**属未验证**。
- `npm run tauri:dev` 桌面窗口 / `cargo check`：缺 Rust + MSVC，未执行。
- `npm run tauri:test`：需 Rust + MSVC + Windows SDK `mt.exe`，未执行（`pwsh` 已具备）。

> ⚠️ **后续更新（同日「二轮」补齐，详见附录 A 二轮复核行与第 6.4 / 6.5 / 7.9 节）**：上表 `Rust / MSVC` 行（「not found」）与本列表后两条（`cargo check` / `tauri:dev` 因缺工具链未执行）**均已过时**——Rust `1.98.1` 与 MSVC `14.44.35207` 已于同日安装并实测：`cargo check` `EXIT 0`、`cargo build` 链接出 `app.exe`、桌面二进制可运行；`npm run tauri:dev` 则受**子进程 stdin 管道缺陷**影响、无法在 WorkBuddy 内启动（⚠️ 归因已作废：真实原因是 WorkBuddy 注入层，见 §7.10）。**`npm run tauri:test` 仍为未验证**。以上原始记录保留作历史对照。

---

## 附录 C：QA 三轮决定性复核（2026-09-22 · 沙箱外验证 · 本轮新增）

> ⚠️ **本附录是按时间顺序的原始实测记录。其中关于「本机环境缺陷 / 最可疑火绒（Huorong）HIPS」的归因，已在终局被推翻**（见 §7.9 判定更正、§7.12 已作废路径）。原文保留仅供追溯，**不作为结论** —— **请勿据此去修改火绒、防火墙或系统设置**。

> 目标：证伪/证实「`cargo check` 失败是**沙箱特有**、换普通终端即可通过」这一关键判定。全部在**关闭沙箱**（`dangerouslyDisableSandbox`；已用 `cmd //c echo CMD_OK` 自证沙箱黑名单确已解除）或**独立于 Bash 的执行路径**下重跑。

| 复核项 | 独立操作 | 实测结果 |
| --- | --- | --- |
| **CI 门禁（★决定性）** | 关闭沙箱后重跑 `cargo check --manifest-path src-tauri/Cargo.toml`（**不加 RUSTFLAGS**） | ❌ **`EXIT=101`（88s）**，与沙箱内**完全同错**：`error[E0107]` @ `schemars-0.8.22/src/lib.rs:12`；`target/debug/build/indexmap-*/stderr` = `warning: autocfg could not probe for \`std\``。→ **工程师「沙箱特有、换普通终端可通过」的结论被证伪** |
| **`npm run tauri:dev`** | 关闭沙箱后重跑（`timeout 110`） | ❌ 同样 panic `failed to run \`npm run dev\``（`dev.rs:207:31`，rc=127，7s）→ 亦**非沙箱特有** |
| **最小复现（QA 自写程序）** | `Command::new(child).stdin(Stdio::piped()).spawn()`，child ∈ {rustc, cmd, node} | 三种子进程**全部** `os error 231`；沙箱内 **5/5 一致** |
| **A/B 对照** | 同程序分别在 ①沙箱内 ②关闭沙箱 ③PowerShell 路径 运行 | **三者结果一致**（均 `231`）→ 与沙箱无关 |
| **界定失败范围** | 同程序改设 `stdout(Stdio::piped())` / `stderr(Stdio::piped())` / `.output()` | **全部正常**（40/40）→ **仅子进程 `stdin` 管道失败** |
| **「为何只 autocfg 失败」** | 读各 build script 源码 | 仅 `autocfg` 用 `.stdin(Stdio::piped())`；`cc` 用 `.output()` / `.stdin(Stdio::null()).status()` → 只命中 autocfg |
| **可疑根因探测** | `tasklist` | 发现 **`HipsDaemon.exe`(PID 27068) + `HipsTray.exe`(PID 2900)** = **火绒（Huorong）HIPS** 正在运行（系统级进程创建钩子） |
| **磁盘产物核对** | `stat` / 目录探测 | `app.exe` = **27,945,984 B**（与自述一致）；MSVC `14.44.35207`、SDK `10.0.26100.0`（含 x64 `mt.exe`）、WebView2 `153.0.4234.48`、`target/` = **6.1 GB** 均属实 |
| **污染复查** | `git status --short` / `netstat` / `tasklist` | 工作区仅 `?? docs/local-dev-setup.md`；端口 `1420`/`5752` 无残留；无 `app.exe`/`cargo`/`vite` 残留进程 |

**结论**：`cargo check` 失败与 `tauri dev` 失败**同源**——本机**为子进程创建 stdin 管道稳定失败**（`os error 231` / `ERROR_PIPE_BUSY`）。该缺陷在**关闭沙箱**以及**独立于 Bash 的 PowerShell 路径**下**同样成立**，因此：

- ✅ 「**与 Rust / MSVC 安装无关**」——成立（`cargo build --cfg has_std` 能链出并运行 27 MB 的 `app.exe`）；
- ❌ 「**沙箱特有 / 换普通终端即可通过**」——**不成立（已证伪）**；但**当时把矛头指向火绒同样是错的**（见 §7.9 判定更正）。真正的边界是 **WorkBuddy 注入层（`tsbx.dll`）**，离开该进程树即正常；而 E0107 另有 **`target/` 缓存污染**这一独立成因。**无需任何火绒放行，也不需要 `RUSTFLAGS`**。

**后续已定论（2026-09-22 深夜）**：用户在自己的普通终端实测 —— 进程树外 `tsbx.dll=false`、`stdin=pipe=OK`，而 E0107 仍在。据此把问题一分为二：**① 子进程 stdin 管道缺陷 = WorkBuddy 注入层**（**非**本机缺陷）；**② E0107 = `target/` 缓存污染**（`cargo clean -p indexmap` 后修复）。**已实测 `cargo check` `EXIT 0`、`cargo build` `EXIT 0` 并链接出 `app.exe`**。详见 §7.9 / §7.10。


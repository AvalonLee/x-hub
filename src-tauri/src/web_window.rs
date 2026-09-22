//! 外部网页窗口（label `web-1`）：把外部站点以**原生 WebView2 窗口**的形式开在 x-hub 内部。
//!
//! ## 为什么必须是原生窗口，而不是扩展 iframe
//!
//! 扩展的四种形态（view / module / window / drawer）**全部由 iframe 承载**
//! （见 `src/composables/useExtensionFrame.ts`），而主流 AI 网页版一律拒绝被嵌入：
//! 实测 `chat.deepseek.com` 回 `Content-Security-Policy: frame-ancestors 'none'`；
//! 通义 / Qwen / 秘塔回 `frame-ancestors` 白名单或 `X-Frame-Options: SAMEORIGIN`；
//! Gemini 回 `X-Frame-Options: DENY`；ChatGPT / Grok / Perplexity 回 `SAMEORIGIN`。
//! 嵌进 iframe 必然白屏。**顶层导航不受 `frame-ancestors` 约束**，且 Cookie 是第一方、
//! 登录态可持久、SSE 流式与 WebSocket 均正常 —— 所以外部站点一律走本窗口。
//!
//! ## 生命周期（约定 41 铁律，零例外）
//!
//! - 启动期 `init` **无条件**预创建 + 隐藏常驻，初值 `about:blank`；
//! - 运行期 `open` 只做 `navigate` + `show` —— 导航不是建窗，不触发「现场 build 挂死」；
//! - 关闭只 `hide`，**绝不 destroy**（destroy 与现场 build 同罪，见约定 41 的三次事故实录）；
//! - 换站 = 对同一个窗口再次 `navigate`（首批单窗口，够用；多站并存时按 `SLOT` 扩成窗口池，
//!   每个槽位仍在启动期一次性建好）。
//!
//! ## 预创建初值为什么是 `about:blank` 而不是目标站点
//!
//! ① 隐藏的 WebView2 照样会真发网络请求 ——「启动即请求外部站」等于绕过设置里的「联网」
//! 总开关，也不该在用户没点开之前替他联网；② 首次加载推迟到真正要用时，启动零开销。
//!
//! ## 已知限制（有意接受）
//!
//! 外部页面里 `target="_blank"` / `window.open` 打开的新窗口会被 wry 直接拒绝
//! （宿主未注册新窗口处理器，同约定 6b 的扩展外链坑）—— 表现为点了没反应。要修得在
//! WebView2 层接管 `NewWindowRequested`，本版不做；站内导航（SPA 路由）不受影响。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow};

use crate::config;

/// 窗口 label。新增/改名必须同步四处（约定 31）：
/// ① `capabilities/default.json` 的 `windows` 数组；② 本常量；
/// ③ `lib.rs` 的 `web_window::init` 调用；④ `App.vue` 路由 —— 本窗口**不需要** ④：
/// 它加载的是外部站点（顶层导航），从头到尾不加载宿主 `index.html`，因此既不经过
/// `App.vue` 的 label 路由，也不会闪出启动欢迎页（`#boot-splash` 白名单脚本只作用于
/// 加载 `index.html` 的窗口）。
pub const LABEL: &str = "web-1";

/// 窗口池槽位数。首批只接一个站点（DeepSeek），单窗口即可；扩成多站并存时把这里调大，
/// 每个槽的窗口 label 为 `web-<n>`，且在 `init` 里一次性全部预创建。
const SLOT: usize = 1;

/// 最小/最大尺寸（逻辑 px）。上限用于吸收异常值（被拖到超宽双屏），与 chat_window 同口径。
const MIN_WIDTH: f64 = 420.0;
const MIN_HEIGHT: f64 = 320.0;
const MAX_WIDTH: f64 = 3840.0;
const MAX_HEIGHT: f64 = 2160.0;

/// URL 长度上限（防御性：正常站点地址远小于此，超长输入直接拒）
const MAX_URL_LEN: usize = 2048;

/// 几何持久化节流（拖拽过程中 Moved/Resized 高频触发，只在事件流静止后落盘一次）
const GEOMETRY_DEBOUNCE_MS: u64 = 600;
static GEOMETRY_TICK: AtomicU64 = AtomicU64::new(0);
/// 去抖线程单飞标志（口径同 chat_window.rs）
static PERSIST_IN_FLIGHT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// 进程内记住「已请求加载的地址」。
///
/// 为什么不直接读 `win.url()`：`navigate` 是异步的，导航完成前 `win.url()` 仍是旧值。
/// 用户连点两次入口时若按窗口 URL 判断，就会重复导航 → 页面重载 →
/// **正在进行的 DeepSeek 对话直接丢失**。所以以「我们请求过什么」为准。
///
/// 语义：`hide` 不清空 —— 关闭只是隐藏，页面与对话都还在，重开就是 `show` 恢复现场。
/// 要强制重新加载（例如页面卡住）走 `open(..., reload: true)`。
fn requested_url() -> &'static Mutex<String> {
    static CELL: OnceLock<Mutex<String>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(String::new()))
}

#[derive(serde::Serialize)]
pub struct WebWindowState {
    /// 窗口当前是否可见（已打开）
    pub open: bool,
    /// 已请求加载的地址（空串 = 还没打开过任何站点）
    pub url: String,
}

/// 校验外部地址：**只放行 https 公网站点**。
///
/// 三条拒绝理由：
/// - **只放行 https**：`http://` 明文页面可被中间人改写；`file://` / `javascript:` /
///   `data:` 更直接是本机文件读取通道，一律拒；
/// - **拒绝 IP 字面量与非公网主机名**：宿主的反向代理（`proxy.rs`）与 service 后端都监听
///   在回环地址上，不能允许扩展把这类地址开成一个可交互窗口；
/// - 长度上限：防御性。
pub fn validate_url(raw: &str) -> Result<tauri::Url, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("INVALID_ARGUMENT: url 不能为空".to_string());
    }
    if raw.len() > MAX_URL_LEN {
        return Err(format!("INVALID_ARGUMENT: url 超过 {MAX_URL_LEN} 字符"));
    }
    let url = tauri::Url::parse(raw).map_err(|e| format!("INVALID_ARGUMENT: url 解析失败（{e}）"))?;
    if url.scheme() != "https" {
        return Err("INVALID_ARGUMENT: 只允许 https 地址".to_string());
    }
    let host = url.host_str().unwrap_or("").to_ascii_lowercase();
    if host.is_empty() {
        return Err("INVALID_ARGUMENT: url 缺少主机名".to_string());
    }
    // ⚠️ IPv6 的 `host_str()` **带方括号**（`[::1]`），直接 `parse::<IpAddr>()` 必失败 ——
    // 于是 `https://[::1]/` 会一路放行到窗口里（单测抓到的真实缺陷）。先剥括号再判。
    let bare = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host.as_str());
    // 含冒号 = IPv6 形态（含畸形未闭合括号的），一并拒
    if bare.contains(':') || bare.parse::<std::net::IpAddr>().is_ok() {
        return Err("INVALID_ARGUMENT: 不允许直接打开 IP 地址".to_string());
    }
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
    {
        return Err("INVALID_ARGUMENT: 不允许打开本机/内网地址".to_string());
    }
    Ok(url)
}

/// 首次建窗（尚无记忆位置）的落点：主窗中央略偏右下（与对话独立窗/便签浮窗同口径）。
fn initial_center(app: &AppHandle, width: f64, height: f64) -> Option<(i32, i32)> {
    let main = app.get_webview_window("main")?;
    if !main.is_visible().unwrap_or(false) {
        return None;
    }
    let pos = main.outer_position().ok()?;
    let size = main.outer_size().ok()?;
    let scale = main.scale_factor().unwrap_or(1.0);
    let (sw, sh) = (size.width as i32, size.height as i32);
    let w = (width * scale) as i32;
    let h = (height * scale) as i32;
    Some((pos.x + (sw - w) / 2 + 32, pos.y + (sh - h) / 2 + 20))
}

/// 0..SLOT 的窗口 label（首个槽位即 `LABEL`，其余 `web-2` / `web-3` …）
fn label_of(slot: usize) -> String {
    if slot == 0 {
        LABEL.to_string()
    } else {
        format!("web-{}", slot + 1)
    }
}

/// 几何落盘（尺寸取逻辑 px、位置取物理 px，与对话独立窗/悬浮球同约定）
fn persist_geometry(app: &AppHandle, label: &str) {
    let Some(win) = app.get_webview_window(label) else {
        return;
    };
    // 最小化时 Windows 把窗口搬到 (-32000,-32000) 并触发 Moved，落盘会让下次恢复跑到屏外
    if win.is_minimized().unwrap_or(false) {
        return;
    }
    let (Ok(pos), Ok(size)) = (win.outer_position(), win.inner_size()) else {
        return;
    };
    let scale = win.scale_factor().unwrap_or(1.0);
    let width = (size.width as f64 / scale).clamp(MIN_WIDTH, MAX_WIDTH).round();
    let height = (size.height as f64 / scale).clamp(MIN_HEIGHT, MAX_HEIGHT).round();
    let (x, y) = (pos.x as f64, pos.y as f64);

    let _guard = config::lock();
    let mut cfg = config::load();
    if (cfg.web_window_x, cfg.web_window_y) == (Some(x), Some(y))
        && cfg.web_window_width == width
        && cfg.web_window_height == height
    {
        return;
    }
    cfg.web_window_width = width;
    cfg.web_window_height = height;
    cfg.web_window_x = Some(x);
    cfg.web_window_y = Some(y);
    if let Err(e) = config::save(&cfg) {
        log::warn!("外部网页窗几何落盘失败: {e}");
    }
}

/// 拖拽/缩放结束后防抖落盘（spawn 短命线程等窗口静止，不占用消息线程）
fn schedule_persist(app: &AppHandle, label: String) {
    GEOMETRY_TICK.fetch_add(1, Ordering::Relaxed);
    if PERSIST_IN_FLIGHT.swap(true, Ordering::Relaxed) {
        return;
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut seen = GEOMETRY_TICK.load(Ordering::Relaxed);
        loop {
            std::thread::sleep(std::time::Duration::from_millis(GEOMETRY_DEBOUNCE_MS));
            let now = GEOMETRY_TICK.load(Ordering::Relaxed);
            if now != seen {
                seen = now;
                continue;
            }
            persist_geometry(&handle, &label);
            let latest = GEOMETRY_TICK.load(Ordering::Relaxed);
            if latest == seen {
                PERSIST_IN_FLIGHT.store(false, Ordering::Relaxed);
                return;
            }
            seen = latest;
        }
    });
}

/// 挂窗口事件：关闭 → 隐藏常驻（绝不销毁）；拖拽/缩放 → 防抖落盘几何
fn attach_events(app: &AppHandle, win: &WebviewWindow, label: String) {
    // 闭包要求 'static，必须把 owned 的 handle 捕进去（不能用借来的 app）
    let handle = app.clone();
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide(&handle);
        }
        tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
            schedule_persist(&handle, label.clone());
        }
        _ => {}
    });
}

/// 建窗（隐藏常驻；仅 `init` 与测试调用，运行期绝不建窗）
fn build(app: &AppHandle, slot: usize) -> tauri::Result<WebviewWindow> {
    let cfg = config::load();
    let label = label_of(slot);
    let win = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        // 初值 about:blank：预创建不替用户发起任何外部请求（理由见模块头注释）
        WebviewUrl::External(tauri::Url::parse("about:blank").expect("about:blank 恒可解析")),
    )
    .title("网页")
    // 外部站点无法自绘标题栏（我们不往外部页面注入任何脚本），用系统标题栏
    .decorations(true)
    .inner_size(
        cfg.web_window_width.clamp(MIN_WIDTH, MAX_WIDTH),
        cfg.web_window_height.clamp(MIN_HEIGHT, MAX_HEIGHT),
    )
    .min_inner_size(MIN_WIDTH, MIN_HEIGHT)
    .resizable(true)
    // 常规窗口：进任务栏，便于 Alt+Tab 唤回（与对话独立窗同口径，浮窗才走 win_taskbar）
    .skip_taskbar(false)
    .visible(false)
    .additional_browser_args(crate::ADDITIONAL_BROWSER_ARGS)
    .build()?;

    // 位置：有记忆用记忆（物理 px），首次落在主窗中央略偏右下
    let target = match (cfg.web_window_x, cfg.web_window_y) {
        (Some(x), Some(y)) => Some((x as i32, y as i32)),
        _ => initial_center(app, cfg.web_window_width, cfg.web_window_height),
    };
    if let Some((x, y)) = target {
        let _ = win.set_position(PhysicalPosition::new(x, y));
    }

    attach_events(app, &win, label.clone());
    log::info!("外部网页窗口已创建（隐藏常驻）: {label}");
    Ok(win)
}

/// 启动期预创建：**无条件**建窗 + 隐藏常驻（约定 41 铁律）。
///
/// 不做「按需惰性建窗」——运行期现场 build WebView2 会与悬浮球/剪贴板等窗口操作交错挂死
/// 整 app（v0.5.x 三次事故）。未打开时多付一份 `about:blank` renderer 的内存
/// （远低于加载真实站点），这是铁律已定价的代价。
pub fn init(app: &AppHandle) {
    for slot in 0..SLOT {
        let label = label_of(slot);
        if app.get_webview_window(&label).is_some() {
            continue;
        }
        match build(app, slot) {
            Ok(_) => {}
            Err(e) => log::warn!("外部网页窗口预创建失败 [{label}]: {e}"),
        }
    }
}

/// 收起：只隐藏，窗口常驻复用（关闭按钮 / 用户主动关闭共用此路径）
pub fn hide(app: &AppHandle) {
    for slot in 0..SLOT {
        let label = label_of(slot);
        let Some(win) = app.get_webview_window(&label) else {
            continue;
        };
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
            // 落盘投给后台线程：persist 含配置锁 + fsync，CloseRequested 回调在主线程上
            let handle = app.clone();
            std::thread::spawn(move || persist_geometry(&handle, &label_of(slot)));
            log::info!("外部网页窗口已隐藏（常驻复用）: {label}");
        }
    }
}

/// 打开（或聚焦）外部站点窗口。
///
/// 运行期只做 navigate + show，**不建窗**（约定 41）：
/// 1. 校验地址（仅 https 公网，见 `validate_url`）；
/// 2. 取常驻窗口（预创建失败时明确报错，不做运行期 build 兜底）；
/// 3. `show` 先于 `navigate` —— 隐藏态 WebView2 可能不执行页面脚本（notify.rs 同款坑），
///    先显示再导航可确保页面在可见状态下初始化；
/// 4. 目标与「已请求地址」不同才导航，避免连点重载丢掉正在进行的对话（`reload` 可强制）。
pub fn open(app: &AppHandle, raw_url: &str, title: Option<&str>, reload: bool) -> Result<String, String> {
    let url = validate_url(raw_url)?;
    let target = url.as_str().to_string();

    let Some(win) = app.get_webview_window(&label_of(0)) else {
        return Err(
            "NOT_FOUND: 外部网页窗口不存在（启动期预创建失败），重启 x-hub 后可用".to_string(),
        );
    };

    // 首次打开（尚无位置记忆）时补一次落位：预创建发生在主窗就绪前，取不到「主窗中央」
    {
        let cfg = config::load();
        if (cfg.web_window_x, cfg.web_window_y) == (None, None) {
            if let Some((x, y)) = initial_center(app, cfg.web_window_width, cfg.web_window_height) {
                let _ = win.set_position(PhysicalPosition::new(x, y));
            }
        }
    }

    if win.is_minimized().unwrap_or(false) {
        let _ = win.unminimize();
    }
    if !win.is_visible().unwrap_or(false) {
        let _ = win.show();
    }
    let _ = win.set_focus();

    if let Some(t) = title {
        let _ = win.set_title(t);
    }

    let need_navigate = reload || requested_url().lock().map(|u| *u != target).unwrap_or(true);
    if need_navigate {
        if let Err(e) = win.navigate(url) {
            return Err(format!("FAILED: 导航失败（{e}）"));
        }
        if let Ok(mut slot) = requested_url().lock() {
            *slot = target.clone();
        }
        log::info!("外部网页窗口导航: {target}");
    }

    Ok(target)
}

/// 当前状态（扩展据此显示「已打开 / 未打开」，无需自己记）
pub fn state(app: &AppHandle) -> WebWindowState {
    let visible = app
        .get_webview_window(&label_of(0))
        .map(|w| w.is_visible().unwrap_or(false))
        .unwrap_or(false);
    WebWindowState {
        open: visible,
        url: requested_url()
            .lock()
            .map(|u| u.clone())
            .unwrap_or_default(),
    }
}

/// 供 `config::merge_disk_authoritative` 以磁盘为准保留的字段集
/// （几何由本模块写盘，防止前端启动快照把拖拽后的位置覆盖回去）
pub fn preserve_disk_fields(merged: &mut config::AppConfig, disk: &config::AppConfig) {
    merged.web_window_width = disk.web_window_width;
    merged.web_window_height = disk.web_window_height;
    merged.web_window_x = disk.web_window_x;
    merged.web_window_y = disk.web_window_y;
}

// ---------- 命令（宿主前端入口；扩展侧走桥 API `webview.*`） ----------

/// 打开外部网页窗口
#[tauri::command]
pub fn web_window_open(
    app: AppHandle,
    url: String,
    title: Option<String>,
    reload: Option<bool>,
) -> Result<WebWindowState, String> {
    open(&app, &url, title.as_deref(), reload.unwrap_or(false))?;
    Ok(state(&app))
}

/// 关闭（隐藏）外部网页窗口
#[tauri::command]
pub fn web_window_close(app: AppHandle) {
    hide(&app);
}

/// 查询外部网页窗口状态
#[tauri::command]
pub fn web_window_state(app: AppHandle) -> WebWindowState {
    state(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_sites() {
        assert!(validate_url("https://chat.deepseek.com/").is_ok());
        assert!(validate_url("https://www.kimi.com/chat").is_ok());
        assert!(validate_url("  https://chat.deepseek.com/a/chat/s/xxx  ").is_ok());
    }

    #[test]
    fn rejects_non_https_schemes() {
        for bad in [
            "http://chat.deepseek.com/",
            "file:///C:/Windows/win.ini",
            "javascript:alert(1)",
            "data:text/html,<h1>x</h1>",
            "about:blank",
        ] {
            assert!(validate_url(bad).is_err(), "{bad} 应被拒绝");
        }
    }

    #[test]
    fn rejects_local_and_ip_hosts() {
        for bad in [
            "https://127.0.0.1/",
            "https://127.0.0.1:3000/admin",
            "https://192.168.1.1/",
            "https://10.0.0.5/router",
            "https://[::1]/",
            "https://[fe80::1]/",
            "https://localhost/",
            "https://foo.localhost/",
            "https://printer.local/",
            "https://svc.internal/",
        ] {
            assert!(validate_url(bad).is_err(), "{bad} 应被拒绝");
        }
    }

    #[test]
    fn rejects_empty_and_overlong() {
        assert!(validate_url("").is_err());
        assert!(validate_url("   ").is_err());
        assert!(validate_url(&format!("https://a.com/{}", "x".repeat(MAX_URL_LEN))).is_err());
    }

    #[test]
    fn labels_are_one_based() {
        assert_eq!(label_of(0), "web-1");
        assert_eq!(label_of(1), "web-2");
    }
}

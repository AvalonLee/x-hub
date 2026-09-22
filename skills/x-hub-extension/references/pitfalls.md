# 易错点清单

> **何时读我**：写代码前扫一眼，交付前逐条对一遍。每条都是实机踩过的。

1. **命名**：是 extension 不是 plugin / 插件；`id` 必须是**你自己的**反向域名（`com.你的域名.<短名>`）。**`com.x-hub.*` 是平台保留命名空间**（只给官方自营），第三方用了会在服务端关卡被拒——而本地预检只提示不拦，容易一路拖到上传才发现。
2. **`entry` 是 HTML**，不是 `.js` 文件。
3. **npm script 用 `deploy`**，不要叫 `install`（npm 生命周期钩子会误触发）。
4. **service 端口**：读 `process.env.PORT`，只监听 `127.0.0.1`；`/healthz` 是健康检查路径。
5. **前端调后端**：走 `window.xhub.service.request`，不直接 fetch 端口。
6. **桥 API 现状**以 `runtime.info().capabilities` 为准。已实现：`runtime(info/open/callExtension)`、`storage.*`、`config.*`、`sharedStorage.*`、**`data.*` 读写全套**、`fs.saveText/saveFile/saveAs`、`service.request`、`theme.get`、`events.on/emit`、`openExternal`、`expose`、`webview.open/close/state`。未实现：`clipboard.*`、`net.*`、`system.*`、`ui.*`、`fs.readText/writeText/readDir/exists`。
6b. **开外链一律用 `window.xhub.openExternal(url)`**：宿主用 Tauri/wry 承载扩展 iframe，wry 在宿主未注册新窗口处理器时对 WebView2 的 `NewWindowRequested` 直接 `SetHandled(true)` 拒绝——`target="_blank"` 和 `window.open()` 在宿主里**静默失效**（点了没反应）。更坑的是浏览器直开预览时它们"看起来正常"，所以这个坑只在宿主内才暴露。只放行 `http(s)://`。
6c. **module 多形态：URL query 的形态要自己落到 DOM 属性上**。宿主写 `data-xhub-variant` + 广播事件，但 `?xhub-variant=dual` 只在首帧有值；纯 CSS 分支（`:root[data-xhub-variant="dual"]`）要生效，得在读到 query 后 `document.documentElement.setAttribute('data-xhub-variant', v)`——否则浏览器预览里形态分支永远不触发。
7. **权限声明**：读宿主数据要 `data:read`、写数据要 `data:write`、存文件到下载目录要 `fs`、广播事件要 `events`、跨扩展共享存储要 `shared-storage`；没声明就调用会被拒（`PERMISSION_DENIED`）。
8. **页面底用 `var(--xhub-page-bg, transparent)`**（无壁纸=宿主页面背景，有壁纸=transparent）；内容表面用 `var(--xhub-surface)`；**切勿用 `--xhub-bg-page` 铺底**（壁纸态会盖住壁纸，透底态是白底白字）；更细的壁纸态适配用 `data-xhub-wallpaper` / `data-xhub-wallpaper-clear` / `data-xhub-immersive`。
9. **部署 service 扩展前**：若 x-hub 正在运行并锁定了该扩展的后端文件，`deploy` 会报 EPERM——先退出 x-hub 再部署。
10. **字段名大小写**：`openIn`、`minSize`、`dependsOn`、`backend.engine.minVersion` 是驼峰，写错会解析失败。
11. **`requires` 用 `namespace.method`**：写宿主桥 API 能力名（如 `data.notes.list`），不是权限名（`data:read`）；写错会在扩展中心标「缺能力」。
12. **不要用 CDN**：Tailwind / Font Awesome / 外链字体在宿主 webview 里可能被 CSP 或离线拦掉，入口资源全部本地化。
13. **深浅双主题要双声明 fallback**：`:root` 浅色兜底 + `:root[data-xhub-theme="dark"]` 深色兜底，否则无宿主预览时深色露馅；强调色家族的派生 token（primary / soft / text）最容易漏。
14. **`xhub.storage` 异步且跨形态共享**：先读回再首绘；同扩展 module / view / window / drawer 共用同一份，键名自行保证唯一（如 `'progress'`、`'notes'`）。
15. **`window` / `drawer` 与 view 共用入口**：`entry.window` 直接指向 `./view/index.html` 即可，不必复制一份页面。
16. **`--xhub-bg-page` 是渐变，不是颜色**：`color: var(--xhub-bg-page)` 属无效声明，会静默退化成本身继承色（「看着碰巧对」，换个主题就错）。强调色实底上的文字色按主题显式给（亮色主题的 accent 是深色 → 白字；暗色主题 accent 是浅色 → 深字）。
17. **`data.*` 的 `update` 是全量覆盖**：可省字段会被写成空值（`notes.update` 不传 `content` 清空正文）。改前先 `get` 合并再提交；`todos` / `stickies` 的并发写用 `expectedVersion` 乐观锁，冲突会 reject `VERSION_CONFLICT`。
18. **CSS 里没有条件表达式**：module 多形态分支只能用属性选择器 `:root[data-xhub-variant="compact"] { … }`，不能写 `var(--xhub-variant) === 'compact' ? … : …`。
19. **别用 `__dirname` 反推宿主数据根，数据也别直接写扩展目录**：开发目录直挂时扩展的上一级是版本目录 / 任意源码目录，**不是** `extensions`，反推必然落空；而「反推失败就用扩展目录兜底」会把 `data.json` / `sync.log` 直接写进源码目录——若这个目录正是你的**发布源码目录**，用户数据会被一起打包上传（实机事故：共享待办数据进了待审包）。要落盘的数据一律写扩展目录内的 **`.data/`**（点开头：宿主的打包与热重载都会跳过），旧数据迁移留下的备份文件也放那里。宿主自己的 `.storage.json` / `.config.json` 同样写在扩展目录里（直挂时 = 你的源码目录），那是宿主的键值存储，别拿它当你的数据目录用。
20. **后端起不来先看日志，别猜**：宿主把 service 后端的 stdout/stderr 落盘到 `<数据根>\logs\service\<扩展 id>.log`（单份上限 1MB，每次启动写一条 `----- service 启动 <时间> -----` 分隔标记），探活失败时 `x-hub.log` 里还会带 **`exit=<退出码>` + 日志尾部 20 行**。`runtime.info().serviceReady === false` 只说明「端口没探通」——可能没起来、起来就崩、也可能端口被占，先把那个文件打开看；不要对用户写「正在下载 Node」这类猜测性文案。
21. **路径别原样交给外部程序（Windows `\\?\` 前缀）**：宿主喂给后端的脚本路径自 v0.6.2 起已归一（不带前缀），但 `__dirname` 本身在直挂目录下仍可能是 verbatim 形式（`\\?\C:\…`）——后端自己 `spawn` 子进程、调 `ffmpeg` / `git` 之类的命令行工具、或把路径拼进命令行参数时，一律先剥前缀（`\\?\C:\x` → `C:\x`，`\\?\UNC\srv\share` → `\\srv\share`），否则对方读不了这个路径（Node 就连自己都加载不了带前缀的脚本路径）。
22. **`module` 卡片的视口可以矮到 ~86px，别假设「至少 150px」**：卡片高度由工作台网格决定（行高下限 36px），4×2 的小格子内容区可能只有 86px 左右，**若开了宿主表头还要再减约 30px**（0.6.3 及更早还有宿主 bug 让 iframe 恒为浏览器默认的 150px、矮卡底部被裁，已修）。入口用 `height: 100%` + `box-sizing: border-box` + `cqh`/`clamp()` 排版、溢出交给 `overflow: hidden/auto`；写死 `min-height` 或依赖 150px 默认高度，在矮格里就会被裁。宿主**不给** iframe 内边距，留白自己写（约定 12px，见 `surfaces.md`）。
23. **宿主表头默认不开，别和它抢标题**：扩展 `module` 卡**默认没有**宿主表头，卡面完全由你自己画。只有写了 `"moduleOptions": { "defaultHideTitle": false }` 时，宿主才会在卡顶画一行「品牌色图标 + `manifest.name`」——这时入口**不要再画一遍同名标题**（会重复），卡名也请确认过。用户能在布局编辑器里用「Aa」按卡片覆盖作者默认。
24. **外部网站一律不能 iframe 嵌（「把某个网页搬进 x-hub」的第一道墙）**：主流网站普遍用 `Content-Security-Policy: frame-ancestors` 或 `X-Frame-Options` 拒绝被嵌入，实测 AI 网页版无一例外——`chat.deepseek.com` 是 `frame-ancestors 'none'`；通义 / Qwen / 秘塔回 `frame-ancestors` 白名单；ChatGPT / Grok / Perplexity 回 `X-Frame-Options: SAMEORIGIN`；Gemini 回 `DENY`。Kimi / 豆包 / 元宝 / 清言 / 文心 / 星火首页**没设**这两个头（豆包只有 `report-only`，仅上报不拦截），但①真实拦截面在接口层，②iframe 里的站点对浏览器是**第三方上下文**（Cookie 受限、存储被分区），登录态照样保不住——**「能嵌进去」≠「能正常用」**。唯一可行路径是 `window.xhub.webview.open({ url })`：宿主开**原生窗口**走顶层导航，不受 frame-ancestors 约束、Cookie 是第一方、流式对话正常。所以这类扩展的正确形态是**入口壳**（view + module 做站点库与快捷入口，点开走原生窗口），而不是把网页嵌进自己的页面。只放行 https 公网地址：IP 字面量与 `localhost` / `.local` / `.internal` 会被拒。
25. **宿主新能力要「两处同改」，只注册能力表 = 扩展侧拿到 `undefined`**：`window.xhub` 不是按能力表自动生成的，而是宿主在 `src-tauri/src/extension.rs` 的 `XHUB_BRIDGE_SCRIPT` 里**手写**的映射表，与 Rust 侧 `CAPABILITIES` 是**两份独立清单**（桥里的 `events.emit` / `runtime.open` 还走 postMessage 而非 call 通道，更不能指望自动对齐）。只加 `CAPABILITIES` 而漏加桥封装时：`runtime.info().capabilities` **会宣称该能力可用**（探测误判通过），扩展再写 `window.xhub.<ns>.<method>()` 就是 `Cannot read properties of undefined (reading '<method>')`——实机事故即 `webview.*`（2026-09-22：页面首屏轮询就报错，用户没点也报）。**自查两条**：① 能力探测**同时**校验能力表与桥对象（`caps.some(…) && !!(window.xhub && window.xhub.<ns>)`），只查一边必然误判；② 宿主侧已有 `bridge_script_covers_all_capability_namespaces` + `bridge_script_covers_webview_capabilities` 单测兜底，新增能力时它们会红。

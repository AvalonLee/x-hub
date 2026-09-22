# 桥 API（window.xhub）

> **何时读我**：要用宿主数据、存储、配置、事件、文件导出、service 请求时。**先在这里确认 API 是否已实现、需要什么权限**，再写代码。

宿主在加载扩展入口 HTML 时**自动注入** `window.xhub`。扩展脚本直接调用，**不要 `import` 任何 xhub 包**。

所有方法返回 `Promise`，失败 reject 带 `code` 的 `Error`（`PERMISSION_DENIED`、`NOT_FOUND`、`VERSION_CONFLICT` 等）。

## 权限速查

| 命名空间 | 权限 |
|---|---|
| `runtime.*`、`storage.*`、`config.*`、`theme.*`、`service.request`、`openExternal` | **无需权限** |
| `data.*` 读方法 | `data:read` |
| `data.*` 写方法 | `data:write` |
| `sharedStorage.*` | `shared-storage` |
| `fs.saveText` / `saveFile` / `saveAs` | `fs` |
| `webview.open` / `close` / `state` | `webview` |
| `events.emit` | `events` |
| `events.on`、`xhub.expose` | 无需权限 |
| `net.fetch`（**@planned 未实现**） | `network` |

> **`network` 当前不对应任何可用 API**：它唯一生效的地方是 service 后端对外监听（`manifest.backend.host` 非回环）。普通扩展、包括后端自己发外部请求的 service 扩展，都**不需要**它。详见 `manifest.md` 的 `backend` 表。

**权限是怎么被检查的**：宿主与发布预检会**静态扫描**扩展目录里的 `.html/.js/.mjs/.cjs`，把出现的 `xhub.…` 调用链反推成所需权限，再和 `manifest.permissions` 对账——**代码用到却没声明 = 发布被 error 挡住**；声明了代码里没有 = warn（`network` 除外，它无法静态检测）。扫描是纯文本匹配，注释里写的调用示例也算数。完整规则见 `debug-deploy.md` 的「平台关卡会查什么」。

## 已实现（done，可直接用）

```js
await window.xhub.runtime.info()   // { id, name, version, runtime, serviceReady, proxyPrefix, capabilities }
window.xhub.runtime.open(surface)  // 打开指定形态（view/window/drawer/module），无需权限
await window.xhub.runtime.callExtension('com.x-hub.token-stats', 'getData', {})  // 调其它扩展暴露的方法

// 用系统默认浏览器打开外链（无需权限，只放行 http/https）
window.xhub.openExternal('https://example.com')
// ⚠️ 扩展里**不要**用 target="_blank" 或 window.open 开外链：宿主用 Tauri/wry 承载 iframe，
// wry 在宿主未注册新窗口处理器时对 WebView2 的 NewWindowRequested 直接 SetHandled(true) 拒绝，
// 两种写法在宿主里都是**静默失效**（点了没反应，只有浏览器直开预览时才"看起来正常"）。

await window.xhub.storage.get(key)        // 无则 null；按扩展隔离持久化
await window.xhub.storage.set(key, value) // value 需可 JSON 序列化
await window.xhub.storage.remove(key)
await window.xhub.storage.clear()

await window.xhub.sharedStorage.get(key)     // 跨扩展共享（需 shared-storage 权限）
await window.xhub.sharedStorage.set(key, value)
await window.xhub.sharedStorage.remove(key)

await window.xhub.config.all()            // 合并后的完整配置对象
await window.xhub.config.get(key)         // 用户覆盖值 ?? manifest.config 默认 ?? null
await window.xhub.config.set(key, value)  // 写用户覆盖层（升级扩展不冲掉）
await window.xhub.config.remove(key)      // 删用户覆盖，回退默认

await window.xhub.theme.get()             // { mode, preset, accent, wallpaper, tokens }
window.xhub.events.on('theme-changed', fn)  // 返回取消订阅函数
window.xhub.events.on('xhub:variant-changed', fn)  // module 形态切换
window.xhub.events.on('my-event', fn)       // 订阅其它扩展广播的自定义事件
window.xhub.events.emit('my-event', data)   // 广播给其它扩展（需 permissions: ["events"]）
window.xhub.expose('getData', fn)           // 暴露方法给其它扩展（配合 manifest.expose）

await window.xhub.service.request('/api/x', { method, headers, body })  // 仅 service 扩展
// → XHubHttpResult { status, headers, text(), json() }

// ---- 在原生窗口里打开外部网站（需 webview 权限）----
// ⚠️ 外部站点**不能**用 iframe 嵌：主流 AI 网页版一律回 frame-ancestors / X-Frame-Options
// 拒绝被嵌入，而扩展四种形态全由 iframe 承载 —— 嵌进去必然白屏。要带用户去外部网站，
// 只有这一条路：宿主开一个原生窗口走**顶层导航**，Cookie 是第一方，登录态可持久。
await window.xhub.webview.open({ url: 'https://chat.deepseek.com/', title: 'DeepSeek' })
// → { open: true, url: 'https://chat.deepseek.com/' }；只放行 https 公网地址
await window.xhub.webview.state()   // → { open, url }（渲染「已打开」态用）
await window.xhub.webview.close()   // 只隐藏，页面与登录态都保留；重开即恢复现场
// 重复 open 同一个 url **不会**重新加载（对话不丢）；要强制重载传 { reload: true }

// ---- 把文件存到「系统下载」目录（需 fs 权限；单文件 ≤64MB，重名自动去重）----
// ⚠️ 参数是**位置参数**，不是对象（与 data.* 写方法的对象风格不同，别照抄那边）
await window.xhub.fs.saveText(name, content)   // 文本 → { path, name }
await window.xhub.fs.saveFile(name, base64)    // 二进制 → { path, name }
await window.xhub.fs.saveAs(name, base64)      // 弹系统「保存到…」对话框；取消返回 { canceled: true }
```

### 宿主数据 `data.*`（读写两套都已实装）

```js
// 笔记
await window.xhub.data.notes.list() / get(id) / create({ title }) / update({ id, title, content? }) / delete({ id })

// 待办
await window.xhub.data.todos.list() / get(id)
await window.xhub.data.todos.create({ title, parentId?, createdAt? })
await window.xhub.data.todos.update({ id, title, priority?, expectedVersion? })
await window.xhub.data.todos.toggle({ id, expectedVersion? })
await window.xhub.data.todos.delete({ id, expectedVersion? })
await window.xhub.data.todos.schedule({ id, dueAt?, remindAt?, expectedVersion? })  // 毫秒时间戳，null 清除

// 待办 v1（描述 / 置顶 / 周期 / 标签）
await window.xhub.data.todos.setDescription({ id, description, expectedVersion? })   // 轻量 Markdown 正文，最长 200 字（超限报 INVALID_ARGUMENT）
await window.xhub.data.todos.setPinned({ id, pinned, expectedVersion? })             // 置顶：脱离日期分组
await window.xhub.data.todos.setRepeat({ id, repeat, expectedVersion? })             // 周期规则，mode:'once' 取消
await window.xhub.data.todos.setTags({ id, tagIds })                                 // 全量替换该待办的标签
await window.xhub.data.todos.completeRecurring({ id, expectedVersion? })             // 周期「完成本轮」
await window.xhub.data.todos.undoRecurring({ id, expectedVersion? })                 // 撤销「完成本轮」
await window.xhub.data.todos.expandOccurrences({ fromMs, toMs })                     // → [{ todo_id, at_ms }]；区间上限 366 天，超了报 RECURRENCE_RANGE_TOO_LARGE，需分段查

// 待办标签（**与笔记标签 data.tags 是两套独立定义**，见 ADR 0010）
await window.xhub.data.todoTags.list() / links()                                     // 定义列表 / 全部待办-标签关联
await window.xhub.data.todoTags.create({ name, color? })                             // 同名返回既有（不覆盖颜色）
await window.xhub.data.todoTags.update({ id, name, color? }) / delete({ id })

// 便签（slot 1-2）
await window.xhub.data.stickies.list() / save({ slot, content?, expectedVersion? })   // upsert
await window.xhub.data.detachedStickies.list() / save({ slot, content?, expectedVersion? })  // 仅更新已脱离的浮窗便签

// 速达资源
await window.xhub.data.resources.list() / get(id)
await window.xhub.data.resources.create({ kind, name, target, category?, icon?, args? })
await window.xhub.data.resources.update({ id, kind, name, target, category?, icon?, args? })
await window.xhub.data.resources.delete({ id })

// 提示词
await window.xhub.data.snippets.list() / get(id) / create({ title, content? })
await window.xhub.data.snippets.update({ id, title, content? }) / delete({ id }) / togglePin({ id })

// 标签
await window.xhub.data.tags.list() / ofNote(noteId) / create({ name }) / delete({ id })
await window.xhub.data.tags.setNoteTags({ noteId, tagIds })   // 全量替换语义
```

**两条必须记住的语义**：

1. **`update` 是全量覆盖**——可省字段缺省会被写成空值（`notes.update` 不传 `content` 会清空正文）。**改前先 `get` 合并再提交**。
2. **乐观锁**：`todos` / `stickies` / `detachedStickies` 的写方法支持 `expectedVersion`（= 上次读到的 `version`）。与他人（含宿主 UI）并发修改冲突时 reject `VERSION_CONFLICT`；缺省不校验。

**周期待办（`setRepeat` / `completeRecurring`）三条语义**：

1. **规则只实现于宿主**（Rust `todo_recurrence.rs`）。扩展只负责收集参数落库、用 `expandOccurrences` 取展开结果渲染；**不要在前端重写一套规则**，否则与宿主日历/勾选行为漂移。
2. **勾选周期待办 = 「完成本轮」**：`due_at` 滚到下一个**未来**时刻（逾期不补历史）、`remind_at` 按同一偏移平移并重新武装、`repeat_done_count +1`、子待办全部复位；**不置 `done`**（周期条目永不进已完成列表）。规则用尽（`until` 越界 / `count` 用尽）时自动转回 `once` 并保留该行。
3. **`setRepeat` 要求该待办已有 `due_at`**：基准时刻取 `due_at` 的时分。没有截止时间的周期待办既算不出下一轮、也拒绝 `completeRecurring`，属于无效状态——先 `schedule` 设截止再 `setRepeat`。

**待办相关上限（超限直接报错，不静默截断）**：

1. **`expandOccurrences` 单次区间上限 366 天**：超限报 `RECURRENCE_RANGE_TOO_LARGE`（展开要迭代并全程持有数据库锁，宿主日历自己只查 42 天）。画长跨度日历请分段调用后合并。
2. **`setDescription` 正文上限 200 字**：超限报 `INVALID_ARGUMENT`。它是卡面/浮层展示的短说明，不是长文档——长内容请用笔记（`notes`）。

### 数据模型字段（snake_case）

- `Note { id, title, content, created_at, updated_at }`
- `Todo { id, title, done, priority(0/1/2), created_at, updated_at, completed_at, due_at, remind_at, parent_id, sort_order, version, description, pinned, repeat_mode, repeat_every, repeat_unit, repeat_weekdays, repeat_month_day, repeat_month_nth, repeat_end_mode, repeat_end_at, repeat_count, repeat_done_count, repeat_last_done_at }`
- `TodoTag { id, name, color, sort_order, created_at }`（`color: ''` = 用默认色，否则 `#rrggbb`）
- `TodoTagLink { todo_id, tag_id }`
- `TodoOccurrence { todo_id, at_ms }`（虚拟实例，不落库）
- `Resource { id, kind(app/web/file), name, target, category, icon, args, sort_order, last_launched_at, created_at, updated_at }`
- `Sticky / DetachedSticky { id, slot, content, version }`（Detached 另有 `x` / `y`）
- `Snippet { id, title, content, version }`
- `Tag { id, name }`（笔记标签；待办标签见 `TodoTag`）

## 未实现（planned，勿依赖）

`clipboard.*`、`net.*`、`system.*`（`openUrl` / `openPath` / `openApp`）、`ui.*`（含 `ui.toast` / `ui.notify`）；
`fs.readText` / `fs.writeText` / `fs.readDir` / `fs.exists`（受控读写那一套——**与可用的 `fs.saveText` / `saveFile` / `saveAs` 不是一回事**）；
`data.usage.*`（已从宿主**移除**，AI 用量现在由扩展 `com.x-hub.token-stats` 自行读数据实现）。

## 判据：能不能用以运行期为准

`await window.xhub.runtime.info()` 返回的 **`capabilities`** 是宿主注册的真实能力表（`[{ namespace, method, permission }]`）。

- 扩展可据此探测能力并**优雅降级**；`manifest.requires` 就是用它做装前兼容校验。
- `xhub.d.ts` 里标 `@planned` 的只是提示。**两者冲突时以 capabilities 为准**，并把冲突反馈给宿主维护者。

## 编辑器补全：xhub.d.ts

本 skill 目录附带 **`xhub.d.ts`**（`window.xhub` 的完整 TypeScript 类型声明）。

- 把它连同扩展源码放进同一个项目，VS Code 就会给 `window.xhub.*` 补全、参数提示，并在方法上方显示「需 `data:read`」这类权限说明。
- 扩展**不需要 import 它**（`window.xhub` 由宿主注入），它只是给工具看的。
- 纯 JS 项目在项目根放一份 `jsconfig.json`：

```json
{ "compilerOptions": { "checkJs": false }, "include": ["xhub.d.ts", "**/*.js", "**/*.html"] }
```

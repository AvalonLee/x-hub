# DeepSeek（x-hub 扩展）

在 x-hub 的**原生窗口**里打开 [DeepSeek 网页版](https://chat.deepseek.com/) —— 登录态可持久、流式对话正常。

## 形态

| 形态 | 作用 |
|---|---|
| `view` | 完整入口页（侧栏进入）：打开 / 唤起 / 收起窗口，显示打开次数与时间 |
| `module` | 工作台卡片：整卡点击即打开，卡面显示窗口是否已打开 |

## 权限

只声明一项：`webview` —— 在原生窗口里打开外部网站。**不读宿主数据、不联网、不碰文件。**

## 为什么是独立窗口，而不是把网页嵌进扩展页面

扩展的四种形态（view / module / window / drawer）**全部由 iframe 承载**，而 DeepSeek 官网明确拒绝被嵌入：

```
content-security-policy: frame-ancestors 'none'
```

嵌进 iframe 只会白屏。所以走宿主提供的 `webview.*` 桥 API：开一个**原生窗口**做顶层导航 ——
顶层导航不受 `frame-ancestors` 约束，Cookie 是第一方，登录态能持久、SSE 流式回复也正常。

窗口是**常驻复用**的：`close` 只隐藏，页面与对话都还在，重开即恢复现场；
重复 `open` 同一个地址不会重新加载（正在进行的对话不会丢），要强制刷新用 `open({ reload: true })`。

## 接入调试

扩展中心 →「**我的扩展**」标签页 → 添加本目录（须含 `manifest.json`）→ **添加即加载**，
打开扩展就能跑真机，改代码约 1.5 秒自动重载，移除目录即撤销。
`module` 形态还要到 设置 →「工作台 → 自定义布局」把卡片拖进网格。

## 加别的站点（Kimi / 豆包 / 通义…）

本扩展是按「单站点入口」写的，加站点只需两处：

1. `manifest.json` 的 `name` / `id`（换成新扩展，例如 `com.avalonlee.kimi`）；
2. 两个入口 HTML 里的 `SITE_URL` / `SITE_TITLE` 常量。

宿主侧不用改、不用重编译：`webview.open` 只校验「https + 公网主机」，不维护站点白名单。
注意通义 / Qwen / 秘塔 这类站点虽然接口层可能有额外风控，但原生窗口路径与 DeepSeek 完全一致，
同样可用。

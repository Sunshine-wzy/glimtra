# Glimtra

轻量桌面翻译工具。使用 **Tauri 2 + Rust + TypeScript + Vite**，首版面向 Windows。

## 功能

- 自定义 Chat Completions 完整请求地址、模型和 API Key。
- 流式翻译、取消请求、复制译文、主动粘贴剪贴板、语言切换。
- `Ctrl+Alt+T` 全局唤起/隐藏，可自定义，注册冲突时保留原快捷键。
- `Ctrl+Enter` 翻译，`Esc` 或关闭按钮隐藏到托盘；从托盘明确退出。
- 单实例运行，重复打开会聚焦已有窗口。
- 可切换开机自启动；登录启动只创建托盘，首次唤起时才创建 WebView。
- API Key 按完整 API 地址保存在系统凭据库，普通设置原子写入本地 JSON。
- NSIS 当前用户安装包，缺少 WebView2 时自动下载安装。

## 开发环境

1. Node.js 22.12+（建议 24）和 pnpm 11.25.0。
2. [Rust stable，MSVC 工具链](https://rustup.rs/)。
3. [Visual Studio Build Tools](https://aka.ms/vs/17/release/vs_BuildTools.exe)，选择“使用 C++ 的桌面开发”，包含 MSVC x64/x86 编译工具和 Windows SDK。
4. WebView2 Runtime，见 [Tauri 官方环境说明](https://v2.tauri.app/start/prerequisites/)。

```powershell
pnpm install --frozen-lockfile
pnpm desktop:dev
```

如果项目中存在 `.tools/cargo` 和 `.tools/rustup`，桌面启动脚本会自动使用这套本地 Rust 工具链。该目录已加入 Git 忽略规则，不属于应用安装包。

只预览界面：

```powershell
pnpm dev
```

打开 `http://127.0.0.1:1420`。浏览器模式不会模拟翻译成功，也不会保存密钥或系统设置；这些功能需在桌面应用中验证。

## 连接模型

在“设置”中填写服务商提供的完整 Chat Completions 地址，例如 `https://your-provider.com/v1/chat/completions`，以及模型 ID 和 API Key。填写的地址会直接用于请求，不会自动拼接 `/v1` 或其他路径。

支持 `data: {...}` / `[DONE]` SSE 流，兼容忽略 `stream: true` 而返回完整 JSON 的服务。支持本地 `http://localhost`、`127.0.0.1` 和 `[::1]`，本地无认证服务可以不填 API Key。其他远程地址要求 HTTPS。

更换服务地址时需单独配置密钥。未填写新密钥会保留**该地址**已保存的密钥；不会把原地址的密钥发送给新地址。可在设置中删除当前地址的密钥。

## 构建与检查

```powershell
pnpm build
pnpm check:rust
pnpm test:rust
pnpm desktop:build -- --locked
```

安装包输出位置：`src-tauri/target/release/bundle/nsis/`。默认按当前用户安装。在线安装包会在必要时下载 WebView2；离线分发时可将 `bundle.windows.webviewInstallMode.type` 改为 `offlineInstaller`，包体积会增大。

已添加 Windows GitHub Actions 工作流：构建前端、检查 Rust 格式、运行 Rust 单元测试、生成 NSIS 安装包并上传构建产物。工作流只随代码提交运行，不会发布 GitHub Release。

Rust 单元测试覆盖 UTF-8 在任意字节位置拆包、SSE 多行/未终止事件、超大/无效响应、输出截断、推理字段过滤、服务错误脱敏、URL 校验、设置原子覆盖。测试不需要真实 API Key，不访问真实模型。

### 当前验证状态

已通过 TypeScript 检查、Vite 生产构建、Rust 格式检查，已检查浏览器预览的输入和导航。已生成前后端依赖锁文件。

本机尚缺少 MSVC/Windows SDK：`cargo check` 在依赖构建阶段报 `link.exe not found`。因此桌面后端编译、Rust 单元测试、安装包、自启动/快捷键/凭据库的实机验证尚未完成。没有使用真实模型密钥进行翻译请求，也没有测得桌面进程内存数据。

## 轻量策略与边界

- 原生 TypeScript 界面，无前端 UI 框架、外部字体、动画轮询或常驻 Node 服务。
- 请求直接从 Rust 发往用户配置的模型服务，无中转服务器。
- 开机后台启动时不创建 WebView；首次唤起后，隐藏窗口会保留 WebView 和当前文本，以便快速再次打开。当前尚未实现隐藏后定时销毁 WebView。
- 仅保存配置，不持久化原文、译文或历史记录；退出后当前翻译内容清空。
- 当前处理纯文本及保留原始 Markdown 文本，不渲染模型生成的 HTML。
- 首版未包含截图 OCR、划词取词、全局剪贴板监听、自动更新和代码签名。
- 本地运行模型需要用户自行提供本地服务，其资源消耗不计入客户端。

桌面版本构建后，应测试：快捷键冲突、窗口隐藏/恢复、重复启动、开始菜单启动、登录后托盘启动、卸载后启动项清理；测量 Release 应用及其 WebView2 子进程在托盘冷启动、首次显示、翻译中、隐藏五分钟时的内存和 CPU。

## 代码结构

```text
src/                     TypeScript 界面、样式、IPC 调用
src-tauri/src/lib.rs      生命周期、托盘、快捷键、命令
src-tauri/src/settings.rs 配置持久化与系统凭据库
src-tauri/src/translation.rs LLM 请求与响应处理
src-tauri/src/stream.rs   增量 SSE 解码器
src-tauri/tauri.conf.json 窗口、安全与安装包设置
scripts/desktop.mjs      桌面开发及本地 Rust 工具链入口
```

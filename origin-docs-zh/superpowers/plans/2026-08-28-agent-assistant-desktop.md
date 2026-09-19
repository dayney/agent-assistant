# 代理-助理桌面实施方案

> **对于代理工作人员：** 所需的子技能：使用 superpowers:subagent-driven-development （推荐）或 superpowers:executing-plans 来逐个任务地实施此计划。步骤使用复选框 (`- [ ]`) 语法进行跟踪。

**目标：** 构建一个 macOS 优先的 Tauri 2 客户端，使用已批准的中国原型可视化共享治理源、代理投影和项目配置文件。

**架构：** React/TypeScript 前端在 Tauri 2 shell 内运行。类型化的 `CoreClient` 边界使 UI 状态独立于 Go Agentsync Core。实模式通过 Tauri IPC 调用 Go sidecar；明确标记的演示客户端仍然可供目视检查。

**技术堆栈：** Tauri 2、Rust、React 19、TypeScript、Vite、原生 CSS 令牌、系统继承的字体、现有的 Go Agentsync Core。

**规格：** `docs/superpowers/specs/2026-08-28-agent-assistant-desktop-design.md`

## 全局约束

- 保留现有的Go Core和规范的Agentsync源；本机文件保留渲染输出。
- 使用全局 → 代理 → 项目来源并明确报告 `full`、`partial` 和 `unsupported`。
- 演示模式必须可见，并且不得默默地替换核心错误。
- 请勿添加、加载、捆绑或明确选择字体；所有新文本都会继承当前的系统字体。
- 不要在此垂直切片中添加新的后端、云服务或数据库。
- 使用语义控件、可见焦点样式、键盘导航和响应式布局。
- 分别运行 TypeScript/Vite 和 Rust 检查；成功的前端构建并不能替代核心检查。

### 任务 1：搭建桌面工作区

**文件：**
- 创建：`desktop/package.json`、`desktop/index.html`、`desktop/tsconfig.json`、`desktop/vite.config.ts`
- 创建：`desktop/src-tauri/Cargo.toml`、`desktop/src-tauri/tauri.conf.json`、`desktop/src-tauri/src/main.rs`
- 创建：`desktop/src/main.tsx`、`desktop/src/app.css`

- [ ] **第 1 步：编写失败的工具链检查**

创建需要 `tsc --noEmit` 和 Vite 构建的脚本，以及公开 `health` 命令的 Rust shell。

- [ ] **第 2 步：运行检查以验证脚手架是否缺失**

运行`cd desktop && npm run typecheck`；预期失败，因为 `package.json` 和源条目不存在。

- [ ] **第 3 步：创建最小的 Tauri/Vite 支架**

使用 React 19、TypeScript、Vite 和 Tauri 2 依赖项。保持 Rust 命令较小并返回结构化错误。

- [x] **第 4 步：运行前端和 Rust 检查**

运行 `npm run typecheck`、`npm run build` 和 `cargo check --manifest-path src-tauri/Cargo.toml`；一切都必须通过。

### 任务 2：定义核心客户端合约和演示快照

**文件：**
- 创建：`desktop/src/core/model.ts`、`desktop/src/core/client.ts`、`desktop/src/core/demo-client.ts`
- 测试：`desktop/src/core/model.test.ts`

- [ ] **第 1 步：编写失败的模型测试**

测试能力状态、出处顺序和显式 `mode: 'demo'` 元数据。

- [ ] **第 2 步：运行 `npm run test` 并确认预期的失败**

由于模型和客户端不存在，测试必定失败。

- [ ] **第 3 步：实现类型化模型和演示客户端**

公开 `CoreClient.getSnapshot()` 和 `CoreClient.previewApply()`；演示客户端必须返回经过批准的中国原型数据，并且没有秘密值。

- [x] **第 4 步：运行模型测试和类型检查**

运行`npm run test -- src/core/model.test.ts`和`npm run typecheck`；两者都必须通过。

### 任务3：实现中文应用程序shell

**文件：**
- 创建：`desktop/src/app/App.tsx`、`desktop/src/app/navigation.ts`、`desktop/src/components/StatusBadge.tsx`、`desktop/src/components/ProvenanceChain.tsx`
- 修改：`desktop/src/main.tsx`、`desktop/src/app.css`

- [ ] **第 1 步：为导航和状态文本编写失败的渲染测试**

断言 shell 呈现 `总览`、`全局配置`、`Agent`、`项目` 和 `活动记录`，并且状态标签除了颜色之外还包含文本。

- [ ] **步骤 2：运行测试并观察缺失的组件故障**

运行`npm run test -- src/app`； shell 存在之前的预期失败。

- [ ] **第 3 步：实现 shell 和响应式布局**

使用一个侧边栏/顶栏 shell、语义导航、本机按钮、标记化 CSS 和系统继承字体。每页保留一个主要操作。

- [x] **第 4 步：运行测试并构建**

运行 `npm run test -- src/app`、`npm run typecheck` 和 `npm run build`。

### 任务 4：实施全局、代理、项目和活动视图

**文件：**
- 创建：`desktop/src/views/OverviewView.tsx`、`desktop/src/views/GlobalView.tsx`、`desktop/src/views/AgentsView.tsx`、`desktop/src/views/ProjectsView.tsx`、`desktop/src/views/ActivityView.tsx`
- 修改：`desktop/src/app/App.tsx`、`desktop/src/app.css`

- [ ] **第 1 步：添加失败的视图测试**

涵盖矩阵标签、项目配置文件行、源链和演示模式横幅。

- [ ] **第 2 步：验证缺少视图的测试是否失败**

运行`npm run test -- src/views`；预期的模块/渲染失败。

- [ ] **第 3 步：实施已批准原型中的视图**

将数据保留在类型快照中，避免重复状态，并显式处理加载/错误/演示状态。

- [x] **第 4 步：运行重点测试并构建**

运行 `npm run test -- src/views`、`npm run typecheck` 和 `npm run build`。

### 任务 5：为 Go Core 添加 Tauri IPC 边界

**文件：**
- 创建：`desktop/src-tauri/src/core.rs`、`desktop/src/core/tauri-client.ts`
- 修改：`desktop/src-tauri/src/main.rs`、`desktop/src/core/client.ts`、`desktop/src/app/App.tsx`

- [ ] **第 1 步：添加失败的客户端合同测试**

测试核心错误是否会产生可见的错误状态，并且不会默默地交换到演示数据。

- [ ] **第 2 步：运行测试以捕获缺失的 IPC 实现**

运行`npm run test -- src/core/client.test.ts`；适配器存在之前的预期失败。

- [ ] **第 3 步：实现显式核心模式选择**

在实模式下使用 Tauri `invoke('get_workspace_snapshot')`。仅当 `VITE_CORE_MODE=demo` 显式时才使用演示客户端。以稳定的代码/消息形状返回错误。

- [x] **第 4 步：运行前端和 Rust 检查**

运行 `npm run typecheck`、`npm run build` 和 `cargo check --manifest-path src-tauri/Cargo.toml`。

### 任务 6：验证 UI 质量并记录客户端

**文件：**
- 修改：`README.md`、`docs/architecture.md`、`docs/components.md`、`CHANGELOG.md`
- 测试：任务胶囊中的`desktop/`构建/类型检查和手动屏幕截图证据

- [x] **第 1 步：运行所有前端和 Rust 验证命令**

运行`cd desktop && npm run typecheck && npm run build && cargo check --manifest-path src-tauri/Cargo.toml`。

- [ ] **第 2 步：检查键盘焦点、狭窄布局和演示/错误状态**

使用 1024px、736px 和 360px 的浏览器预览；记录任何重叠或剪切的文本并在完成之前修复它。

- [x] **第 3 步：更新文档和变更日志**

记录桌面 shell、连接的核心边界、显式演示模式以及
剩余的应用/漂移积分边界。

- [x] **第 4 步：运行 `git diff --check` 并查看更改的文件列表**

确认未添加任何字体、机密、生成的本机适配器或不相关的文件。

## 实施状态

任务 1-6 是针对只读连接片实现的。边车
支持 `snapshot` 和 `preview`，使用 `schemaVersion: 1`，并编辑本机
MCP 端点和值。应用、漂移投影、文件监视和写回
有意保留在第一个连接切片之外。
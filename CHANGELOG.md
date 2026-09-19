# Changelog

本项目所有值得记录的变更都在这里。

格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/spec/v2.0.0.html)。

> **关于版本号的说明**
> `v0.15.0` 及以后是 **Desktop 应用**阶段（Tauri + Rust Core + React），对应本仓库 `main` 分支的持续迭代。
> `v0.1.0` ~ `v0.14.0` 是**早期 CLI 工具**阶段（Go），记录了 agentsync 从第一行代码成长为支持 30+ Agent 的全功能同步工具的历程。

---

## [Unreleased]

> 下一个版本待发布的变更将记录在此处。

### 修复

- **本地 macOS 应用图标正确打包** — Tauri bundle 配置现在显式包含 macOS `.icns` 和跨平台 PNG/ICO 图标，避免本地 `.app` 显示默认占位图标。
- **Windows Desktop Core 写入与路径输出跨平台兼容** — 原子替换会先关闭临时文件句柄，快照逻辑路径统一使用 `/`，golden fixture 在 CI 中固定使用 LF 换行。
- **自动更新绑定当前 GitHub 仓库** — 更新检查地址与 `latest.json` 资产地址改为使用 `dayney/agent-assistant`，CI 会从 `GITHUB_REPOSITORY` 派生地址，避免继续访问迁移前的仓库。

---

## [0.0.16] — 2026-09-18

> **Desktop 阶段**：MCP 服务来源盘点与去重精修。

### 修复

- **MCP 服务目录合并重复服务** — canonical 与本机原生配置中的同一服务现在只显示一条，并合并 Agent 适配与凭据摘要；同一 ID 但端点或 Recipe 不同的配置仍保留为独立条目
- **分层去重全局与项目 MCP** — 全局 MCP 与项目 MCP 分层展示，避免跨层级误合并
- **区分本机产品来源与已配置适配器** — Agent 页面汇总全部本机产品来源，已发现和待核实来源分开计数

### 新增

- **完成 MCP 注册表适配与真实更新测试** — MCP 适配、更新与测试使用统一注册表，Antigravity 桌面应用配置链接核验后与 IDE/CLI 共享同一物理配置

---

## [0.15.0] — 2026-09-11

> **Desktop 阶段启动**：Rust Core 内嵌 Tauri，macOS 桌面客户端全面就绪，同时启动 Windows 支持。这是 agentsync 从 CLI 工具向 Desktop 应用的战略转型里程碑。

### 重大变更 ⚠️

- **彻底移除旧 Go CLI** — 仓库和发布流水线完全收敛为 Desktop-only，Go sidecar 下线，公开 CLI 和文档站同步下线
- **Rust Core 正式内嵌 Tauri** — 工作区快照、Rule 编辑、漂移检测、备份、本机 Rule 导入、项目注册等核心功能不再跨 JSON-lines 进程边界，直接在 Tauri 内运行

### 新增

- **macOS 与 Windows 签名自动更新** — 生产级 macOS ARM64 和 Windows x64 构建，启动时检查 GitHub stable 渠道，支持更新进度显示、版本说明预览、强制确认与 Rule 编辑保护
- **跨项目 Agent 前置治理检查** — `governance scan / init / check / capabilities` 暴露 Baseline/Profile 合约与显式能力矩阵
- **多 Agent Rule 管理工作台** — macOS 客户端可编辑全局与项目母版 Rule，预览所有生产 Agent 适配器，漂移时展示 Diff 并强制选择导入或备份后覆盖
- **本机 Rule 工作区面板** — 原生 Rule 编辑 Sheet，规则命令状态选择器，按项目范围隔离预览
- **macOS 原生应用图标** — 遵循 Apple HIG 设计规范的桌面端图标

### 修复

- **各类 Antigravity 来源分类** — 盘点 Subagents、Hooks、Workflows、Skills 在 Antigravity 各端（桌面 / IDE / CLI）的全局来源，分别展示，不混淆
- **MCP、Rules 来源隔离** — 分别展示 Antigravity IDE 与 CLI 共享配置；区分 Antigravity 双端与 Cursor 全局 Rules 来源
- **canonical 写入原子化** — 修复非原子写入导致的潜在数据竞争问题
- **MCP 盘点解析错误隔离** — 本机 Agent MCP 来源解析出错时，按来源单独上报，不阻塞整体快照

---

## [0.14.0] — 2026-08-01

> CLI 阶段最后一个大版本。代号 **"诊断体验重构"** —— CLI 输出格式全面升级，为 Desktop 阶段奠定人机交互基线。

### 变更

- **全面引入诊断严重级别标签** — 所有诊断输出统一使用 `✗ ERROR` / `⚠ WARN` / `ℹ INFO`（输出到 stderr），成功行以 emoji 开头；`--json` 不受影响
- **`--lossless` 更详细的跳过说明** — 指出被跳过的 bump 未覆盖的 Agent，给出精确排查路径
- **`--lossless` 正确区分"无法评估"与"有损"** — 取包/解析失败的 bump 不再被误报为 lossy
- **`status --agents` 说明已跳过的 Agent 重复检查** — 缩窄报告时，明确说明哪些已启用的 Agent 未经重复检查

### 修复

- **CI 容器化测试回退 Docker** — 当 hosted runner 的 podman/crun 配对损坏时，自动回退至 Docker，避免与代码无关的 infra 原因阻断发布

---

## [0.13.0] — 2026-07-30

> **命名空间 + 孤儿回收**：代号 **"Plugin 命名治理"**，解决多插件同名组件冲突、apply 无法清理孤儿的长期历史债务。

### 重大变更 ⚠️

- **Plugin 组件命名空间化** — Plugin 提供的 subagent/skill/command 现在渲染为 `<plugin>-<name>`（如 `feature-dev-code-reviewer`）；手写组件不受影响；MCP/LSP server 不改名（同 ID 分歧仍拒绝）
- **`subagents/` 目录重命名** — canonical 树中 `agents/*.md` 迁移至 `subagents/`；渲染目标路径不变（仍为 `~/.claude/agents/`），提供 `agentsync migrate subagents` 一键迁移命令，未迁移的树强制拒绝加载
- **多个 CLI 动词整合** — `agentsync plugin install` 改为 `plugin add`；`agentsync secrets` 改为 `secret`（单数）；`agentsync verify` 改为 `check`；`agentsync update` 改为 `plugin outdated/upgrade`

### 新增

- **`apply` 回收孤儿 subagent 和 command** — 从 canonical 删除或重命名后，`apply` 自动删除目标文件（如有手工编辑则先备份），彻底杜绝残留文件被 Agent 误加载
- **`mcp enable / mcp disable`** — MCPServerSpec.enabled 现在可通过命令行控制
- **`secret list / secret remove`** — 不再需要进入 `$EDITOR` 才能查看或删除 key
- **`agentsync explain <path>#<pointer>`** — 溯源一个目标文件的来源 Plugin、适配器转换、所有权状态和漂移类型；仅展示元数据，从不打印 secret 值
- **一次性升级提示** — 每台机器在首次运行新版本时，打印变更说明、需执行的命令和文档链接；stderr 输出，不污染 `--json`
- **`--agents` 统一选择器** — `apply / status / diff / reconcile / revert` 全部接受 `--agents`，保持一致
- **每个 peer 组件均有 `list` 命令** — `skill list / subagent list / command list / hook list / lsp list`

### 修复

- **两个 Plugin 同名组件不再导致 `status` 和 `apply` 崩溃** — 命名空间使常见情况可行；残余碰撞在 `checkProjectedConflicts` 统一上报，不再静默去重
- **`import` 和 `reconcile` 不再把 Plugin 提供的组件回写为 canonical 源** — 防止下次 apply 与 Plugin 自身投影冲突
- **`status / apply / reconcile` 不再因非普通文件（如 FIFO）而永久挂起** — 所有目标读取统一经过形状检查
- **`native_agents = []` 空列表正确持久化** — 改为指针类型，`omitempty` 不再抹除显式空列表

---

## [0.12.0] — 2026-07-29

> **Plugin 去重核心修复** — 修复 `apply` 长期向自管插件的 Agent 双重投影组件的静默 Bug。

### 新增

- **`native_agents` 字段** — `plugins/<id>.toml` 新增字段，声明哪些 Agent 由其自身的插件管理器安装该插件；`apply` 不再向这些 Agent 投影组件；`import` 交互式询问是否延迟

### 修复

- **`apply` 的翻译报告不再多计数** — 延迟插件的组件不再被计入另一个插件的行
- **`status --scope project` 读取项目的插件 pin** — 不再误报项目中不存在的重复
- **`doctor` 不再对未启用的 Agent 发出重复警告**

---

## [0.11.0] — 2026-07-28

> **CLI 全面升级**：代号 **"v1.x 表面锁定"**，完成 #200 设计史诗中 F1/F2/F4~F11 全部子任务。

### 重大变更 ⚠️

- **`--scope / --project` 提升为根 flag** — 原来只有部分命令声明，现在所有命令继承；不支持 scope 的命令明确拒绝并说明原因
- **Plugin 生命周期动词整合到 `plugin` 子组** — `agentsync update` / `agentsync explain` 等顶层命令全部迁移
- **`agentsync verify` 改为 `agentsync check`** — `doctor` 验机器环境，`check` 验配置文件，两者职责明确分离
- **`agentsync plugin install` 改为 `agentsync plugin add`** — 统一所有组的创建动词

### 新增

- **`status --legend`** — 独立打印 9 种漂移分类状态的词汇表
- **`mcp add / remove / list` scope 感知** — 项目 scope 下读写项目树的 `mcp/*.toml`
- **`secret get` 终端输出提示** — 检测到 stdout 是 TTY 时，在 stderr 打印安全提示

### 修复

- **一次性升级提示与 Shell 补全互不干扰** — 补全请求不再触发并消耗升级提示
- **首次升级提示不误触发全新项目 scope 用户** — 触发条件改为 `agentsync.toml` 存在与否
- **`ensureStateGitignore` 改用 `O_APPEND`** — 修复并发 truncate 导致用户自有 gitignore 规则丢失的问题
- **损坏的运行记录不再永久静默升级提示**

---

## [0.10.1] — 2026-07-15

### 修复

- **暂时禁用 Chocolatey pipe** — 等待社区仓库审核期间，CI 跳过 choco 发布步骤，避免阻塞整体发布流水线

---

## [0.10.0] — 2026-07-15

> **项目 scope 精化与 LSP 修复**。

### 变更

- **项目 scope 要求显式声明 Agent** — `--scope project` 下，只渲染项目配置中明确列出的 Agent，不再隐式继承用户全局 Agent 列表

### 修复

- **跳过不支持的 Claude LSP 设置** — 不再因 Claude 不支持的 LSP 字段而报错

---

## [0.9.0] — 2026-06-30

> **输出体验升级**：发布后自动更新 docs、版本指令友好化、状态报告更清晰。

### 新增

- **`agentsync version` 子命令** — `--version` flag 的别名，方便脚本使用
- **`status` 折叠技能目录** — 多技能目录合并显示，`--verbose` 展开，`--agent` 过滤
- **`apply --dry-run` 标注已同步目标** — 已同步的目标文件明确标注，不再与待变更项混淆

### 变更

- **CI 只在 Tag 触发时运行发布** — 合并到 main 不再触发发布流水线，降低误发布风险
- **发布后自动触发 docs 发布** — docs 站随每个 release 自动更新

---

## [0.8.0] — 2026-06-28

> **扩展适配器广度**：代号 **"30+ Agents"**，新增 Cursor、Gemini CLI、Windsurf、Roo Code、Cline 及通用适配器，覆盖市场主流 Agent 工具。

### 新增

- **Cursor 全功能适配器** — MCP、Memory、Skills、Subagents、Commands、Hooks 全覆盖
- **Gemini CLI 全功能适配器** — MCP、Memory、Commands、Subagents、Hooks
- **Windsurf (Cascade) 适配器** — MCP、Memory、Slash Commands
- **Roo Code 适配器** — MCP、Memory、Slash Commands
- **Cline 适配器** — MCP、Memory、Slash Commands
- **通用广度层适配器（22 个 Agent）** — 数据驱动的 generic adapter，一次覆盖 22 款 Agent
- **Agent Skills 在广度层投影** — 18/22 个广度层 Agent 支持 Skills 同步
- **`agentsync explain` 支持多 Plugin、`--all`、`--list`** — 列出跳过组件而非裸计数，输出带样式
- **agentsync managed 内存 banner** — 渲染的内存文件头部自动注入管理声明，防止用户误手工编辑

### 修复

- **`explain` 覆盖范围收窄到指定 Plugin** — 不再对全局 Plugin 联合集做覆盖分析
- **`import claude` 真实世界兼容性修复** — marketplace fetch 错误处理、warning 展示、skill 大小限制
- **Plugin 组件发现扩展到所有规范位置** — 不再遗漏非标准目录下的 Plugin 组件
- **`import` 解析宽松 YAML frontmatter** — 静默丢弃的组件改为明确上报

---

## [0.7.0] — 2026-06-19

> **目标自动 Git 版本化 + `revert` 命令** — 任何 `apply` 写入的目标文件都会被 Git 追踪；引入 `agentsync revert` 支持精确回滚。

### 新增

- **目标文件自动 Git 版本化** — `apply` 写入前自动在目标目录初始化 Git 仓库并 commit，为 `revert` 提供回滚基础
- **`agentsync revert`** — 将一个或全部 Agent 目标文件回滚到上一次 apply 前的状态

---

## [0.7.1] · [0.7.2] · [0.7.3] — 2026-06-19 ~ 2026-06-20 · [0.7.4] — 2026-06-21 · [0.7.5] — 2026-06-28

> **Windows 包管理器发行与构建可重现性修复系列补丁**。

### 修复

- **修复 Chocolatey 跨平台构建不可重现问题（0.7.1 ~ 0.7.3）** — 三项非确定性输入（GOPATH、构建时间戳、文件 mtime）全部钉住到 commit；修复 Windows/Linux 归档文件 Unix mode 差异；GoReleaser 版本锁定为精确版本
- **修复 Scoop token 模板引用错误（0.7.4）**
- **Chocolatey Windows 包正式上线（0.7.5）** — 完成社区仓库审核后重新启用 chocolatey pipe

---

## [0.6.0] — 2026-06-19

> **Windows 分发正式支持** — Scoop 与 Chocolatey 包管理器接入。

### 新增

- **Windows Scoop + Chocolatey 发行渠道** — agentsync 正式支持 Windows 用户通过包管理器安装

---

## [0.5.0] — 2026-06-18

> **安全加固**：插件标签展示边界强制、双向 Unicode 注入防护。

### 新增

- **展示边界强制** — `untrusted.Text` 类型携带插件原始名称，在输出前统一经过 `Sanitize` 处理
- **`apply / verify` 报告中 Plugin 标签脱敏**

### 修复

- **`Sanitize` 剥离双向控制符和零宽字符** — 防止 Plugin 名称通过 bidi/zero-width 字符进行视觉欺骗
- **Plugin 跳过严重级别字段化** — 将 skip 严重级别建模为有类型字段，替换字符串魔法值

---

## [0.4.0] — 2026-06-17

> **`import` 与 `explain` 体验升级**。

### 新增

- **`import` 美化输出** — 分节展示，warning 高亮，结构更清晰
- **`explain` 多 Plugin 支持** — 可同时解释多个 Plugin，支持 `--all` 和 `--list`，带样式输出
- **`status / diff` 彩色输出与 `--json`** — 支持 `--color` flag，`status` 和 `diff` 均有 `--json` 结构化输出

---

## [0.3.0] — 2026-06-16

> **Project Scope 正式发布** — 项目级 `.agentsync/` 配置树支持，`verify` scope 感知。

### 新增

- **项目 scope 支持** — 通过 `.agentsync/` 树为单个仓库单独管理 Agent 配置，与用户全局配置独立
- **`verify --scope project / --project`** — 配置校验支持项目 scope
- **Review Loop Skill** — 内置 `review-loop` 项目级 Claude Code Skill，规范 AI 辅助代码审查流程
- **Codex CLI 全功能适配器** — MCP、Memory、Skills、Subagents、Commands、Hooks 以及插件导入全覆盖

### 修复

- **`apply --scope project` 只写项目级条目** — 修复错误覆盖用户全局配置的问题
- **项目 scope MCP/config 写入正确路径** — 修复 Claude 和 OpenCode 项目级配置读取路径错误

---

## [0.2.0] — 2026-06-15

> **适配器广度扩展第一波** — Continue、Windsurf、Roo、Cline 等适配器奠基；文档站上线。

### 新增

- **Continue 适配器** — MCP、Memory、Slash Commands
- **多款社区 Agent 适配器** — 第一批广度层适配器接入
- **docs.agentsync.cc 文档站** — 通过 GitHub Pages 自动发布，每次 Release 触发更新
- **`import` 捕获已安装插件与 marketplace** — 跨 Agent 全量导入

---

## [0.1.0] — 2026-06-05

> **第一个公开 Beta 版本**。功能端到端可用（`just test-release` 全绿）。

### 核心能力

- **完整的 apply / status / diff / reconcile / import 流水线** — 从 canonical 配置到各 Agent 原生配置的全链路同步
- **Claude Code、OpenCode、Codex 适配器** — 三款主流 Agent 全面支持：MCP、Memory、Skills、Subagents、Commands、Hooks
- **age 加密 Secret 保险库** — 本机凭据加密存储，支持 `${secret:...}` 模板引用
- **Plugin 系统** — 从 marketplace 安装 Plugin，一键同步到所有 Agent
- **Homebrew cask 发行渠道** — macOS 用户可通过 `brew install` 安装
- **Tag 触发发布流水线** — GitHub Actions 自动化 Release + Homebrew 更新

---

*通过 [GitHub Releases](https://github.com/dayney/agent-assistant/releases) 查看完整发布历史。*

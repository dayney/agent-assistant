# macOS 规则工作区实施计划

> **对于代理工人：** 所需的子技能：使用
> `superpowers:subagent-driven-development`（推荐）或
> `superpowers:executing-plans` 逐项实施此计划。步骤使用
> 用于跟踪的复选框 (`- [ ]`) 语法。

**目标：** 将现有的“规则”选项卡转变为 Mac 原生编辑工作区
显式保存、预览和同步阶段，同时保留 agentsync 的
规范源和漂移安全模型。

**架构：** 保留全局配置作为路由所有者并提取
将工作区规则到集中的 React 视图中。纯 TypeScript 选择器自己的命令
启用和预览新鲜度； React 拥有草稿、选择、焦点和
对话框； Tauri 公开原生 macOS 菜单并发出命令事件；走向核心
功能、漂移、差异、备份保持不变且具有权威性，
并写道。

**技术堆栈：** Tauri 2.11.5、React 19、TypeScript 5.8、原生 CSS、Vitest 3.2、
Rust 2021，现有的 Go Core。

**规格：** `docs/superpowers/specs/2026-08-31-rule-management-design.md` 和
`.agentsync/skills/agentsync-macos-app-design/references/design-contract.md`

## 全局约束

- 不添加依赖项、字体、样式系统、编辑器框架、图标包、
  自定义标题栏或窗口状态插件。
- 保留现有的 `1280x820` 默认值和 `960x640` 最小窗口大小。
- 预览和同步绝不能隐式保存规则。
- 任何草稿、范围、项目或目标选择更改都会导致预览无效。
- 备份覆盖是一个单独的两阶段破坏性确认和名称
  每个受影响的目标。
- React 不得推断能力、漂移或写入安全性； Go Core 的回应是
  权威并在写入之前立即重新验证。
- 任何已解决的秘密都不能进入 UI 状态、日志、差异、固定装置或屏幕截图。
- 演示模式保持明确，永远不会掩盖核心故障。

---

### 任务 1：规则命令状态和预览新鲜度

**文件：**
- 修改：`desktop/src/core/rule-state.test.ts`
- 修改：`desktop/src/core/rule-state.ts`

**接口：**
- 产生：`createRulePreviewKey(input: RulePreviewIdentity): string`
- 产生：`deriveRuleCommandState(input: RuleCommandContext): RuleCommandState`
- 生成：`RuleCommandState` 字段 `dirty`、`previewFresh`、`canSave`、
  `canPreview`、`canSync`、`hasBlockedTargets` 和 `primaryAction`

- [ ] **第 1 步：编写显式预览和失效的失败测试**



```ts
it('requires an explicit fresh preview before synchronization', () => {
    const state = deriveRuleCommandState({
        body: '# Rule\n', savedBody: '# Rule\n', selectedAgents: ['codex'],
        scope: 'global', previewKey: null, targets: [safeTarget], busy: false,
    });
    expect(state.canPreview).toBe(true);
    expect(state.canSync).toBe(false);
});

it('invalidates preview when target selection changes', () => {
    const previewKey = createRulePreviewKey({
        scope: 'global', body: '# Rule\n', selectedAgents: ['codex'],
    });
    const state = deriveRuleCommandState({
        body: '# Rule\n', savedBody: '# Rule\n', selectedAgents: ['claude', 'codex'],
        scope: 'global', previewKey, targets: [safeTarget], busy: false,
    });
    expect(state.previewFresh).toBe(false);
    expect(state.canSync).toBe(false);
});
```



- [ ] **第 2 步：运行重点测试并验证缺失的导出是否失败**

运行：`cd desktop && npm test -- src/core/rule-state.test.ts`

预期：失败，因为 `createRulePreviewKey` 和
`deriveRuleCommandState` 不存在。

- [ ] **第 3 步：实现最小的纯选择器**

在形成内存中预览键之前对代理 ID 进行排序。仅启用保存
非空脏草稿，仅预览带有目标的干净保存草稿，以及
仅针对没有阻止目标的显式匹配预览进行同步。

- [ ] **第 4 步：运行重点测试并验证其通过**

运行：`cd desktop && npm test -- src/core/rule-state.test.ts`

预期：通过且无警告。

### 任务 2：Mac 原生规则工作区和可访问的工作表

**文件：**
- 创建：`desktop/src/views/RulesView.tsx`
- 创建：`desktop/src/components/RuleConflictSheet.tsx`
- 修改：`desktop/src/views/GlobalView.tsx`
- 修改：`desktop/src/app/App.tsx`

**接口：**
- `RulesView` 消耗现有的 `GlobalViewProps` 形状。
- 规则工作区仅在 `getRules` 完成后存储预览键
  显式预览命令。
- 它监听 `app-menu-command` 值 `rule.save`、`rule.preview`、
  `rule.sync`、`rule.import-native` 和 `view.toggle-inspector`。

- [ ] **步骤 1：提取规则编排而不改变行为**

将规则编辑器从 `GlobalView.tsx` 移至 `RulesView.tsx`；保持全球
选项卡和非规则面板不变。运行现有的前端测试并构建。

- [ ] **第 2 步：将命令状态连接到可见控件**

删除两个隐式 `saveMother()` 调用。 Save 坚持规范规则并且
清除预览新鲜度；仅预览调用 `getRules`；仅同步通话
`syncRules` 并需要 `canSync`。单一强调的动作从
脏时保存以在全新安全预览后同步。

- [ ] **第 3 步：添加脏范围更改和冲突表**

使用本机 HTML `dialog.showModal()` 行为进行焦点遏制和转义。
范围/项目更改提供保存、放弃和取消。冲突审核优惠
导入原生或继续备份；备份和覆盖仅出现在
第二次确认列出了所有被阻止的选定目的地。

- [ ] **第 4 步：添加检查器选择和命令事件行为**

保持目标 Diff 文本可选择，之后将焦点恢复到调用行
工作表关闭，并通过 `role=status` / `role=alert` 更新状态。支持
浏览器预览中的标准 `Cmd+S` 和本机菜单中的相同处理程序。

- [ ] **第 5 步：运行前端测试和类型/构建验证**

运行：`cd desktop && npm test`

运行：`cd desktop && npm run build`

预期：所有测试均通过且 TypeScript/Vite 构建退出 0。

### 任务 3：本机 macOS 命令菜单

**文件：**
- 修改：`desktop/src-tauri/src/lib.rs`

**接口：**
- 添加 Tauri 命令 `set_rule_menu_state`，其中包含用于活动、保存的布尔值，
  预览、同步、导入本机和检查器检查状态。
- 使用自定义菜单项 ID 发出 `app-menu-command`。

- [ ] **第 1 步：为自定义命令白名单编写失败的 Rust 测试**



```rust
#[test]
fn recognizes_only_rule_workspace_menu_commands() {
    assert!(is_rule_workspace_menu_command("rule.save"));
    assert!(is_rule_workspace_menu_command("view.toggle-inspector"));
    assert!(!is_rule_workspace_menu_command("edit.copy"));
}
```



- [ ] **第 2 步：运行 Rust 测试并验证缺失的函数是否失败**

运行：`cd desktop/src-tauri && cargo test recognizes_only_rule_workspace_menu_commands`

预期：编译失败，因为缺少 `is_rule_workspace_menu_command`。

- [ ] **第 3 步：构建标准应用程序、文件、编辑、规则、视图和窗口菜单**

仅使用 Tauri 2.11.5 `MenuBuilder`、`SubmenuBuilder`、预定义编辑/窗口
项目和自定义 `MenuItemBuilder` 项目。文件 > 保存规则使用 `CmdOrCtrl+S`；
产品特定命令没有快捷方式。保持菜单启用同步
通过 `set_rule_menu_state` 使用 React 选择器。

- [ ] **第 4 步：仅发出列入许可名单的自定义命令 ID**

使用 `tauri::Emitter` 发出 `app-menu-command`；预定义的菜单项保留
它们的本机行为不会转发到 React。

- [ ] **第 5 步：格式化并验证 Rust**

运行：`cd desktop/src-tauri && cargo fmt --check`

运行：`cd desktop/src-tauri && cargo test`

运行：`cd desktop/src-tauri && cargo check`

预期：所有命令退出 0。

### 任务 4：语义桌面视觉系统和文档

**文件：**
- 修改：`desktop/src/app.css`
- 修改：`docs/superpowers/specs/2026-08-31-rule-management-design.md`
- 修改：`CHANGELOG.md`

**接口：**
- CSS 保留现有非规则视图使用的类契约。
- 规则布局在 `1280x820`、`960x640` 和更宽的窗口中仍然可用。

- [ ] **第 1 步：用语义外观角色替换固定的仪表板样式**

将系统感知的令牌用于窗口、侧边栏、内容、检查器、分隔符、
文本、重音/焦点、成功、警告、破坏性和差异状态。去除假货
搜索/帐户镶边、嵌套规则卡、装饰阴影和无法访问
低于 560 像素的桌面规则。不要选择字体系列。

- [ ] **第 2 步：添加外观和辅助功能媒体行为**

实现深色外观、增加对比度、降低透明度、减少
运动、稳定的对焦环和非颜色状态提示。保持尺寸稳定
通过配置的最小窗口大小。

- [ ] **第 3 步：更新设计合同和变更日志**

文档显式保存->预览->同步，原生菜单，单
窗口/编辑器/检查器层次结构和两阶段备份覆盖行为。

- [ ] **第 4 步：运行全面验证**

运行：`cd desktop && npm test && npm run build`

运行：`cd desktop/src-tauri && cargo fmt --check && cargo test && cargo check`

在存储库所需的容器中运行受影响的 Go 测试命令。
启动演示开发服务器并检查 `1280x820`、`960x640` 处的屏幕截图，以及
宽阔的视口；验证亮/暗外观、键盘焦点、对话框、阻止
流动，无重叠。
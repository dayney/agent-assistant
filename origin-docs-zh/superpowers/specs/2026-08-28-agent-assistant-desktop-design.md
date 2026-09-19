# 代理助手桌面设计

## 目标

交付一款 macOS 优先的 Tauri 2 桌面客户端，品牌为 `agent-assistant`
通过以下方式可视化并安全管理现有的 Agentsync 治理模型
全局、代理和项目维度。

## 产品型号

客户端显示一个规范源及其派生的投影，而不是
每个代理重复配置。有效值在此得到解决
订单：



```text
Baseline < Global source < Agent override < Project Profile < runtime resolution
```



本机代理文件是渲染输出。客户必须证明每件物品的出处
值、适配器功能（`full`、`partial` 或 `unsupported`）以及
写入前比较。所需的不受支持的组件会停止操作并
需要明确的批准决定。

全局配置包含共享规则、MCP、技能、工作流程、Hooks 和
子代理。代理视图比较本机功能并显示特定于代理的功能
投影。项目视图绑定本地配置文件、文档根、堆栈、
验证命令、启用的代理和本地覆盖。项目及代理
是相同有效配置上的独立导航维度。

秘密永远不会放置在 UI 状态或普通配置 JSON 中。用户界面显示
`${secret:...}` 或 `${env:...}` 参考； macOS 钥匙串拥有解析值。
新作继承当前系统字体，可能不会添加字体包，
`next/font`、`@font-face`、远程字体 URL、命名系列或任意字体
选择器。

## 架构



```text
Tauri 2 shell (Rust)
    ├── macOS window, Keychain boundary, file watch, IPC
    ├── React + TypeScript UI
    └── Go Agentsync Core sidecar
          ├── Baseline / Profiles / governance
          ├── source / project overlays
          ├── Agent adapters and capability report
          ├── preview / apply / drift / reconcile
          └── JSON output contract
```



第一个垂直切片使用类型化的 `CoreClient` 接口。默认金牛座
运行时通过 Tauri IPC 调用 Go Core； Go sidecar 读取本地
通过 JSON 行规范/项目树和编辑本机代理库存
合同。显式演示客户端仅可用于视觉审查；
演示模式在 UI 中可见，并且不是静默的后备模式。

## 信息架构

shell 具有 `总览`、`全局配置`、`Agent`、`项目` 和 `活动记录` 导航。
每个详细信息视图都支持 `有效配置`、`配置源`、`原生投影` 和 `差异` 选项卡。
概述显示运行状况计数、功能警告和来源链。
代理页面是一个组件矩阵。项目页面是配置文件注册表
具有堆栈和验证状态。应用始终是预览→查看警告→
明确批准→写入→结果。

## 验收标准

- `npm run build` 为桌面前端生成一个 Vite 包。
- `cargo check` 验证 Tauri shell。
- 所有主要控件都是语义按钮或具有可见的导航元素
  焦点状态和可访问的名称。
- 没有加载或明确选择新的字体系列。
- UI 重排，桌面上没有重叠且宽度紧凑。
- 演示数据被标记为演示数据；核心错误可见且不落下
  默默地回来。
- 实模式报告 `schemaVersion: 1` 并发现下面的一级
  `$HOME/git/work`（可使用 `AGENT_ASSISTANT_PROJECTS_ROOT` 覆盖）。本地人
  库存是只读证据，直到应用状态和漂移预测
  连接。
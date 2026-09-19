# 代理同步文档

> 🌐 **更喜欢带有搜索功能的可浏览网站？** 这些文档已发布 — 已扩展，
> 具有全文搜索和渲染图表 — 位于 **[agentsync.cc](https://agentsync.cc)**。
> 来源位于 [`../website/`](../website/);下面的四个合同文件
>（概念、架构、组件、能力矩阵）都镜像在那里
> 在构建时逐字记录，因此该目录仍然是事实来源。

从这里开始。这些文档是分层的——从上到下阅读，从零到流利，
或跳转到您需要的任何内容。

|文档 |当您想要...时请阅读它观众|
|---|---|---|
| **[用户指南](user-guide.md)** |安装agentsync并从0→100：第一次同步、每日循环、秘密、插件、项目配置。 |用户 |
| **[概念和术语](concepts.md)** |了解三态模型、漂移、协调以及一页中的每个术语。 |大家 |
| **[能力矩阵](capability-matrix.md)** |准确了解每个代理支持什么以及有损或延迟的内容。 |用户·贡献者|
| **[agentsync 的比较](comparison.md)** |了解 agentsync 如何与 gaal、rulesync、agentsmesh 以及其他领域相比较。 |用户·评估者|
| **[架构](architecture.md)** |了解应用/捕获管道、漂移分类器和秘密不变量如何工作。 |贡献者 |
| **[组件图](components.md)** |按包导航代码库包。 |贡献者 |

**回购根文档：**

- **[README](../README.md)** — 快速入门、安装、已知限制、完整的环境变量表。
- **[CONTRIBUTING](../CONTRIBUTING.md)** — 如何构建、测试和提交更改。
- **[SECURITY](../SECURITY.md)** — 威胁模型和漏洞报告。
- **[CLAUDE.md](../CLAUDE.md)** — 用于处理代码的 AI 代理的项目内存。
- **[CHANGELOG](../CHANGELOG.md)** — 更改内容，逐个版本。

**建议阅读顺序**

1. 新用户 → [用户指南](user-guide.md)（当不熟悉的术语时略读[概念](concepts.md)）。
2. 评估适合度 → [能力矩阵](capability-matrix.md) 和 [agentsync 比较方式](comparison.md)。
3. 贡献 → [概念](concepts.md) → [架构](architecture.md) → [组件图](components.md) → [贡献](../CONTRIBUTING.md)。

---

### 内部设计历史

`docs/superpowers/`（原始 v1.0 设计规范和里程碑计划）和
`docs/decisions/`（架构决策记录）被保存以供参考。的
上面的文档将取代它们用于日常使用；该规范仍然具有权威性
*为什么* v1.0 的记录是这样形成的。
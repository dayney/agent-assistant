# agentsync 的比较方式

agentsync 在 AI 编码代理配置领域中的地位 — 它是什么
与该领域的其他人分享，以及少数真正属于自己的东西。

这是诚实定位页面。 agentsync **不是**唯一的工具
将一个配置渲染到多个代理中；那个球场很拥挤。难得的是
*组合*它提供：一个单二进制 Go CLI，**双向**（本机
编辑被捕获回来，而不仅仅是向外生成）**并且**带有
**年龄加密的秘密保险库**具有参考分辨率。我们没有竞争对手
找到了所有这三个配对。

:::注意[时间点]
形势变化很快——这里的大多数工具都是在几个月内推出的，
明星数量/经纪人名单每周都会发生变化。下面的数字是 **2026 年中期的数据
快照** 均为近似值；将它们视为“数量级”，而不是福音。
欢迎指正。
:::

## 重要的四个轴

在比较这些工具时，有四个问题将它们分开：

1. **多代理** — 它是针对多个代理，还是仅在两个代理之间进行端口？
2. **双向** - 它是否检测本机文件中的*漂移*并协调编辑
   回到规范源，或者它是一个单向生成器？
3. **组件覆盖** — 它是否管理整个表面（内存、技能、MCP、
   子代理、命令、挂钩），还是只是规则/技能？
4. **秘密**——它会解决并保护秘密，还是把它们留给你？

几乎每个人都会做#1，并且至少做一部分#3。 **#2和#4是该字段所在的位置
稀疏**，也是agentsync 集中的地方。

## 比较矩阵

图例： ✅ 完整 · ◐ 部分/实验 · ❌ 无。组件缩写
**记忆**理论·**技能**毛病·**MCP**·**子**代理·**Cmd**·**钩子**。对于
agentsync 的每个代理的准确保真度（本机、预计、跳过），请参阅
[能力矩阵](capability-matrix.md) — 下面的 ✅ 是一个总结，而不是一个
保真度主张。

|工具|郎 |代理|内存 | SK | MCP|子|命令 |挂钩|双向/漂移|秘密 |
|---|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|---|---|
| ⭐️ **agentsync** ⭐️ *（此工具）* | **去** | **31**[^宽度] | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ **三态分类器 + `reconcile`/`import` 捕获** | ✅ **年龄库，`${secret:}`/`${env:}`，重新引用 + 泄漏保障** |
| [代理网格](https://github.com/sampleXbro/agentsmesh) | TS/Py | 30+ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ `generate`/`import`/`check`（CI 中的锁定文件漂移）| ❌（以您的商店为准）|
| [规则同步](https://github.com/dyoshikawa/rulesync) | TS | 25+ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ◐ `generate` + `import`（一次性摄取，无状态模型）| ❌ |
| [加尔](https://github.com/getgaal/gaal) | **去** | 17-20 | 17-20 ◐ 文件 | ✅ | ✅ | ❌ | ◐ 文件 | ✅ | ❌ 单向（`--prune`、`init --import-all` 引导）| ❌ |
| [标尺](https://github.com/intellectronica/ruler) | TS | 32 | 32 ✅ | ◐ | ✅ | ◐ | ❌ | ❌ | ❌ 单向（+ 来自备份的 `revert`）| ❌ |
| [ai-rulez](https://github.com/Goldziher/ai-rulez) | **去** | 19+ | ✅ | ✅ | ◐ | ✅ | ✅ | ◐ | ❌ 单向（+ 预提交强制/验证）| ❌ |
| [amtiYo/代理](https://github.com/amtiYo/agents) | TS | 11 | 11 ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ◐ 单向 + `sync --check` 漂移 | ◐ 占位符分割（gitignored 明文）|
| [口径/ai-设置](https://github.com/caliber-ai-org/ai-setup) |节点| 5 | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ 单向生成（LLM-“定制”，`undo`）| ◐ 仅限提供商密钥 `0600` |
| [ai-config-sync-manager](https://github.com/slash9494/ai-config-sync-manager) |节点| 2 (CC↔法典) | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ 真正的双向主机感知翻译+回滚 | ◐ 携带MCP不记名代币 |
| [nicepkg/垂直同步](https://github.com/nicepkg/vsync) | TS | 4 | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ | ◐ 来自“真相来源”工具的单向（+导入）| ◐ 重写 ref 语法，从不扩展 |
| [mcpup](https://github.com/mohammedsamin/mcpup) | **去** | 13 | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ◐ 每个客户端启用/禁用 + `doctor` 漂移 | ◐ 普通环境 |
| [技能分享](https://skillshare.runkids.cc) | **去** | 60+ | ◐ 文件 | ✅ | ❌ | ✅ | ◐ 文件 | ❌ | ❌ 单向（符号链接/复制；`commit` 检查点）| ❌（仅限技能安全审核）|

Error 500 (Server Error)!!1500.That’s an error.There was an error. Please try again later.That’s all we know.

- **完全双向/多组件同步**（agentsync 的真正对等体）：
  **agentsmesh** （最接近架构 - 一个规范的目录
  `generate`/`import`/`check`，框架为“`package.json` 生成
  `package-lock.json`”；TypeScript，无秘密库），**rulesync**（最
  流行度~1.1k★；广泛的组件，`generate` + `import`，但没有漂移状态
  模型或秘密）和 **ai-config-sync-manager** （真正的双向，但仅
  克劳德代码 ↔ 法典）。
- **单向生成器** — **gaal** （Gosibling；“一个 YAML，每个代理，每个
  机，”添加了多协议存储库克隆，但没有漂移，也没有秘密），
  **ruler**（~2.7k★，规则 + MCP），**ai-rulez**（Go，广泛的组件），
  **口径/人工智能设置**。这些向外渲染并停止。
- **一次性搬运工** — **[cc2codex](https://github.com/ussumant/cc2codex)**（
  Claude Code → Codex 迁移插件；一次性、单向、编辑秘密
  而不是移动它们）。与持久规范相反的设计点
  来源。
- **技能/命令工具** — `vercel-labs/skills`、`skillshare`、`skillkit`、
  和朋友。狭窄但人流量大；与原生越来越多余
  跨工具技能加载。
- **仅限 MCP 的管理器** — **mcpuup** （Go，涵盖了大部分 agentsync
  Claude Code + OpenCode + Codex + Cursor + Gemini + Continue 设置，但仅限 MCP），
  [vek-sync](https://github.com/Vektor-Memory/vek-sync)（AES 加密的 MCP 保管库
  与 `export`/`diff`/`sync`)、[mcpm](https://github.com/pathintegral-institute/mcpm.sh)。
  与作为服务器运行的 MCP *网关*（MetaMCP、Docker MCP 网关）不同
  在请求路径中而不是编写本机配置。

### 关于 AGENTS.md 标准的说明

[AGENTS.md](https://agents.md/) (~22k★，现在在 Linux 下托管
Foundation 的 Agentic AI Foundation）**不是竞争对手 - 它是底层**。
agentsync 为 Codex/OpenCode 渲染内存，就像本文中的其他人一样
列表。这些工具存在的原因是 Claude Code 的 `CLAUDE.md`，
技能、挂钩和子代理位于 AGENTS.md 之外，因此单个 Markdown
标准并不能解决扇出问题。

## 来源

主要来源（存储库/项目站点），2026 年中期验证：
[加尔](https://github.com/getgaal/gaal),
[代理网格](https://github.com/sampleXbro/agentsmesh),
[规则同步](https://github.com/dyoshikawa/rulesync),
[标尺](https://github.com/intellectronica/ruler),
[ai-rulez](https://github.com/Goldziher/ai-rulez),
[amtiYo/代理](https://github.com/amtiYo/agents),
[口径/ai-设置](https://github.com/caliber-ai-org/ai-setup),
[ai-config-sync-manager](https://github.com/slash9494/ai-config-sync-manager),
[nicepkg/垂直同步](https://github.com/nicepkg/vsync),
[mcpup](https://github.com/mohammedsamin/mcpup),
[vek-sync](https://github.com/Vektor-Memory/vek-sync),
[cc2codex](https://github.com/ussumant/cc2codex),
[vercel-labs/技能](https://github.com/vercel-labs/skills),
[技能分享](https://github.com/runkids/skillshare),
[AGENTS.md](https://agents.md/)。

[^breadth]：以与该字段的其他大数字相同的方式进行计数 - 每个
    读取配置文件的代理。在 agentsync 中，**9 是深度适配器**
    （多组件、双向投影 — Claude Code、OpenCode、Codex、
    Cursor、Gemini CLI、Continue、Windsurf、Roo Code、Cline）以及其余的
    数据驱动**广度层**（内存+同形状MCP+代理技能）。这个广度/深度
    分裂在该领域很常见：agentmesh 就是其中之一，几乎涵盖了它的所有功能
    约 30 个具有记忆 + 技能但完整的多组件表面的特工
    （子代理、命令、挂钩）仅~9。但这并不具有普遍性——
    Rulesync 在广泛的集合中构建真正的每工具适配器（覆盖范围
    沿着长尾变薄），所以大计数“本身”并不能告诉你
    广度与深度。
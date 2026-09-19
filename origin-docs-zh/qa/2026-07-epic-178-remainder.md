# QA 剩余部分 — 史诗 #178，问题 #164（真实安全带验证）

诚实记录第 164 个问题——史诗的现实世界 QA 保护伞——
需求、实际交付的内容（回购内、可验证）以及剩余的内容
真正未执行，因此 v1.0 标签决策可以明确权衡差距
而不是从问题的关闭状态推断完整性。

## #164 的要求是什么

- 针对边缘情况存储库（packed/gc'd）练习 **git-backup / revert**
  历史、权限漂移、嵌套的外部存储库、模式剥离档案）。
- **推出七个未经测试的特工安全带**（windsurf、roo、cline、继续、
  双子座、光标、法典 — 超出往返测试的）
  agentsync-rendered 配置并记录每个代理 F5 重新读取的判决。
- 测量 `HasNestedRepoBelow` 全树行走的**挂钟成本**
  大型 IDE 目录。
- 为问题附上**书面质量检查报告**。

## 仓库中交付的内容（可在 HEAD 验证）

回归装置替代 git-backup/revert 边缘情况，全部
标准套件中的容器运行（`internal/git`、`internal/cli`）：

- `TestRestore_PreservesUntrackedFiles`、`TestRevert_PreservesUntrackedUserFiles`
  —#128 未追踪生存合约。
- `TestApply_GitBackupBaselineRevertsFirstApply`,
  `TestApply_GitBackupBaselineCoversOrphanDeletes` — 首先应用并
  仅删除应用可恢复性（本修复中添加了后者
  后续行动，缩小计划删除基线差距）。
- `TestRevertRootSkipsForeignNestedRepo`、`TestHasNestedRepoBelowFollowsSymlink`
  — 嵌套/符号链接的外国回购安全。
- `TestGitPermLifecycle` — `.git` 0700 权限生命周期。
- `TestRestore_PackedHistory` — 跨完全重新打包的基于增量的恢复
  对象存储（零松散对象；非真空断言），打包/GC'd
  历史案例#164 命名。
- `TestRevert_Errors`' 祖先验证案例 - `revert --to` 现在拒绝
  检查点历史记录之外的散列（此补救措施后续行动；以前是
  跳过记录不安全行为的测试）。
- `BenchmarkHasNestedRepoBelow` — 现在测量步行成本（大约 10 毫秒
  深度为 4 的 2,000-leaf-dir 树，在容器内）并记录在函数的
  文档评论。仅限集装箱内号码；没有对真实的物体进行测量
  用户计算机针对真实的 IDE 目录。

## 真正未执行的内容

- **从未启动过带电线束。** 密封测试容器无法运行
  代理应用程序，因此没有代理重新读取代理同步渲染的配置
  这部史诗；保真度证据仅是基于工件的往返测试。的
  留下的一个开放的上游问题在代码中被跟踪为
  `TODO(#164)`（光标远程-`type`拒绝-vs-忽略，`internal/adapter/cursor/mcp.go`）。
- 对于该问题指定的七个代理，**不存在 F5 重读裁决**。
- **真实 IDE 目录上的挂钟成本**（例如填充的 `~/.config/zed`）是
  未测量——仅是上面的综合容器内基准。

运行 live-harness pass 需要一个安装了代理的工作站；
它被替换，而不是默默地跳过。如果 v1.0 发行时没有它，则该文件是
该决定的记录。
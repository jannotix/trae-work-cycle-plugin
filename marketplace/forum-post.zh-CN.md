# [Skill 分享] Cycle for Trae Work — 给交付加上证据门禁：五个隔离角色、盲审、仲裁与字节级交付

## 简介

Cycle for Trae Work 是一个本地运行的交付治理插件：它不替代 TRAE 的规划、浏览或评审能力，而是在"改完了"和"真的交付了"之间加一道证据门禁。开源（FSL-1.1-MIT，两年后转 MIT），完全本地运行，无云端、无账号、无遥测。

一次对话里，AI 既是需求的解读者，又是代码的编写者，还是自己工作的验收者——未完成的层级就这样被藏了起来：后端写完了 UI 没接、迁移没跑、测试全在 mock 上通过。Cycle 把原始请求冻结为不可变契约，把执行和审批分开，在交付前留下可检查的证据。

## 解决什么问题

我自己带项目时最怕的不是 AI 写不出代码，而是它声称"完成了"。Cycle 的答案是五个相互隔离的角色：

1. **架构师（Architect）** — 把原始请求拆成有验收标准的任务 DAG；
2. **执行者（Executor）** — 在受管 worktree 内一次只做一件授权范围内的事（就是 TRAE 会话本身，模型不变）；
3. **功能评审（Functional reviewer）** — 盲审：只看冻结的候选、原始请求和原始证据；
4. **安全与架构评审（Security reviewer）** — 同样盲审，两份评审互不可见；
5. **仲裁者（Arbiter）** — 拿着用户的**原始请求**、候选摘要、证据和两份评审做最终裁决。执行者永远不能批准自己的工作。

关键机制：

- **不可变请求**：用户消息逐字进入 `cycle_start`，SHA-256 绑定后续每个环节；
- **候选冻结**：Git worktree 内的精确字节、diff、文件模式全部冻结，冻结后任何改动都会让证据失效；
- **真实验证**：跑的是项目自己的命令（测试、构建、浏览器检查），不是模型的口头总结；
- **字节级交付**：仲裁批准后，`cycle_promote` 把冻结的精确字节写回源仓库；
- **有界返修**：最多 5 轮返修，第 5 次拒绝后阻塞等待用户决策；基础设施故障不消耗返修次数；
- **审计链**：每一步进哈希链账本，`/cycle history verify` 本地验签。

模型完全自选：四个只读角色各自配置任意 OpenAI 兼容端点（云端或本地 Ollama/LM Studio 都行），执行者用你在 TRAE 里选的模型，插件永不读取 TRAE 的密钥。

## 安装（5 步，约 5 分钟）

1. 从 [GitHub Releases](https://github.com/jannotix/trae-work-cycle-plugin/releases) 下载对应平台压缩包，解压 `trae-cycle` 到固定目录；
2. TRAE 设置 → MCP：按仓库里的 `plugin/install/mcp.example.json` 注册（本地环境）；
3. 把 `cycle-delivery` 技能解压到全局技能目录（Windows: `%USERPROFILE%\.trae-cn\skills\`）；
4. 设置 → 命令：按 `plugin/command/cycle.md` 创建 `cycle` 命令；
5. 创建 `%LOCALAPPDATA%\Trae Cycle\config\roles.json` 配置四个角色模型，然后：

```
/cycle setup
/cycle doctor
```

## 使用示例

```
/cycle run quick
给登录接口加上速率限制
```

之后的流程：架构计划 → 受管 worktree 内实现 → 真实命令验证 → 候选冻结 → 仲裁裁决 → 字节级交付。full 模式在这之上加入双盲评审。也可以单独咨询某个角色：

```
/cycle architect  拆这个服务值得吗？
```

## 效果数据（认证报告）

- Windows x64 全流程认证：quick / full 自主循环、返修路径、36 个工具全量、多项目并发全部通过；
- 50 万文件仓库索引：0 解析错误，图 217 万节点 / 170 万边，增量刷新 35 秒，峰值内存 308 MB；
- CI 双平台（Windows/Linux）fmt + clippy -D warnings + 279 个测试全绿；
- Release 产物带 SHA256SUMS、CycloneDX SBOM、来源清单与构建来源证明（attestation）。

## 链接

- 仓库与文档：https://github.com/jannotix/trae-work-cycle-plugin
- 用户手册 / 命令参考 / 威胁模型 / 认证报告：仓库 `documentation/` 目录

## 总结与思考

做这个插件的出发点很简单：AI 编码的瓶颈已经从"写得快"变成了"验收可信"。Cycle 没有发明新流程，只是把软件工程里最老的那条规则搬了进来——谁实现，谁就不能批准。欢迎体验和提 issue。

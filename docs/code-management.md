# 项目代码管理安全手册（Code Management）

> 目的：安全、可追溯地管理所有代码提交与分支，降低泄露风险并保持历史清晰。

## 0. 核心原则

- 任何特性/改动都优先在 `feat/*` 或 `fix/*` 分支完成，最终合并到 `main`。
- 只要涉及密钥、令牌、私密配置，就必须从源代码中剔除，并确保不会被提交进 git 历史。
- 提交作者邮箱使用 GitHub noreply，避免 GH007 这类邮箱隐私拒绝。
- 默认使用 PR 合并（合并策略按团队偏好：如 `squash` 或 merge commit）。

## 1. 分支约定

- `main`：稳定分支，只用于发布/稳定整合。
- `feat/xxx`：新功能分支，命名示例：`feat/pet-interactions`、`feat/new-skin`。
- `fix/xxx`：修复分支，命名示例：`fix/hook-crash`。

## 2. 默认工作流（新增功能/改动）

1. 在本地更新 `main`。
   - `git checkout main`
   - `git pull origin main`
2. 基于最新 `main` 开新分支。
   - `git checkout -b feat/xxx`
3. 开发并提交多次小提交（建议每个提交尽量围绕单一目的）。
   - `git add -A && git commit -m "feat: xxx"`
4. 推送到远程（首次推送需 `-u` 建立上游）。
   - `git push -u origin feat/xxx`
5. 在 GitHub 创建 PR，通知需要的评审/自检。
6. 合并到 `main` 后，删除无用分支（本地用 `-d`，远程走 GitHub 或 `git push origin --delete`）。

## 3. 默认工作流（修复/Hotfix）

1. 从最新 `main` 拉出 `fix/xxx`。
2. 快速修复并验证（尽量覆盖最关键的场景）。
3. 推送分支并创建 PR。
4. 合并到 `main`，必要时补充回归测试或文档说明。

## 4. 提交信息规范（Commit Message）

- 格式建议：`type(scope?): message`
- `type` 常用值：`feat`、`fix`、`docs`、`chore`、`refactor`、`test`
- `scope` 可选：例如 `hook`、`pet`、`state`、`build`
- `message`：使用中文或英文均可，但建议简短可读，尽量说明“为什么/影响是什么”，不要只写“更新代码”。

示例：
- `feat(hook): integrate cursor hooks and state protocol`
- `fix: prevent private email push rejection by using noreply`
- `docs: add code management workflow`

## 5. PR 合并策略建议

- 推荐 PR 合并后保持历史清晰：常用做法是对 PR 做 `squash merge`（单个 PR 对应一次干净的提交）。
- 如果你希望保留细粒度提交历史，也可以选择 merge commit；但请确保团队知晓并保持一致。

## 6. 冲突处理

- 合并前先拉取最新：`git pull origin main`
- 冲突解决后，确认运行关键路径（至少 `cargo test` 或 `cargo run` 的核心二进制）
- 解决冲突的提交 message 建议包含来源与目的。

## 7. 安全与隐私（强制）

### 7.1 禁止提交

- `.env`、`*.pem`、`*.key`、`*.crt`、`secrets/` 等任何密钥/证书/令牌。
- 任何包含账号密码、access token、私有配置的文件。
- 编译产物（如 `target/`、`dist/`、`build/`、`*.o` 等）。

### 7.2 强制检查（推送前）

- 推送前先确认作者邮箱与变更内容：
  - `git log -1 --format='%an <%ae>'`
  - `git status`
  - `git diff --stat`
- 如果看到提交作者邮箱不是 noreply，优先改写本地作者并 `--amend`，再推送。

### 7.3 `.gitignore` 策略

- 需要写入仓库的“资产”才允许放入（例如 HTML 内嵌内容）。
- 不确定是否敏感的文件，先加入 `.gitignore` 并在本地验证不再误提交。

## 8. 版本与发布（可选）

- 发布前建议：
  - 更新 `CHANGELOG.md`（写清楚用户可见变更）
  - 在 `main` 上打 tag（如 `v1.0.0`）
- 不要在发布分支上随意叠加未完成功能；必要时拆分为 PR。

## 9. 文档与可追溯性

- 与开发无关但“会影响运行/使用”的变更，必须补到：
  - `README.md`
  - `docs/` 下对应说明文件
  - 或 `scripts/` 的使用说明

## 10. 建议的本地自检命令（按需要选用）

- Rust：
  - `cargo test`
  - `cargo run -p nixie-pet`
  - `cargo run -p nixie-hook`
- Git：
  - `git status`
  - `git log --oneline --decorate -n 10`
  - `git branch -vv`


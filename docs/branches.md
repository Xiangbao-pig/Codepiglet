# 分支管理约定

本地和远程都使用 Git 分支来管理代码，建议按下面方式操作。

更完整的提交/推送/安全检查规则见 `docs/code-management.md`。

## 分支角色

| 分支 | 用途 |
|------|------|
| **main** | 稳定/发布分支，与远程 `origin/main` 同步，只合并已完成的特性或修复。 |
| **feat/xxx** | 功能分支，如 `feat/pet-interactions`、`feat/new-skin`，在分支上开发，完成后合并回 `main`。 |
| **fix/xxx** | 修复分支（可选），用于 bug 修复，合并回 `main`。 |

## 日常流程

### 1. 新功能开发

```bash
# 从最新的 main 拉出新分支
git checkout main
git pull origin main
git checkout -b feat/你的功能名

# 开发、提交...
git add -A && git commit -m "feat: 简短描述"

# 推送到远程（首次）
git push -u origin feat/你的功能名
```

### 2. 功能完成后合并到 main

```bash
# 在功能分支上确保已提交
git status

# 切回 main 并更新
git checkout main
git pull origin main

# 合并功能分支（无冲突时会是 fast-forward）
git merge feat/你的功能名 -m "Merge feat/你的功能名"

# 推送到远程
git push origin main
```

### 3. 查看当前分支与远程对应关系

```bash
git branch -a
git status   # 会显示与 upstream 的领先/落后
```

## 远程仓库

- **GitHub**: https://github.com/Xiangbao-pig/Codepiglet
- **main**：默认分支，与本地 `main` 对应。
- **feat/xxx**：可按需推送，用于 PR 或备份。

## 建议

- 在 `main` 上不要直接开发，始终在 `feat/xxx` 或 `fix/xxx` 上改完再合并。
- 合并前先 `git pull origin main`，减少冲突。
- 长期不用的本地分支可删除：`git branch -d feat/旧分支`；远程分支在 GitHub 上删除或 `git push origin --delete feat/旧分支`。

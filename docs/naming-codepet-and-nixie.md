# 命名对照：Codepet 与 Nixie（留痕）

避免以后改 UI / 打包 / Hook 时对不上号。原则：**用户-facing 用 Codepet；工程与数据路径大量仍叫 nixie，属历史与兼容性，不必强行统一成同一字符串。**

## 用户可见（产品名）

| 场景 | 当前约定 |
|------|-----------|
| 窗口标题 | `Codepet`（`nixie-pet/src/main.rs` → `WindowBuilder::with_title`） |
| 关于页标题等 | `Codepet` / `Codepet 小猪`（`nyanpig-i18n.js`、`nyanpig-body.html`） |
| macOS 程序坞 / 左上角应用菜单显示名 | `Info.plist` 的 `CFBundleName`、`CFBundleDisplayName` → **Codepet** |
| 版权串 | `NSHumanReadableCopyright` → `Copyright © Codepet` |

## macOS 应用包（`scripts/bundle-nixie-macos-app.sh`）

| 项 | 值 |
|----|-----|
| 产物目录名 | `dist/Codepet.app` |
| Bundle Identifier | `dev.codepet.pet` |
| 包内可执行文件名 | `Contents/MacOS/codepet`（由 `target/release/nixie-pet` **复制**而来，**不**改 Cargo 产物名） |
| `CFBundleExecutable` | `codepet`（须与上一行文件名一致） |
| 图标 | `Contents/Resources/AppIcon.icns` |

换 Bundle ID 后，系统在「登录项 / 防火墙 / 自动化」里会视为**新应用**；与旧包并存时注意清理旧条目。

## Rust / Cargo（仓库与二进制）

| 项 | 值 |
|----|-----|
| 宠物 crate 名 | `nixie-pet`（`cargo build -p nixie-pet`、`cargo run -p nixie-pet`） |
| 宠物 release 二进制 | `target/release/nixie-pet` |
| Hook crate / 二进制 | `nixie-hook` → `target/release/nixie-hook`，安装到 `~/.cursor/hooks/nixie-hook` |

crate 名不必与产品显示名一致；改名成本高（脚本、文档、CI），除非有计划性迁移。

## 本机数据与 Hook 协议（仍为 `nixie`）

以下路径与文件名**刻意保留** `.nixie` / `nixie-hook`，与 Cursor 侧 hooks 配置、既有用户目录一致：

- 状态与配置目录：`~/.nixie/`（如 `state.json`、`pet.sock`、`window.json`、`overlay.json`、`quotes.json` 等）
- Hook 写入、宠物读取的约定仍以该目录为准（见 `architecture.md`、`hook_state.rs`、`window_prefs.rs` 等）

若将来要改为 `~/.codepet/` 等，需要**迁移脚本 + 文档 + 可选兼容读旧路径**，单独立项，勿在改显示名时顺手改路径。

## 前端与资源（实现层前缀）

下列为**实现细节**，与产品对外名称可以不一致；除非做大重构，不必为「Codepet」全局重命名：

- WebView 自定义协议：`nixie://`（`main.rs` → `with_custom_protocol("nixie", …)`）
- JS 全局 / localStorage 等：`__nixieMeta`、`__nixieSoundEnabled`、`nixie.pomodoroSec`、`nixieT()`、`NixieIdlePlay` 等
- HTML/SVG id：如 `nixie-pixel-speech`

新功能若新增对外文案，**面向用户的字符串**用 **Codepet**；内部变量/协议前缀可继续 `nixie` 前缀以保持代码稳定。

## 改名字时要动哪些地方（速查）

| 目标 | 建议改动位置 |
|------|----------------|
| 用户看到的应用名 / Dock / 菜单栏 | `packaging/macos/Info.plist`（Name、DisplayName）；必要时核对上表「用户可见」 |
| 关于 / 多语言标题 | `nyanpig-i18n.js`、`nyanpig-body.html` |
| 窗口标题 | `main.rs` |
| Bundle ID / `.app` 内二进制名 | `Info.plist` + `bundle-nixie-macos-app.sh`（须与 `CFBundleExecutable` 一致） |
| 数据目录名 | **单独设计迁移**，勿仅改字符串 |

## 相关文件

- `nixie-pet/packaging/macos/Info.plist`
- `scripts/bundle-nixie-macos-app.sh`
- `nixie-pet/assets/icon/build_macos_icon.py`（仅影响图标，不改变名称）
- 架构与数据流：`docs/architecture.md`

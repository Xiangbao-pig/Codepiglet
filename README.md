# Codepiglet（Nixie）— 你的桌面「彩虹小猪」

> 一只会飞的像素小猪，**不领工资、不交周报**，但会盯着 Cursor 里的 AI 干活，并在你写代码时假装自己也在加班。

它会在 AI **发呆**时变蓝、**敲代码**时拖彩虹、**翻车**时气得发红、**跑通**时金光闪闪。  
你也可以投喂、遛它、关掉 8-bit 音效（邻居会感谢你）。**它不评判你的品味，只评判你的 Agent 有没有在摸鱼。**

---

## 三分钟上手（真的只要三步）

```bash
# 1. 装好 Cursor Hooks（会编 nixie-hook，并写好 ~/.cursor/hooks.json）
./scripts/install-hooks.sh

# 2. 重启 Cursor（让钩子生效——对，就是得重启，没有魔法）

# 3. 把小猪叫出来
cargo run -p nixie-pet

# 可选：指定工作区，让小猪对你的项目更「专一」
cargo run -p nixie-pet -- /path/to/your/project
```

装完如果小猪没反应：先看 Cursor 是否真的在跑、Hooks 是否装好；**技术细节在下面「给好奇宝宝」一节**，别慌。

---

## 小猪今天心情怎么样？（人话版）

| 你大概会看到 | 人话 |
|-------------|------|
| 粉粉的、彩虹晃悠 | 你在写代码，或者 AI 在写，总之有人在动键盘 |
| 蓝蓝的、冷色条 | AI 在「思考人生」（thought） |
| 橙/火系 | 终端/命令在跑，小心别 `rm -rf` |
| 绿绿的、像在翻书 | 在读文件、搜代码 |
| 戴墨镜冲浪色 | 在上网搜东西（MCP 那套） |
| 红红的、抖一抖 | 出错了——小猪比你还急 |
| 金闪闪 | 成了！可以假装自己很厉害 |
| 灰灰的、想睡觉 | 太久没人理它，小猪进入省电模式 |

想查**完整状态名、触发条件、皮肤表**：请看 [`docs/pet-states.md`](docs/pet-states.md)（那里才是正经文档）。

---

## 台词自定义（可选）

小猪气泡里会随机冒台词；你也可以改成自己的毒鸡汤或冷笑话。

- 配置文件：`~/.nixie/quotes.json`（**UTF-8**）
- 抄示例：`cp quotes.example.json ~/.nixie/quotes.json` 再改

---

## 给好奇宝宝：它到底怎么知道 AI 在干嘛？

一句话：**Cursor Hooks** 把事件丢给本地的小程序 `nixie-hook`，它把状态写进 `~/.nixie/state.json`；桌面小猪读这个文件（在 macOS 上还会用套接字**拍一下**小猪让它别睡太死）。  
**不需要装 VS Code 扩展**，东西都在你电脑上转，延迟≈没有。

- Hook 与状态对照：[`docs/hooks-to-pet-states.md`](docs/hooks-to-pet-states.md)  
- 架构、Core/Overlay 分工：[`docs/architecture.md`](docs/architecture.md)  
- 分支气泡、Git 角标：[`docs/git-branch-bubble.md`](docs/git-branch-bubble.md)  
- 分支怎么管、怎么提交不翻车：[`docs/branches.md`](docs/branches.md)、[`docs/code-management.md`](docs/code-management.md)

---

## 项目长什么样？（极简地图）

```
nixie-hook/     # 钩子入口：JSON 进，状态出
nixie-pet/      # 透明窗口 + 像素猪（HTML/CSS/JS 拼进 Rust）
hooks.json      # 钩子配置模板
scripts/        # 一键安装等脚本
docs/           # 正经说明都在这儿
```

---

## 小猪视觉素材（致谢）

桌面小猪的**像素飞猪 / 彩虹猫风格视觉**参考并改编自 CodePen 上的公开作品，特此标明来源，避免不必要的误会：

- [Nyan Cat — CodePen by aelweak](https://codepen.io/aelweak/pen/YzaLRGB)

我们在其思路上做了大量本地化与改造（状态皮肤、气泡、交互、与 Rust/WebView 集成等），**与原作者作品并非 1:1 拷贝**；若你二次分发或商用，请同时遵守该 CodePen / 原作者的许可说明（如有）。

---

## License

MIT

### 字体

气泡用 **Ark Pixel**（简体中文子集，已嵌进二进制，不用单独装字体）。授权见 [`nixie-pet/assets/fonts/OFL.txt`](nixie-pet/assets/fonts/OFL.txt)。

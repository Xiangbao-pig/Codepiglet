# Fonts（随 nixie-pet 二进制分发）

- **`ark-pixel-10px-monospaced-zh_cn.otf.woff2`** — [Ark Pixel](https://github.com/TakWolf/ark-pixel-font) 10px 等宽、**简体中文**子集，与 macOS/Windows 上安装的「Ark Pixel 10px Mono · zh_cn」一致。由 `main.rs` 通过 `include_bytes!` 编入可执行文件，运行时经 wry 自定义协议 `nixie://localhost/fonts/ArkPixel.woff2` 提供给 WebView（URL 仅为资源路径，与文件名无关）。
- **`OFL.txt`** — SIL Open Font License 1.1；再分发须保留本文件（见许可证正文）。

更新字体时：用上游同名的 `zh_cn` woff2 覆盖本目录对应文件后执行 `cargo build` 即可。

# Codex CLI 消息折叠

**中文** | [English](README.en.md) | [项目入口](README.md)

这是一个源码完整的 OpenAI Codex CLI 实验性分支，在终端显示层增加了由用户控制的消息折叠功能。项目基于上游 Codex 提交
`1bbdb32789e1f79932df44941236ea3658f6e965`。

## 功能范围

本分支在不改变 Agent 语义的前提下增加：

- 折叠或展开任意一条已经完成的用户消息；
- 折叠或展开任意一条已经完成的助手消息；
- 把流式助手回复视为一条消息，只显示一个占位符；
- 折叠当前任务中的全部用户和助手消息；
- 展开当前任务中的全部消息；
- 按 Codex 任务/thread 分别持久化折叠状态；
- 切换任务或执行 `codex resume` 后恢复折叠状态；
- 关闭 `Ctrl+T` transcript 时重建普通 inline 历史，使两个界面同步；
- 不进行任何自动折叠。

折叠只改变终端渲染，**不会**改变 rollout、模型上下文、已保存消息、工具调用、文件修改、Git 状态或外部副作用。

## 操作方式

在普通 Codex CLI 界面按 `Ctrl+T` 打开由程序管理的 transcript。

| 快捷键          | 作用                                    |
| --------------- | --------------------------------------- |
| `Tab`           | 选择下一条已经完成的用户或助手消息      |
| `Shift+Tab`     | 选择上一条已经完成的用户或助手消息      |
| `Space`         | 折叠或展开当前选中的消息                |
| `f`             | 折叠当前任务中的全部用户和助手消息      |
| `Shift+F`       | 展开当前任务中的全部消息                |
| `q` 或 `Ctrl+T` | 关闭 transcript，并重建普通 inline 界面 |

折叠消息会被替换为可恢复的单行占位符：

```text
▶ User message collapsed
▶ Assistant message collapsed
```

普通界面使用终端原生 scrollback，并不保留每条旧消息的可交互组件。因此，选择操作在 `Ctrl+T` transcript 中完成；关闭它以后，结果立即同步回普通界面。

## 依赖与环境

仓库通过 [`codex-rs/rust-toolchain.toml`](codex-rs/rust-toolchain.toml) 固定工具链：

- Rust 与 Cargo：精确版本 `1.95.0`；
- rustup：用于安装固定工具链以及 `rustfmt`、`clippy`、`rust-src` 组件；
- Git：建议 `2.23+`；
- 内存：最低 4 GB，建议 8 GB；
- 文档中的开发检查还需要 `just`、`cargo-nextest` 和 `dotslash`；本次验证版本为 `just 1.56.0`、`cargo-nextest 0.9.140`、`dotslash 0.5.9`；
- macOS：Xcode Command Line Tools（Clang 与系统 SDK）；
- Ubuntu/Debian：C/C++ 构建工具、`pkg-config` 和 `libcap-dev`；
- Windows：按照上游支持范围使用 Windows 11 + WSL2。

上游基线支持 macOS 12+、Ubuntu 20.04+/Debian 10+ 和 Windows 11 WSL2。本分支已在 macOS 15.7.7、Apple Silicon（`arm64`）、Apple Clang 17、Rust/Cargo 1.95.0 上完成构建和测试。终端需要能把 `Ctrl+T`、`Tab`、`Shift+Tab` 与 `Shift+F` 传递给 TUI。

仓库提交了 `codex-rs/Cargo.lock`。加上 `--locked` 可以让 Cargo 在依赖解析发生变化时直接失败。Cargo 还会获取锁定的 Git 依赖及子模块；若所在网络阻止 Google Source，需要由管理员为 `libyuv` 配置镜像，并确保提交版本不变。

## 克隆、构建与运行

```shell
git clone https://github.com/wdxhr3/codex-transcript-folding.git
cd codex-transcript-folding/codex-rs

rustup show active-toolchain
cargo build --locked --bin codex
cargo run --locked --bin codex -- --no-alt-screen
```

首次运行请使用 Codex CLI 正常登录流程。编译后也可以直接运行：

```shell
./target/debug/codex --no-alt-screen
./target/debug/codex resume --last --no-alt-screen
./target/debug/codex resume --no-alt-screen
```

人工检查普通 inline scrollback 时建议使用 `--no-alt-screen`；该功能也适用于默认的 alternate-screen 模式。

## 自动化检查

在仓库根目录执行：

```shell
just fmt-check
just clippy -p codex-tui -- -D warnings
just test -p codex-tui
```

从 `codex-rs/` 直接执行的等价命令：

```shell
cargo fmt --all -- --check
cargo clippy --tests -p codex-tui -- -D warnings
RUST_MIN_STACK=8388608 NEXTEST_PROFILE=local \
  cargo nextest run --no-fail-fast -p codex-tui
```

发布候选已经过格式检查、无警告 Clippy、debug binary 构建、折叠功能定向测试和完整 `codex-tui` nextest 测试：3,087 项通过，4 项按上游测试配置跳过。

## 从零开始人工验证

1. 按上面的命令克隆并构建。
2. 启动 `./target/debug/codex --no-alt-screen`，创建一个新任务。
3. 连续发送两条容易区分的消息，并等待两次助手回复结束。
4. 按 `Ctrl+T`，再用 `Tab`/`Shift+Tab` 移动选择。只有已完成的用户和助手消息应被选中；工具和状态 cell 不应被选中。
5. 选中用户消息并按 `Space`，预期出现 `▶ User message collapsed`。
6. 选中助手回复并按 `Space`。即使它由多个流式 cell 组成，也应只出现一个 `▶ Assistant message collapsed`。
7. 按 `q`。普通 inline 界面应重建并显示相同的两个占位符，其他内容保持可见。
8. 再打开 `Ctrl+T` 并按 `f`，预期所有用户/助手消息都折叠；关闭后确认普通界面一致。
9. 重新打开并按 `Shift+F`，预期两个界面中的全部原文恢复。
10. 再次折叠至少一条用户消息和一条助手消息，关闭 overlay，然后正常退出 Codex。
11. 运行 `./target/debug/codex resume --last --no-alt-screen`。历史重放完成后，普通界面和 `Ctrl+T` 中相同的消息都应保持折叠。
12. 检查 `~/.codex/ui-state/transcript-folds/<thread-id>.json`，应存在对应 JSON 文件。
13. 追问一条依赖被折叠消息的问题，并检查 `git diff`。Agent 应仍能使用上下文，折叠操作本身不应修改项目文件。
14. 执行 `/clear` 后发送新消息，确认新消息不会继承旧任务 ordinal 的折叠状态。

## 典型场景与限制

该功能适合长时间实现任务：旧问题或冗长回复妨碍阅读，但完整上下文仍需保留。

- 只能选择已经完成并写入历史的用户/助手消息；正在流式生成的 active 输出不能折叠。
- 工具调用、命令输出、状态卡和推理 cell 不属于“全部折叠”范围。
- 重建终端历史可能有轻微闪烁，并可能重置终端文本选择或滚动位置。
- 折叠状态独立于 rollout JSONL；只复制 rollout 不会复制 UI 状态。
- `/clear` 会清空当前任务的 ordinal 映射；分叉具有新的 thread ID，不继承父任务折叠状态。
- 本功能没有增加鼠标选择，也不能直接在终端原生 scrollback 中选择旧消息。可靠实现这种交互需要把普通界面改造成完全由程序管理的 transcript。
- 本项目跟踪一个上游快照，不是 OpenAI 官方发行版。

## 实现与归属

主要修改位于 `codex-rs/tui/src/transcript_folding.rs`、`pager_overlay.rs`、`app_backtrack.rs` 和 `app/resize_reflow.rs`。状态文件以 thread ID 和 rollout 内稳定的用户/助手 ordinal 为键，原子写入 `~/.codex/ui-state/transcript-folds/`。

架构、源码地图、状态格式、安全边界和更详细的验证矩阵见 [`codex-rs/tui/TRANSCRIPT_FOLDING.zh-CN.md`](codex-rs/tui/TRANSCRIPT_FOLDING.zh-CN.md)。

本项目派生自 [OpenAI Codex](https://github.com/openai/codex)，保留上游 [Apache-2.0 许可证](LICENSE)及 [NOTICE](NOTICE)。Codex 与 OpenAI 商标归各自权利人所有；本仓库是独立的实验性分支。

# Codex CLI 消息折叠：使用、设计与人工验证

[中文](TRANSCRIPT_FOLDING.zh-CN.md) | [English](TRANSCRIPT_FOLDING.en.md) | [项目中文说明](../../README.zh-CN.md)

## 1. 功能范围

这份修改为 Codex CLI 增加纯显示层的 transcript 折叠能力：

- 可单独折叠任意一条已经完成的用户消息；
- 可折叠一条已经完成的助手回复，并同时收起该回复关联的工具调用和其他执行活动；
- 可一次折叠全部用户和助手消息；
- 可一次展开全部消息；
- 折叠状态按 Codex 任务（thread）持久化；
- 退出并用 `codex resume` 恢复任务后，折叠状态仍然生效；
- 关闭 `Ctrl+T` transcript 后，普通 inline 界面会重建历史记录，并显示折叠占位符。

折叠不修改 rollout、模型上下文、工具调用、文件改动或工作区状态。它只改变终端中的显示结果。

## 2. 源码路径与状态格式

- 官方源码仓库：<https://github.com/openai/codex>
- 本项目仓库：<https://github.com/wdxhr3/codex-transcript-folding>
- Rust workspace：`codex-rs/`
- 核心折叠状态与消息编号：`codex-rs/tui/src/transcript_folding.rs`
- `Ctrl+T` 选择与折叠交互：`codex-rs/tui/src/pager_overlay.rs`
- overlay 打开、关闭和普通界面重绘：`codex-rs/tui/src/app_backtrack.rs`
- 普通界面历史重放：`codex-rs/tui/src/app/resize_reflow.rs`
- 持久化状态文件：`~/.codex/ui-state/transcript-folds/<thread-id>.json`

状态文件示例：

```json
{
  "collapsed_messages": [
    {
      "kind": "user",
      "ordinal": 0
    },
    {
      "kind": "assistant",
      "ordinal": 1
    }
  ]
}
```

`ordinal` 是该任务内同类消息的顺序编号。用户消息保持独立；一条用户消息之后、下一条用户消息之前的全部助手输出和已提交活动共享一个助手编号。工具 cell 不能单独选择，但会随其所属的助手回复一起折叠。状态文件采用原子写入。

## 3. 为什么选择操作在 `Ctrl+T` 中完成

Codex CLI 的普通 inline 界面把已经完成的消息写入终端原生 scrollback。写入后，Codex 不再拥有一个可以逐行点击或选择的完整消息组件树，因此无法可靠地在原位置选择任意旧消息。

`Ctrl+T` transcript 是 Codex 自己管理的全屏视图，保留了每个 history cell 的边界，所以适合选择消息。用户关闭 transcript 后，程序清空 Codex 管理的历史显示，并从原始 history cell 重新生成普通界面：已折叠消息变成一行占位符，其他内容照常显示。

因此，这个方案的实际体验是：

1. 在普通界面按 `Ctrl+T` 管理折叠；
2. 在 transcript 中选择和折叠；
3. 关闭 transcript；
4. 折叠结果立即出现在普通界面。

如果要求直接在普通 scrollback 中移动光标、点选旧消息并原地折叠，就需要把普通界面改造成由 Codex 持有的全屏 transcript，而不是继续使用终端原生 scrollback。这会是一项明显更大的 TUI 架构改造。

## 4. 快捷键

先在普通 Codex CLI 界面按 `Ctrl+T` 打开 transcript。

| 快捷键          | 作用                                        |
| --------------- | ------------------------------------------- |
| `Tab`           | 选择下一条用户或助手消息                    |
| `Shift+Tab`     | 选择上一条用户或助手消息                    |
| `Space`         | 折叠或展开当前选中的消息                    |
| `f`             | 折叠当前任务中的全部用户和助手消息          |
| `Shift+F`       | 展开当前任务中的全部消息                    |
| `q` 或 `Ctrl+T` | 关闭 transcript，并把折叠结果重绘到普通界面 |

助手的流式回复在内部可能由多个 cell 组成，并可能包含网页搜索、命令、补丁、MCP 调用或其他活动。整段回复只显示一条摘要，例如：

```text
▶ Assistant response collapsed · 3 tool calls · 12 lines · 684 chars · 1 related item
```

任何调用都不会自动隐藏；只有用户折叠其所属的助手回复后，调用才会随之收起。

## 5. 构建与运行

仓库通过 `codex-rs/rust-toolchain.toml` 固定 Rust、Cargo 1.95.0，并要求 `rustfmt`、`clippy` 与 `rust-src` 组件。开发检查还使用 `just`、`dotslash` 与 `cargo-nextest`。本次验证版本分别为 just 1.56.0、DotSlash 0.5.9、cargo-nextest 0.9.140。

macOS 需要 Xcode Command Line Tools；Ubuntu/Debian 需要 C/C++ 构建工具、`pkg-config` 和 `libcap-dev`。Cargo 会按 `codex-rs/Cargo.lock` 获取锁定依赖及子模块。如果网络阻止 Google Source，可以为 `libyuv` 配置可信镜像，但必须保持锁定提交不变。

在终端运行：

```shell
git clone https://github.com/wdxhr3/codex-transcript-folding.git
cd codex-transcript-folding
just fmt-check
just clippy -p codex-tui -- -D warnings
just test -p codex-tui
```

从源码启动修改后的 CLI：

```shell
cd codex-rs
cargo run --locked --bin codex -- --no-alt-screen
```

第一次运行需要编译，之后也可以直接运行：

```shell
./target/debug/codex --no-alt-screen
```

恢复最近的任务：

```shell
./target/debug/codex resume --last --no-alt-screen
```

也可以显示任务选择器：

```shell
./target/debug/codex resume --no-alt-screen
```

## 6. 人工验证步骤

### 验证 A：折叠一条用户消息

1. 用上面的命令启动修改后的 Codex CLI。
2. 连续发送至少两条容易辨认的消息，例如“折叠测试：用户消息 A”和“折叠测试：用户消息 B”。
3. 等助手回复完成后按 `Ctrl+T`。
4. 按 `Tab`，观察反色选中框只在用户和助手消息之间移动，不选择工具输出。
5. 选中“用户消息 A”后按 `Space`。
6. 确认该消息在 transcript 中变成 `▶ User message collapsed`。
7. 按 `q` 关闭 transcript。
8. 确认普通界面已经重绘，并在原消息位置显示相同占位符；消息 B、工具输出和助手回复仍正常存在。

### 验证 B：折叠一条助手消息

1. 再按 `Ctrl+T`。
2. 用 `Tab` 或 `Shift+Tab` 选中一条已完成的助手回复。
3. 按 `Space`。
4. 确认只出现一行 `▶ Assistant response collapsed ...` 摘要。即使回复原来由多个流式 cell 组成，也不应重复出现多个占位符。
5. 如果该回复使用了网页搜索、命令、补丁或 MCP 工具，确认这些关联 cell 也已消失。
6. 展开该回复，确认回复文本和关联活动同时恢复。
7. 按 `q`，确认普通界面显示一致。

### 验证 C：全部折叠和全部展开

1. 按 `Ctrl+T`，再按小写 `f`。
2. 确认所有用户消息和助手回复都变成占位符；每条助手回复关联的工具调用和执行活动应随之折叠。
3. 按 `q`，确认普通界面显示相同结果。
4. 再按 `Ctrl+T`，按 `Shift+F`。
5. 确认全部用户和助手消息恢复完整显示。
6. 按 `q`，确认普通界面也恢复。

### 验证 D：任务恢复后的持久化

1. 折叠至少一条用户消息和一条助手消息，然后按 `q` 返回普通界面。
2. 正常退出 Codex CLI。
3. 运行修改后 binary 的 `resume --last --no-alt-screen`。
4. 任务恢复完成后，确认普通界面中的对应消息仍是折叠状态。
5. 按 `Ctrl+T`，确认 transcript 中也保持相同状态。
6. 检查 `~/.codex/ui-state/transcript-folds/`，应存在以当前 thread UUID 命名的 JSON 文件。

### 验证 E：折叠不影响上下文和产出

1. 折叠一条包含明确事实的旧消息，但不要删除或修改工作区。
2. 在下一轮询问助手该事实，确认助手仍能使用原上下文。
3. 执行 `git status` 或 `git diff`，确认折叠动作本身没有产生项目文件改动。
4. 展开该消息，确认原文完整恢复。

### 验证 F：`/clear` 不误折叠新消息

1. 折叠第一条用户消息。
2. 执行 `/clear`。
3. 发送一条新消息。
4. 确认新消息不会因为重新成为“第 0 条用户消息”而继承旧折叠状态。

## 7. 已知行为与限制

- 只能折叠已经提交到历史记录的用户或助手消息；正在流式生成的 active cell 不可选择。
- 工具调用不能单独选择，也不会被自动隐藏；它们跟随所属助手回复的折叠状态。
- 关闭 transcript 后需要重建 Codex 管理的终端历史，因此终端滚动位置或正在进行的文本选择可能丢失，并可能出现一次轻微闪烁。
- 折叠状态独立于 rollout；复制 rollout 文件不会自动复制 UI 状态文件。
- `/clear` 会清空当前任务的折叠映射，避免新消息继承旧 ordinal。
- 任务分叉会获得新的 thread ID，因此新分支默认没有父任务的折叠状态。

## 8. 数据安全边界

折叠按钮不会：

- 从模型上下文删除消息；
- 删除存储的对话历史；
- 撤销命令、文件修改或外部操作；
- 修改 Git 历史；
- 改变 Agent 后续遵循的指令。

它等价于“可恢复的显示折叠”，不等价于删除、遗忘或撤销。

# Demo 界面设计（v1.6.0 验证版）— tauri-plugin-multiline-menubar

> 仓库：`tauri-plugin-multiline-menubar`
> 分支：feat/demo-v160（demo 换新身份 `multiline-menubar-demo-new`，插件 API 为 **v1.6.0** 方案）
> 更新：2026-08-15
> 用途：用**新的 demo 页面设计**（5 实例管理界面 + popup 面板）验证 **v1.6.0 插件方案**——⌘拖出单个 item 时只移除该 item、只取消设置界面中该实例的 toggle，**不是**整个 app 的所有 menubar 一起消失。
> 定位：本文档**不含 v1.7.0 stats 复刻内容**（removeStatusItem 隐藏语义 / autosaveName / Rebuild shown 恢复钥匙 / 2s 轮询 / app 级隐藏警示），仅改 demo 设计，插件保持 1.6.0。

---

## 1. 总览

Demo 由三块 UI 组成（同设计文档，页面不动，语义换 1.6.0）：

| 面 | 文件 | 作用 |
|---|---|---|
| **设置界面**（主窗口） | `examples/demo/src/index.html` + `main.js` | 5 实例的开启/关闭管理 + 移除反应 |
| **菜单栏** | 插件 native 层（`.mm`，v1.6.0） | 5 个两行文本 item + 右键菜单 + 点击开 popup |
| **Popup 设置面板** | `examples/demo/src/popup.html` + `popup.js` | 点击菜单栏实例弹出的**该实例**外观设置（文本/字体/大小/加粗/颜色/对齐/布局） |

数据流（1.6.0 命令集）：`create`/`set_visible`/`set_text`/`set_font_sizes`/`set_colors`/`set_bold`/`set_font_family`/`set_monospaced`/`set_alignment`/`set_layout`/`set_menu`；事件：`ready`/`click`/`enter`/`leave`/`menu`/`remove`/`popup-open`/`popup-close`。

**本版核心验证点**：`{id}//remove` 事件（v1.6.0 KVO 检测）触发后，设置界面**只把该实例的 toggle 置 off**，其余 4 个实例不受影响、继续显示。

---

## 2. 设置界面（主窗口，index.html）

```
Multiline Menubar Plugin
multiline-menubar-demo-new · com.tauri.multiline-menubar-demo-new
说明文案（5 实例 + 单实例移除语义提示）
┌ Menubar instances ─────────────────────────────┐
│ [Show all] [Hide all]  状态                    │
│ ┌──────────────────────────────────────────┐  │
│ │ mb-1   "mb-1"/"mb-1"        [===● 开关]  │  │  ← 每行：实例名 + 当前文本 + switch
│ │ mb-2   "mb-2"/"mb-2"        [===● 开关]  │  │
│ │ ...（mb-3/4/5 同）                        │  │
│ └──────────────────────────────────────────┘  │
│ Click an item in the menu bar to open its settings popup. │
└──────────────────────────────────────────────┘
```

### 设计要点

- **5 个固定实例** `mb-1`…`mb-5`，启动即创建（macOS 26：运行期动态 create 可能不显示，必须在启动早期预创建）——沿用原设计
- **每实例独立 switch toggle**（iOS 风格 `.switch`）：
  - 开 = `set_visible(true)`（v1.6.0：`statusItem.visible = YES`，同时复位 `removedByUser`，被拖出的 item 可重新显示）
  - 关 = `set_visible(false)`（v1.6.0：`programmaticHide` 守卫下 `visible = NO`，**不触发** remove 事件）
- **移除反应（本版核心，与 1.7.0 不同）**：收到 `multiline-menubar://{id}//remove`（用户 ⌘拖出）→
  - `shown[id] = false` + 写入 localStorage
  - 只渲染该行 switch 为 off；**其他实例保持不变**
  - 状态行/日志轻提示（如 `mb-3 被拖出 — toggle 已关闭，重新打开可恢复显示`）
  - **没有** 1.7.0 的"整个 app 菜单栏消失"红色警示 banner（1.6.0 语义下其他实例仍显示）
- **两个批量按钮**：`Show all` / `Hide all`（**没有 Rebuild shown**——那是 1.7.0 stats 恢复钥匙，1.6.0 单实例移除不需要系统设置介入）
- **持久化**：shown 状态存 `localStorage`（key `multiline-menubar-demo-new:shown-v1`），重启保持
- 状态行：`5 instances · showing 3 / hidden 2`

### 样式（styles.css）

- `.instance-list` / `.instance-row`：卡片式列表行（实例名 + 当前文本 + 右侧 switch）——沿用
- `.switch`：iOS 风格 toggle（40×24，滑块动画，开=蓝）——沿用
- 无 `.remove-banner`（或仅保留为行内轻提示样式，不承载"系统设置-菜单栏重新勾选"文案）

---

## 3. 菜单栏（menubar，v1.6.0 插件）

- **5 个 item**：两行文本都显示自身 id（`mb-1`…`mb-5`，便于沟通指认），默认 emphasis-bottom 布局（上小下大）
- **点击** → 打开该实例的设置 popup（`click` 事件 + auto-popup）
- **右键菜单**：`multiline-menubar-demo-new v0.1.0`（disabled 版本行）+ 分隔 + `Quit (⌘Q)`（内置 `quit` id，Rust 侧 `app.exit`）
- **⌘拖出（v1.6.0 机制）**：
  - native 设 `NSStatusItemBehaviorRemovalAllowed` → 系统允许**单个 item** 被拖出移除
  - KVO 观察 `visible`（macOS 13+）/ `button.window`（旧系统）：`visible` 翻 NO 或脱离 window → `removedByUser = YES` → `g_removeCallback` → Rust `emit("multiline-menubar://{id}//remove")`
  - 移除检测为"延迟布防"（`everVisible` + `tryArmAfterSettle` 轮询，启动瞬态不误判）
  - **预期**：只移除被拖出的那一个 item，其余 4 个继续显示——这是本版要验证的行为

---

## 4. Popup（实例设置面板，400×700）

与设计文档一致，无改动：

标题：`Instance settings — mb-N`（打开时由 `popup-open` 事件填充该实例当前值）

| 区块 | 控件 | 按钮 |
|---|---|---|
| 文本 | Top label / Bottom value | Update / Reset（Reset=恢复 mb-N） |
| 颜色 | Top/Bottom color picker + Hex（picker 选中自动填 hex，可手改） | Apply colors / Reset（回系统色） |
| 字号 | 布局 radio（Small·Large / 镜像 / Equal）+ 大小滑块 | Apply sizes / Reset（恢复 7/12/9） |
| 加粗 | Top/Bottom bold checkbox | Apply bold / Reset |
| 字体族 | Top/Bottom family 输入 | Apply family / Reset |
| 等宽数字 | Top/Bottom checkbox | Apply / Reset |
| 对齐 | Top/Bottom select（Left/Center/Right） | Apply / Reset |
| 底部 | — | Close |

设计要点：每选项组独立 Update/Reset；只作用于当前实例（`currentInstanceId`）；打开时回显当前值；窗口 400×700，菜单栏下方展开定位；Close / 点击别处关闭。

---

## 5. 设计原则（v1.6.0 版，宿主可复用）

1. **实例身份**：稳定 id（mb-1…mb-5）；**无 autosaveName**（那是 1.7.0 加的，1.6.0 不记忆位置）
2. **开关语义（1.6.0）**：隐藏 = `set_visible(false)`（`programmaticHide` 守卫，`visible=NO` 不触发 remove 事件）；显示 = `set_visible(true)`（`visible=YES`，复位 `removedByUser`）
3. **移除语义（1.6.0，本版验证目标）**：⌘拖出 = 系统移除**单个** item → KVO 检测 → remove 事件 → 设置界面**只取消该实例 toggle**；恢复 = 重新打开该实例 toggle（`set_visible(true)`），**无需系统设置-菜单栏操作**
4. **编辑即 popup**：点击菜单栏实例 = 编辑该实例（所见即所得，避免主窗口堆控件）
5. **持久化**：shown 状态 localStorage（宿主可用自己的存储）

---

## 6. v1.6.0 方案 vs v1.7.0 stats 复刻（行为对比）

| 维度 | v1.6.0 方案（本文档） | v1.7.0 stats 复刻 |
|---|---|---|
| 拖出允许 | `NSStatusItemBehaviorRemovalAllowed`（系统允许单 item 移除） | 不设 RemovalAllowed |
| ⌘拖出结果 | 单个 item 被系统移除，其余 4 个不动 | macOS 26：app 级隐藏（整个 app 菜单栏消失） |
| remove 检测 | KVO `visible` / `button.window`（延迟布防） | 宿主轮询 is_visible + 插件 2s 轮询 `button.window` |
| 隐藏语义 | `set_visible(false)` → `visible=NO`（programmaticHide） | removeStatusItem（从不 `setVisible(false)`） |
| 恢复路径 | 重新开 toggle → `set_visible(true)` | 系统设置-菜单栏重新勾选 + **Rebuild shown** |
| UI 反应 | 只取消该实例 toggle + 行内提示 | 整个 app 红色警示 banner |
| 位置记忆 | 无 autosaveName | `autosaveName = instanceId` |

---

## 7. 实现文件映射

| 文件 | 职责 |
|---|---|
| `examples/demo/src/index.html` | 主窗口骨架（5 实例列表 + switch + Show/Hide all + 行内移除提示） |
| `examples/demo/src/main.js` | 实例生命周期（create/toggle）+ localStorage + remove 事件 → **仅置 off 对应实例** |
| `examples/demo/src/popup.html` | popup 表单（7 个选项组） |
| `examples/demo/src/popup.js` | popup-open 填充 + 各 Apply/Reset 按钮 + currentInstanceId |
| `examples/demo/src/styles.css` | `.instance-list/.instance-row/.switch`（无 `.remove-banner`） |
| `src/desktop.rs`（v1.6.0） | 命令 + `on_native_remove` → emit `{id}//remove` |
| `src/native/multiline_menubar.mm`（v1.6.0） | RemovalAllowed + KVO 移除检测（延迟布防）+ `set_instance_visible`（programmaticHide/removedByUser） |

---

## 8. 待验证清单（macOS 26 实测）

1. ⌘拖出单个 item：是否**只**移除该 item（RemovalAllowed 在 macOS 26 是否仍生效，还是也会 app 级隐藏）？
2. KVO `visible`/`button.window` 在 macOS 26 上能否捕获移除（1.7.0 曾发现 `visible` 不翻 NO 的 app 级隐藏场景；1.6.0 的 `button.window` 观察路径是否兜住）？
3. remove 事件触发后，设置界面是否只关掉该实例 toggle、其余 4 个保持显示？
4. 重新打开被移除实例的 toggle：`set_visible(true)` 能否把 item 恢复显示（`removedByUser` 复位路径）？
5. 程序化 Hide all / Show all：`programmaticHide` 守卫下不误发 remove 事件？

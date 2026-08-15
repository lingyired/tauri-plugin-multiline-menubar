# Demo 界面设计 — tauri-plugin-multiline-menubar

> 仓库：`tauri-plugin-multiline-menubar`
> 分支：main（v1.6.0 原始渲染 + stats 复刻改动，commit `811574d`）
> 更新：2026-08-15
> 用途：这是 `examples/demo` 的设置/管理交互范式总结——比旧 demo 合理（用户认可），可作为宿主（fund01）多实例管理的参考实现。

---

## 1. 总览

Demo 由三块 UI 组成：

| 面 | 文件 | 作用 |
|---|---|---|
| **设置界面**（主窗口） | `examples/demo/src/index.html` + `main.js` | 5 实例的开启/关闭管理 + 恢复操作 + 拖出警示 |
| **菜单栏** | 插件 native 层（`.mm`） | 5 个两行文本 item + 右键菜单 + 点击开 popup |
| **Popup 设置面板** | `examples/demo/src/popup.html` + `popup.js` | 点击菜单栏实例弹出的**该实例**外观设置（文本/字体/大小/加粗/颜色/对齐/布局） |

数据流：设置界面/popup → 插件命令（`create`/`set_visible`/`set_text`/`set_font_sizes`/`set_colors`/`set_bold`/`set_font_family`/`set_monospaced`/`set_alignment`/`set_layout`/`set_menu`）→ native 渲染；插件事件（`ready`/`click`/`enter`/`leave`/`menu`/`remove`/`popup-open`/`popup-close`）→ 设置界面/popup 响应。

---

## 2. 设置界面（主窗口，index.html）

```
Multiline Menubar Plugin
[⚠️ 红色警示条：菜单栏项被移出 — 系统设置→菜单栏 勾选后点 Rebuild shown]   ← 拖出时出现，恢复后隐藏
说明文案（5 实例 + macOS 26 拖出恢复提示）
┌ Menubar instances ─────────────────────────────┐
│ [Rebuild shown] [Show all] [Hide all]  状态    │
│ ┌──────────────────────────────────────────┐  │
│ │ mb-1   "mb-1"/"mb-1"        [===● 开关]  │  │  ← 每行：实例名 + 当前文本 + switch
│ │ mb-2   "mb-2"/"mb-2"        [===● 开关]  │  │
│ │ ...（mb-3/4/5 同）                        │  │
│ └──────────────────────────────────────────┘  │
│ Click an item in the menu bar to open its settings popup. │
└──────────────────────────────────────────────┘
```

### 设计要点

- **5 个固定实例** `mb-1`…`mb-5`，启动即创建（macOS 26：运行期动态 create 可能不显示，必须在启动早期预创建）
- **每实例独立 switch toggle**（iOS 风格，`styles.css` `.switch`）：开 = `set_visible(true)`（缺失/被摘除时插件自动重建），关 = `set_visible(false)`（removeStatusItem，不触发 macOS 26 隐藏记忆）
- **三个批量按钮**：
  - **Rebuild shown** = stats 恢复钥匙：全部可见实例先 off 再 on（重建 item → 系统重新注册整个 app 菜单栏）
  - **Show all / Hide all**：批量开关
- **红色移除警示 banner**（`.remove-banner`）：收到 `remove` 事件（⌘拖出）显示，文案提示「系统设置-菜单栏勾选 + Rebuild shown」；Rebuild/Show all 后隐藏
- **持久化**：shown 状态存 `localStorage`（key `multiline-menubar:shown-v1`），重启保持
- 状态行：`5 instances · showing 3 / hidden 2`

### 样式（styles.css）

- `.instance-list` / `.instance-row`：卡片式列表行（实例名 + 当前文本 + 右侧 switch）
- `.switch`：iOS 风格 toggle（40×24，滑块动画，开=蓝）
- `.remove-banner`：琥珀→红色警示（`#dc2626`，暗色模式 `#f87171`），明暗自适应

---

## 3. 菜单栏（menubar）

- **5 个 item**：两行文本都显示自身 id（`mb-1`…`mb-5`，便于沟通指认），默认 emphasis-bottom 布局（上小下大）
- **点击** → 打开该实例的设置 popup（插件 `click` 事件 + auto-popup）
- **右键菜单**：`MenubarLegacy160 v0.1.0`（disabled 版本行）+ 分隔 + `Quit (⌘Q)`（内置 `quit` id，Rust 侧 `app.exit`，无需 JS 处理）
- **⌘拖出** → 插件 2s 轮询检测（`button.window` 脱离）→ emit `remove` 事件 → 设置界面显示红色警示条

---

## 4. Popup（实例设置面板，400×700）

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

### 设计要点

- **每选项组独立 Update/Reset**：改完即点，不用滚到底部（用户明确要求）
- **只作用于当前实例**：popup 记录 `currentInstanceId`，所有 set_* 都带该 id
- **状态回显**：打开时填充实例当前值（文本/字号/布局/加粗/字体族/等宽/对齐/颜色）；颜色 hex 是新加能力（InstanceState 记录 + popup-open payload 携带）
- 窗口：400×700（大尺寸，`tauri.conf.json`）；**定位**：菜单栏下方展开，高度超出屏幕时顶部贴屏幕顶（`position_popup_under_status_item` 的 `tauri_y.max(0)`）
- 关闭：Close 按钮 / 点击别处（插件 auto-hide）

---

## 5. 设计原则（宿主可复用）

1. **实例身份**：稳定 id（mb-1…mb-5）+ `autosaveName`——系统记住位置，hide/show 重建不重排
2. **开关语义**：隐藏 = removeStatusItem（不 setVisible(false)）；显示 = 缺失/被摘除时重建
3. **恢复引导**：拖出 → 事件 + 红色提示 → 用户系统设置勾选 → Rebuild shown（stats 同款流程，比 stats 稳：重启直接恢复）
4. **编辑即 popup**：点击菜单栏实例 = 编辑该实例（所见即所得，避免主窗口堆控件）
5. **持久化**：shown 状态 localStorage（宿主可用自己的存储）

---

## 6. 实现文件映射

| 文件 | 职责 |
|---|---|
| `examples/demo/src/index.html` | 主窗口骨架 + 警示 banner |
| `examples/demo/src/main.js` | 实例生命周期（create/toggle/rebuild）+ localStorage + remove 事件 → banner |
| `examples/demo/src/popup.html` | popup 表单（7 个选项组） |
| `examples/demo/src/popup.js` | popup-open 填充 + 各 Apply/Reset 按钮 + currentInstanceId |
| `examples/demo/src/styles.css` | `.instance-list/.instance-row/.switch/.remove-banner` |
| `src/desktop.rs` | 命令 + popup 定位 + 拖出轮询（`poll_drag_out_removals`） |
| `src/native/multiline_menubar.mm` | item 构建（autosaveName/view/handler）+ hide/show 语义 + `is_on_screen` |

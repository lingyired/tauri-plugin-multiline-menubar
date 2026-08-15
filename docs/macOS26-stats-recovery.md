# macOS 26 Stats 复刻 — 当前状态与修复记录

> 仓库：`tauri-plugin-multiline-menubar`
> 分支：main（v1.6.0 原始架构），worktree：`/private/tmp/menubar-legacy-test`
> 插件版本：1.7.0（未发布）
> 最后更新：2026-08-15
> 状态：**核心验证通过 ✅**（用户真机实测）——多实例显示/尺寸正常、stats 式拖出恢复、show/hide toggle、重启持久化、与 thaw（第三方菜单栏管理 app，前身 ice）交互正常；**比 stats 更稳定**（重启后直接全显示）。
> **重大结论（2026-08-15 晚）**：用户宿主 app 改用**原始 1.6.0 + 全新 bundle id/name** 后一切正常 → **「之前不显示」的根因是 macOS 26 的 Control Center 旧记忆（bundle 级污染），不是插件代码缺陷**。本仓库的改动（stats 语义 hide/show、autosaveName、remove 事件感知、demo 重构）仍有价值——它们是「正确应对 macOS 26」的健壮实现，且 demo 界面形态更合理（见 §7）。参考：https://b-log.to/tech-analysis/macos-26-controlcenter-trackedapplications-ghost/

---

## 1. 架构决策

- **放弃 demo2-based 重写**（`refactor/demo2-based` 分支的 TrayIcon + objc2 attributedTitle 方案）：能显示但渲染尺寸/内容截断有问题。
- **保留 main 分支 v1.6.0 原始渲染**：自定义 `MultilineMenubarView` + `NSStatusItem`（尺寸调校过、无截断），在干净 bundle 下 macOS 26 正常绘制（调查 §10 实验⑤已证实）。
- **只移植 stats 的 hide/show 语义**到 `.mm` 原生层。

## 2. 已验证通过（用户真机实测，2026-08-14）

| # | 项 | 结果 |
|---|---|---|
| 1 | 启动显示 | 5 实例（mb-1..mb-5）全部显示，渲染尺寸正常、中文/descender 无截断 ✅ |
| 2 | 恢复流程 | ⌘拖出任一实例 → 整个 app 菜单栏消失 → 系统设置-菜单栏勾选 → 点击任意实例/「Rebuild shown」→ 全部恢复 ✅；**app 重启后直接全显示（无需手动恢复，比 stats 稳）** |
| 3 | show/hide toggle | 反复切换同一实例开关正常，不再「首次打开不显示」「隐藏错实例」「实例被挤没」 ✅ |
| 4 | 重启持久化 | hide 过的实例重启后保持隐藏（localStorage）✅ |
| 5 | Hide all → Show all | 全消失 → 全恢复 ✅ |
| 6 | 第三方交互 | 与 thaw（前身 ice 的菜单栏管理 app）配合一切正常 ✅ |
| 7 | 系统设置显示名 | 从 /Applications 启动后显示 app 名（adhoc+临时路径时 fallback bundle id，正常现象）✅ |
| 8 | 应用图标 | menu/bar 两行圆角图标（自定义生成）✅ |

## 3.5 拖出感知（remove 事件，v1.7.0 新能力）

宿主可感知「用户 ⌘拖出 menubar」：

- **事件**：插件后台每 ~2s 轮询一次（`poll_drag_out_removals`，启动 3s 布防防瞬态误判），对比「宿主期望显示」与「原生 item 实际可见性」——期望显示但 item 不可见（系统摘除）→ emit `multiline-menubar://{id}//remove`（`onRemove` API，此前占位现已激活）。
- **状态**：`isVisible(id)` 在拖出后返回 `false`，宿主可轮询。
- **宿主用途**：收到 remove 事件后可在 UI 提示「该实例被移出，系统已隐藏整个 app 菜单栏，请到系统设置勾选后点 Rebuild 恢复」。
- 防重复：每实例 `removed_notified` 标志，re-show 后重置可再次通知。

## 3. 修复历程（show/hide 状态混乱）

### 3.1 背景现象
- 反复 toggle 某一实例：首次 hide 正常 → 首次 show 该实例不出现 → 再次 hide 时**别的实例**（如 mb-5）消失 → 再次 show 恢复 → 之后重复 toggle 正常。
- 关闭某实例后，app 内其他实例勾选正常但菜单栏少一个（被「挤没」）；重启后全部恢复。

### 3.2 根因
1. **macOS 26 会自动摘除 item**：系统把 item 从菜单栏摘除时 `visible` 翻 NO，但插件内部 `itemInBar` 仍为 YES → show 只设 `statusItem.visible = YES` 无效 →「打开不显示」。
2. **无 autosaveName → 重建后位置重排**：hide（`removeStatusItem`）后再 show 是**全新 NSStatusItem**，系统把它当新 item 重新排队 → 位置漂移到不可见区（刘海后）→「首次 show 不出现」；同时 item 增删触发系统重排 → 其他实例被挤到刘海后 →「实例被挤没」（重启后布局重建即恢复，所以看起来「随机」且「重启就好」）。
3. **demo 无持久化** → 重启后全部重新显示。

> stats 为什么正常：stats 每次创建 item 都设 `autosaveName`（`"\(module)_\(type)"`），系统记住每个 item 的身份与位置，hide→show 重建后**恢复原位、不重排其他 item**。

### 3.3 修复（src/native/multiline_menubar.mm）

| 改动 | 说明 |
|---|---|
| `build_status_item(inst)` 抽出 | 新建 item + 挂 view + handler + tracking + redraw；创建/重建复用 |
| **`statusItem.autosaveName = instanceId`** | 对齐 stats：稳定身份 + 位置记忆，重建恢复原位，不扰动其他实例（**关键修复**） |
| show 检测系统摘除 | `!itemInBar \|\| !instance_is_visible(inst)` → 强制重建（先 remove 旧 item）；仅在 item 确实在栏时才 `visible = YES` |
| hide 无条件 removeStatusItem | 幂等，修正 `itemInBar` 与系统状态不同步 |
| view 重建前 `removeFromSuperview` | 排除 view 复用挂错 superview 的边缘情况 |
| 去掉 RemovalAllowed | macOS 26 会系统自动移除并永久记忆（调查 §9） |
| 删除 KVO 移除检测 | 启动瞬态误判风险；拖出感知改由宿主轮询 `isVisible` |
| 诊断日志 `diag_native` | 写 `/tmp/menubar-diag.log`（id / itemInBar / sysVisible / item 指针），**REMOVE BEFORE RELEASE** |

### 3.4 语义总结

```
setVisible(false) → removeStatusItem + itemInBar=NO（不触发 macOS 26 隐藏记忆）
setVisible(true)  → 若 item 不在栏或系统已摘除（visible==NO）→ remove 旧 + 重建（autosaveName 恢复原位）
                  → 否则 visible=YES
isVisible()       → itemInBar && statusItem.visible
```

## 4. demo 重构（examples/demo）

- **主窗口**：5 个固定实例（mb-1…mb-5）管理列表——每行 = 实例名 + 当前文本 + **switch toggle**（iOS 风格）；顶部 Rebuild shown / Show all / Hide all；`localStorage` 持久化 shown 状态（key `multiline-menubar:shown-v1`）。
- **菜单栏文本**：每个实例两行都显示自己的 id（`mb-1`…`mb-5`），便于沟通指认。
- **popup（点击菜单栏实例打开）= 该实例的设置面板**：文本、布局（small-top/big-bottom/equal）、字号、加粗、字体族、等宽数字、对齐、颜色（含 hex）；打开时自动填充该实例当前值，改动只作用于当前实例。
- 移除旧 UI：Menu Bar 大表单、Colors/Bold/Family/Mono/Alignment 面板、Speed 模拟、Stress 测试、Add 2nd、Greet。
- bundle：productName `MenubarLegacy160`，identifier `com.tauri.menubarlegacy160`（全新，无隐藏记忆）。

## 5. 测试 Checklist（2026-08-15）

> 测试载体：`MenubarLegacy160.app`（`com.tauri.menubarlegacy160`）
> 启动：`pkill -f multiline-menubar-example 2>/dev/null; open .../MenubarLegacy160.app`
> ⚠️ 刘海屏：测试前先清出菜单栏空间（关掉不常用的系统菜单栏项），避免「不显示」误判。

### A. 基础显示（回归）

- [ ] 启动后菜单栏显示 mb-1 … mb-5 共 5 个实例
- [ ] 每个实例两行文本都显示自己的 id（`mb-1`…`mb-5`），无截断
- [ ] 主窗口列表与菜单栏一一对应（5 行，每行 switch 为开）
- [ ] 重启 app：5 个实例仍全部显示

### B. show/hide toggle（重点回归——之前修的 bug）

- [ ] **首次 toggle**：启动后第一次关 mb-1 → 开 mb-1，mb-1 直接出现（不再「开了不显示」）
- [ ] 反复切换 mb-1 多次（≥5 次），**其他实例（尤其 mb-5）不再被挤没**
- [ ] 乱点多个实例的开关（不按顺序），实例总数不减少
- [ ] 隐藏的实例从菜单栏消失、主窗口 switch 状态一致
- [ ] Hide all → 菜单栏全空 → Show all → 5 个全部回来

### C. 持久化（localStorage）

- [ ] 关掉 mb-2、mb-4 → 退出 app → 重启 → 菜单栏只显示 mb-1/3/5，主窗口对应 switch 保持关闭
- [ ] 重启后打开隐藏实例的开关 → 正常出现
- [ ] 全部显示 → 重启 → 全部恢复显示

### D. popup 设置面板（点击菜单栏实例打开）

- [ ] 点击 mb-1 → popup 打开，标题显示「Instance settings — mb-1」，**填充的是 mb-1 的当前值**
- [ ] 改 top/bottom 文本 → Update → 菜单栏 mb-1 文本变化，其他实例不变
- [ ] 布局：切换 small-top/big-bottom ↔ 镜像 ↔ equal → Apply → mb-1 布局变化
- [ ] 字号：改 large/small 滑块（equal 布局用 single 滑块）→ Apply → 字号变化、无溢出截断
- [ ] 加粗：勾 top/bottom bold → Apply → 对应行加粗；Reset 恢复 layout 推导
- [ ] 字体族：填 `Menlo` / `PingFang SC` → Apply → 生效；清空 Reset → 回系统字体
- [ ] 等宽数字：勾选 → 数字宽度稳定（可配合连续变化的数字观察不抖动）
- [ ] 对齐：left/center/right → Apply → 行内对齐变化
- [ ] 颜色：picker/hex 改色 → Apply colors → 行变色；Reset → 回系统色（随深色模式）
- [ ] **每项改动只作用于当前实例**，其余 4 个不受影响
- [ ] 关闭 popup（Close / 点击别处）→ 再点其他实例 → popup 显示该实例的值

### E. 拖出恢复流程（stats 核心，macOS 26）

- [ ] ⌘+拖出任一实例（如 mb-3）拖出菜单栏 → **全部实例消失**（系统行为，正常）
- [ ] 系统设置-菜单栏中 MenubarLegacy160 被自动取消勾选
- [ ] 在系统设置-菜单栏重新勾选 MenubarLegacy160
- [ ] 打开 app → 点 **Rebuild shown** → 已开启实例全部恢复显示
- [ ] 若一次不显示 → 再点一次 Rebuild（stats 的「多开几下」）
- [ ] 拖出 mb-1 后，主窗口 mb-1 的 switch 仍为开（宿主可感知：`isVisible` 应返回 false，可后续加轮询提示）

### F. autosaveName 风险验证（⚠️ 待确认项）

- [ ] 反复 toggle 后，没有出现「某个实例怎么开都显示不出来」（系统按 autosaveName 记了隐藏记忆）
- [ ] 拖出恢复流程后，各实例仍能正常 toggle
- [ ] 长时间使用（多次开关 + 重启）后，无实例永久消失

### G. 边界 / 稳定性

- [ ] 快速连续点击 switch（连点 10+ 次）不崩溃、状态不混乱
- [ ] 菜单栏空间不足时（系统 item 多）实例被吞后，关掉几个系统 item 或 Hide 几个实例即恢复
- [ ] 异常时检查 `/tmp/menubar-diag.log`（记录每次 show/hide 的 itemInBar/sysVisible/item 指针）并反馈

## 6. 发布前待办

- [ ] 删除诊断日志 `diag_native` 及 `/tmp/menubar-diag.log` 引用
- [ ] 版本号确认：Cargo.toml / package.json / Cargo.lock 已同步 1.7.0
- [ ] 决定最终方案：worktree（main 1.6.0 改造）vs refactor/demo2-based（废弃？），把改动开分支提交
- [ ] demo 的 `tauri.conf.json` bundle id 是否作为正式值


## 7. Demo 界面设计

独立文档：**[docs/demo-interface-design.md](demo-interface-design.md)** —— 设置界面（5 实例 list + switch + 警示 banner）、菜单栏（右键菜单/点击开 popup）、Popup 设置面板（400×700 分组按钮/状态回显）的完整设计说明与实现文件映射。
## 8. 附录：macOS 26 Control Center 记忆机制（参考文章要点）

来源：https://b-log.to/tech-analysis/macos-26-controlcenter-trackedapplications-ghost/

- macOS 26 的菜单栏可见性由 **Control Center 维护的 `trackedApplications`** 记忆（`~/Library/Group Containers/group.com.apple.controlcenter/Library/Preferences/group.com.apple.controlcenter.plist`），不是 app 自己 defaults 能完全控制
- 系统会把**旧 app 的禁止记录/位置记忆套到新 app 身上**（bundle id + status item identity 关联）→ 「代码没变却突然不显示」的根源
- `NSStatusItem VisibleCC Item-N = 0` 是 Control Center 介入 item 可见性的痕迹（app 侧 `defaults delete` 只能清症状）
- **恢复手段**：换全新 bundle id / 清 `trackedApplications`（需完整磁盘访问 + 备份 + 重启 cfprefsd/ControlCenter/SystemUIServer）
- 结论：「看起来像 app 坏了，实际是系统记错了」——排查顺序：代码是否创建 item → app 自身 VisibleCC 是否 0 → Control Center trackedApplications 是否套了旧状态

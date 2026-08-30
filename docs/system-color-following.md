# 文字颜色"跟随系统"机制 — macOS 实现说明

> 日期：2026-08-30
> 范围：`tauri-plugin-multiline-menubar` 的菜单栏文字颜色（`default` 跟随系统亮/暗模式）
> 目的：与 Windows 兄弟插件（`tauri-plugin-multiline-taskband`）对齐设计语义，
> 并说明 macOS 侧与 Windows 侧在探测与通知两处的差异。

---

## 1. 设计决策（与 taskband 保持的两点）

1. **`default` 不落盘具体颜色，只在每次绘制时解析。**
   实例里存的是 `ColorStyle` 语义本身：`Default` 存为 `nil`（不存任何色值），
   绘制时用 `NSColor.labelColor`（AppKit 的 appearance-aware 动态色）现取。
   这样无论主题探测/通知用什么方式，只要发生一次重绘，`default` 行自动正确，
   无需遍历更新存储的颜色。

2. **主题探测不走系统"文字颜色"API。**
   Windows 侧 `GetSysColor(COLOR_BTNTEXT)` 在 Win11 任务栏上不可靠（深色任务栏
   仍返回黑色），因此改读注册表主题开关。macOS 无此问题：`NSColor.labelColor`
   本身就是动态颜色，绘制时按 `effectiveAppearance` 解析，天然对。

## 2. API 层（与 taskband 完全同形）

### 2.1 类型 — tagged enum，前后端一致

Rust（`src/models.rs:126`）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ColorStyle {
    Default,                    // { "type": "default" } — 跟随系统亮/暗
    Solid { value: String },    // { "type": "solid", "value": "#rrggbb" }
}
```

TypeScript（`guest-js/index.ts:118`）：

```ts
type ColorStyle =
  | { type: "default" }              // 跟随系统亮/暗
  | { type: "solid"; value: string } // 固定色 "#rrggbb"
```

### 2.2 命令 — 每行颜色互相独立

```ts
interface SetColorsOptions {
  id: string          // 实例 id
  top: ColorStyle     // 上行
  bottom: ColorStyle  // 下行
}
```

- 前端绑定：`setColors(options)`（`guest-js/index.ts:385`）
- 原始 invoke：`plugin:multiline-menubar|set_colors`
- Rust 入口：`commands::set_colors`（`src/commands.rs:80`）→
  `native.set_colors(id, top, bottom)`（`desktop.rs:1066`）
- 语义：每次调用同时设置两行；只想改一行时另一行原样回传当前值。

## 3. 实现机制（macOS，`src/native/multiline_menubar.mm`）

三个环节：**绘制时解析 → effectiveAppearance 探测 → KVO 事件驱动重绘**。

### 3.1 绘制时解析颜色

`drawLine:`（`multiline_menubar.mm:288`）：

```objc
NSColor *fg = color ? color : [NSColor labelColor];
```

- `solid` 行：`parse_color_style` 把 hex 解析成固定 `NSColor`，与主题无关。
- `default` 行：存 `nil`，绘制时取 `labelColor`（亮色≈黑 / 暗色≈白，与
  Windows 侧 `(0,0,0)/(255,255,255)` 语义一致）。
- 解析发生在每次 `drawRect:` 内，即每次重绘都重新求值（见 §1 决策 1）。

### 3.2 主题探测 — effectiveAppearance

`current_theme_is_dark()`（`multiline_menubar.mm`）：

```objc
NSAppearance *appearance = NSApp.effectiveAppearance;
NSString *match = [appearance bestMatchFromAppearancesWithNames:@[
  NSAppearanceNameAqua, NSAppearanceNameDarkAqua
]];
return [match isEqualToString:NSAppearanceNameDarkAqua];
```

- 探测 `NSApp.effectiveAppearance`（app 实际解析出的外观）：系统切亮/暗即翻转；
  若宿主 app 显式固定了自身外观（`NSApp.appearance` 或
  `NSRequiresAquaSystemAppearance`），跟随 app 也是正确行为。
- 注意：探测结果只用于判断"是否翻转"，不用于取色 —— 取色始终走 `labelColor`。

### 3.3 切换检测 — KVO，事件驱动

`MenubarAppearanceObserver`（`multiline_menubar.mm`）：

1. `ensure_instance` 首次创建实例时惰性安装一次
   （`[MenubarAppearanceObserver install]`，`dispatch_once` 幂等）。
2. 对 `NSApp` 以 `NSKeyValueObservingOptionInitial | New` 观察
   keyPath `effectiveAppearance`；`Initial` 使注册时立即得到首次探测值，
   等价于 Windows 侧"首次 tick 必然同步一次"。
3. `observeValueForKeyPath:` 里探测当前亮/暗，与缓存
   `g_lastDarkMode: NSNumber *`（镜像 Windows 的 `LAST_LIGHT_THEME`）比较；
   只有真正翻转（或首次，缓存为 nil）才调 `redraw_all_instances()`。
4. `redraw_all_instances()` 遍历 `g_instances` 对每个实例走
   `redraw_instance`（标脏 view + button + `updateWidth` 动 `statusItem.length`）。

效果：系统切换亮/暗后，事件立刻到达、主线程回调，所有 `default` 行立即换色，
无轮询、无感知延迟。与 Windows 侧 500ms 轮询是同一语义的不同实现。

### 3.4 数据流小结

```
系统主题切换
  → NSApp.effectiveAppearance 变化
  → (立即) MenubarAppearanceObserver KVO 回调（主线程）
  → 探测 new value，与 g_lastDarkMode 不符（真翻转）
  → redraw_all_instances() → redraw_instance × N
  → drawRect: 内 labelColor 按新外观解析 → default 行换色
  → solid 行不受影响，按存储的 hex 原样绘制
```

## 4. 与 Windows 兄弟插件对照

| 关注点 | taskband（Windows） | menubar（macOS） |
|---|---|---|
| 主题探测 | 注册表 `SystemUsesLightTheme` | `NSApp.effectiveAppearance`（`bestMatch` 含 `DarkAqua` 即深色） |
| 切换通知 | 500ms 定时器轮询 + 缓存比较 | KVO 观察 `effectiveAppearance`（事件驱动，无轮询） |
| 切换后动作 | `paint_all()` 重绘 DIB | `redraw_all_instances()` 重绘 status item |
| `default` 色值 | 浅色 `(0,0,0)` / 深色 `(255,255,255)` | `NSColor.labelColor`（动态色，绘制时解析） |
| 翻转缓存 | `LAST_LIGHT_THEME: Mutex<Option<bool>>` | `g_lastDarkMode: NSNumber *` |
| 前端 API | `setColors` / `ColorStyle` | 同名同形，共用同一套前端代码 |

结构层完全复用：`ColorStyle` tagged enum + 每行独立 + `default` 延迟到绘制时
解析。差异只在探测与通知两处，见上表。前端无需感知平台差异。

## 5. 代码位置索引

| 内容 | 位置 |
|---|---|
| `ColorStyle` 枚举定义 | `src/models.rs:126` |
| `SetColorsRequest` | `src/models.rs:137` |
| `set_colors` 命令入口 | `src/commands.rs:80` |
| JS 绑定 `setColors` / `ColorStyle` | `guest-js/index.ts:385` / `:118` |
| 绘制时解析（`labelColor`） | `src/native/multiline_menubar.mm` `drawLine:`（~:288） |
| 主题探测 `current_theme_is_dark()` | `src/native/multiline_menubar.mm`（light/dark 区段） |
| 切换检测 `MenubarAppearanceObserver` | 同上（KVO `effectiveAppearance`） |
| 翻转缓存 `g_lastDarkMode` | 同上 |
| 全量重绘 `redraw_all_instances()` | 同上 |
| 惰性安装点 | `ensure_instance`（首次创建实例时） |

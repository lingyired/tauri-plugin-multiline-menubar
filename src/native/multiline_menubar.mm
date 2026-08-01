#import <Cocoa/Cocoa.h>
#import <CoreText/CoreText.h>
#include <vector>
#include "multiline_menubar.h"

// Forward declaration for the file-local color helper defined further down
// (after the view implementation).
static NSColor *color_from_hex(NSString *hex);
static NSColor *parse_color_style(NSString *json);

// Vertical layout of the two lines. Naming is role-based rather than
// position-based: in the two asymmetric modes one line is *emphasized* (larger,
// regular weight — typically the value) and the other is de-emphasized
// (smaller, light weight — typically the label).
typedef NS_ENUM(NSInteger, MenubarLayoutMode) {
  // Small light label on top, large regular value below (Stats-style default).
  MenubarLayoutEmphasisBottom = 0,
  // Mirror image: large regular value on top, small light label below.
  MenubarLayoutEmphasisTop = 1,
  // Both lines share one size and sit vertically centered & symmetric.
  MenubarLayoutEqual = 2,
};

// Font-size ranges (points), keyed by the *role* a line plays rather than by
// its position, so the two asymmetric layouts are exact mirrors of each other.
// The de-emphasized (small) line:
static const CGFloat kMinSmallSize = 5.0;
static const CGFloat kMaxSmallSize = 11.0;
// The emphasized (large) line:
static const CGFloat kMinLargeSize = 8.0;
static const CGFloat kMaxLargeSize = 16.0;
// The equal layout shares ONE size across both lines, so the per-role ranges
// above must not be applied there (a shared value of 14 would otherwise be
// clamped to 11 on one line and stay 14 on the other, silently breaking
// symmetry). Two full-size lines have to fit a ~22pt bar, which caps it.
static const CGFloat kMinEqualSize = 5.0;
static const CGFloat kMaxEqualSize = 11.0;

static const CGFloat kDefaultSmallSize = 7.0;
static const CGFloat kDefaultLargeSize = 12.0;
static const CGFloat kDefaultEqualSize = 9.0;

static inline CGFloat clamp_size(CGFloat value, CGFloat lo, CGFloat hi) {
  return MAX(lo, MIN(hi, value));
}

// Custom NSView that renders two lines of text in the macOS menu bar.
// See `MenubarLayoutMode` for the three vertical layouts.
@interface MultilineMenubarView : NSView
@property (copy) NSString *topText;
@property (copy) NSString *bottomText;
// Effective per-line sizes, derived from the instance's role-based sizes and
// the current layout mode (see `apply_layout_sizes`).
@property CGFloat topFontSize;
@property CGFloat bottomFontSize;
@property (assign, nonatomic) MenubarLayoutMode layoutMode;
// Per-line text color. `nil` => system `textColor` (follows light/dark mode).
@property (copy, nonatomic) NSColor *topColor;
@property (copy, nonatomic) NSColor *bottomColor;
+ (NSFontWeight)weightForTop:(BOOL)isTop layout:(MenubarLayoutMode)layout;
@end

@implementation MultilineMenubarView

- (instancetype)initWithFrame:(NSRect)frame {
  self = [super initWithFrame:frame];
  if (self) {
    _topText = @"";
    _bottomText = @"";
    _topFontSize = kDefaultSmallSize;
    _bottomFontSize = kDefaultLargeSize;
    _layoutMode = MenubarLayoutEmphasisBottom;
  }
  return self;
}

/// Font weight for a line, given the layout: the emphasized line is drawn
/// regular, the de-emphasized one light. In the equal layout the weights are
/// kept distinct so the two lines stay visually separable at the same size.
+ (NSFontWeight)weightForTop:(BOOL)isTop layout:(MenubarLayoutMode)layout {
  if (layout == MenubarLayoutEmphasisTop) {
    return isTop ? NSFontWeightRegular : NSFontWeightLight;
  }
  return isTop ? NSFontWeightLight : NSFontWeightRegular;
}

- (void)drawRect:(NSRect)dirtyRect {
  [super drawRect:dirtyRect];

  CGFloat barHeight = self.bounds.size.height;  // ~22 pt menu-bar height
  CGFloat barWidth = self.bounds.size.width;

  NSFont *topFont = [NSFont
      systemFontOfSize:self.topFontSize
                weight:[MultilineMenubarView weightForTop:YES
                                                   layout:self.layoutMode]];
  NSFont *bottomFont = [NSFont
      systemFontOfSize:self.bottomFontSize
                weight:[MultilineMenubarView weightForTop:NO
                                                   layout:self.layoutMode]];

  NSRect topRect;
  NSRect bottomRect;

  if (self.layoutMode == MenubarLayoutEqual) {
    // Equal layout: one shared size, vertically centered and symmetric.
    // Use the real line height so glyphs are not clipped at larger sizes, and
    // let the gap absorb whatever vertical space is left. At the top of the
    // range the gap collapses to 0 instead of pushing text out of the bar.
    CGFloat lineH = ceil(MAX(topFont.ascender - topFont.descender,
                             bottomFont.ascender - bottomFont.descender));
    CGFloat gap = MAX(0.0, MIN(2.0, barHeight - 2.0 * lineH));
    CGFloat y0 = MAX(0.0, (barHeight - (2.0 * lineH + gap)) / 2.0);
    topRect = NSMakeRect(0, y0 + lineH + gap, barWidth, lineH);
    bottomRect = NSMakeRect(0, y0, barWidth, lineH);
  } else {
    // Asymmetric layouts (emphasis-bottom and emphasis-top). The geometry is
    // identical for both: the top line hugs the top of the bar and the bottom
    // line hugs the bottom. The two modes differ only in which line is large
    // and regular vs small and light — decided by `apply_layout_sizes` — so
    // when the top line is the large one (emphasis-top) it automatically
    // renders as the "big number on top". The MIN() keeps the top line inside
    // the bar at the largest sizes.
    CGFloat topH = self.topFontSize + 1.0;
    CGFloat bottomH = self.bottomFontSize + 1.0;
    topRect = NSMakeRect(0, MIN(12.0, MAX(0.0, barHeight - topH)), barWidth, topH);
    bottomRect = NSMakeRect(0, 1, barWidth, bottomH);
  }

  if (self.topText.length > 0) {
    [self drawLine:self.topText font:topFont rect:topRect color:self.topColor];
  }
  if (self.bottomText.length > 0) {
    [self drawLine:self.bottomText
              font:bottomFont
              rect:bottomRect
             color:self.bottomColor];
  }
}

/// Draw a single line of text, painted with the given solid `NSColor`. When
/// `color` is nil the system `textColor` is used, which follows light/dark
/// mode automatically.
- (void)drawLine:(NSString *)text
            font:(NSFont *)font
            rect:(NSRect)rect
           color:(NSColor *)color {
  if (text.length == 0) return;

  NSColor *fg = color ? color : [NSColor textColor];
  NSDictionary *attrs = @{
    NSFontAttributeName : font,
    NSForegroundColorAttributeName : fg,
  };
  [text drawWithRect:rect
              options:NSStringDrawingUsesLineFragmentOrigin
           attributes:attrs];
}

@end

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Parse a hex color (`#rgb`, `#rrggbb`, or `#rrggbbaa`) into an `NSColor`.
/// Returns nil for empty/invalid input so the caller can fall back to the
/// system text color.
static NSColor *color_from_hex(NSString *hex) {
  if (hex.length == 0) return nil;
  NSMutableString *s = [hex mutableCopy];
  [s replaceOccurrencesOfString:@"#"
                       withString:@""
                          options:0
                            range:NSMakeRange(0, s.length)];
  if (s.length == 3) {
    NSString *r = [s substringWithRange:NSMakeRange(0, 1)];
    NSString *g = [s substringWithRange:NSMakeRange(1, 1)];
    NSString *b = [s substringWithRange:NSMakeRange(2, 1)];
    s = [NSMutableString stringWithFormat:@"%@%@%@%@%@%@", r, r, g, g, b, b];
  }
  if (s.length != 6 && s.length != 8) return nil;

  unsigned int value = 0;
  if (![[NSScanner scannerWithString:s] scanHexInt:&value]) return nil;

  if (s.length == 6) {
    CGFloat r = ((value >> 16) & 0xff) / 255.0;
    CGFloat g = ((value >> 8) & 0xff) / 255.0;
    CGFloat b = (value & 0xff) / 255.0;
    return [NSColor colorWithSRGBRed:r green:g blue:b alpha:1.0];
  }
  CGFloat r = ((value >> 24) & 0xff) / 255.0;
  CGFloat g = ((value >> 16) & 0xff) / 255.0;
  CGFloat b = ((value >> 8) & 0xff) / 255.0;
  CGFloat a = (value & 0xff) / 255.0;
  return [NSColor colorWithSRGBRed:r green:g blue:b alpha:a];
}

/// Decode a color-style JSON object into a solid `NSColor`. `default` (or
/// anything unrecognized, or a missing/empty value) returns nil so the view
/// keeps using the system text color.
static NSColor *parse_color_style(NSString *json) {
  if (json == nil || json.length == 0) return nil;
  NSData *data = [json dataUsingEncoding:NSUTF8StringEncoding];
  if (data == nil) return nil;
  NSError *err = nil;
  id obj = [NSJSONSerialization JSONObjectWithData:data options:0 error:&err];
  if (err != nil || ![obj isKindOfClass:[NSDictionary class]]) return nil;

  NSDictionary *dict = (NSDictionary *)obj;
  NSString *type = dict[@"type"];
  if ([type isEqualToString:@"solid"]) {
    return color_from_hex(dict[@"value"]);
  }
  // "default" or unknown => nil (system textColor).
  return nil;
}

// ---------------------------------------------------------------------------
// Per-instance state and click/hover handler
// ---------------------------------------------------------------------------

@interface MenubarHandler : NSObject
@property (nonatomic, copy) NSString *instanceId;
@end

@interface MenubarInstance : NSObject
@property (nonatomic, strong) NSStatusItem *statusItem;
@property (nonatomic, strong) MultilineMenubarView *view;
// Sizes are stored per *role*, not per position, so switching between the two
// asymmetric layouts mirrors the item without losing either value, and
// switching in and out of the equal layout keeps its own remembered size.
@property (nonatomic, assign) CGFloat smallFontSize;
@property (nonatomic, assign) CGFloat largeFontSize;
@property (nonatomic, assign) CGFloat equalFontSize;
@property (nonatomic, assign) MenubarLayoutMode layoutMode;
@property (nonatomic, copy) NSString *instanceId;
// Context menu shown on right click. Strong, so the NSMenu handed over from
// muda stays alive independently of the Rust-side wrapper.
@property (nonatomic, strong) NSMenu *menu;
// Click/hover target. Both `NSControl.target` and `NSTrackingArea.owner` are
// weak, so the instance must own the handler or ARC deallocates it right away
// and the button silently stops responding.
@property (nonatomic, strong) MenubarHandler *handler;
- (void)updateWidth;
@end

// Global registry of menubar instances, keyed by id.
static NSMutableDictionary<NSString *, MenubarInstance *> *g_instances = nil;
static MultilineMenubarClickCallback g_clickCallback = NULL;
static MultilineMenubarHoverCallback g_hoverCallback = NULL;

@implementation MenubarInstance

- (void)updateWidth {
  if (!self.statusItem || !self.view) {
    return;
  }

  NSString *topText = self.view.topText;
  NSString *bottomText = self.view.bottomText;

  // Measure with the same fonts `drawRect:` will use, otherwise the item is
  // sized for the wrong weight/size and the text gets clipped.
  NSDictionary *topAttributes = @{
    NSFontAttributeName : [NSFont
        systemFontOfSize:self.view.topFontSize
                  weight:[MultilineMenubarView weightForTop:YES
                                                     layout:self.layoutMode]],
  };
  NSDictionary *bottomAttributes = @{
    NSFontAttributeName : [NSFont
        systemFontOfSize:self.view.bottomFontSize
                  weight:[MultilineMenubarView weightForTop:NO
                                                     layout:self.layoutMode]],
  };

  NSSize topSize = [topText sizeWithAttributes:topAttributes];
  NSSize bottomSize = [bottomText sizeWithAttributes:bottomAttributes];
  CGFloat contentWidth = MAX(topSize.width, bottomSize.width);
  // Add horizontal padding (4 pt on each side) and enforce a minimum width.
  CGFloat width = MAX(contentWidth + 8, 32);

  NSRect frame = self.view.frame;
  frame.size.width = width;
  self.view.frame = frame;

  self.statusItem.length = width;
}

@end

@implementation MenubarHandler

- (void)handleClick:(id)sender {
  (void)sender;
  MenubarInstance *inst = g_instances[self.instanceId];
  if (!inst || !inst.statusItem || !inst.statusItem.button) {
    return;
  }

  NSStatusBarButton *button = inst.statusItem.button;
  NSRect bounds = [button bounds];

  NSEvent *event = [NSApp currentEvent];
  BOOL isRight = NO;
  if (event != nil) {
    NSEventType type = event.type;
    if (type == NSEventTypeRightMouseDown || type == NSEventTypeRightMouseUp) {
      isRight = YES;
    }
  }

  // Always report the click to Rust first (with the on-screen rect and the
  // cursor position) so the JS event fires even when a menu is attached.
  // Rust decides whether a left click also toggles the popup window.
  NSRect frameInWindow = [button convertRect:bounds toView:nil];
  NSRect frameOnScreen = [[button window] convertRectToScreen:frameInWindow];
  NSPoint cursor = [NSEvent mouseLocation];

  if (g_clickCallback) {
    g_clickCallback([self.instanceId UTF8String], isRight ? "right" : "left",
                   (double)frameOnScreen.origin.x,
                   (double)frameOnScreen.origin.y,
                   (double)frameOnScreen.size.width,
                   (double)frameOnScreen.size.height,
                   (double)cursor.x,
                   (double)cursor.y);
  }

  // A right click pops the attached menu up directly under the status item.
  // `popUpMenuPositioningItem:` runs a modal tracking loop and returns once
  // the menu is dismissed, so the manual highlight brackets it correctly.
  if (isRight && inst.menu != nil) {
    NSPoint location = [button isFlipped] ? NSMakePoint(0, NSMaxY(bounds) + 4)
                                          : NSMakePoint(0, NSMinY(bounds) - 4);
    [button highlight:YES];
    [inst.menu popUpMenuPositioningItem:nil atLocation:location inView:button];
    [button highlight:NO];
  }
}

- (void)mouseEntered:(NSEvent *)event {
  (void)event;
  if (g_hoverCallback) {
    g_hoverCallback([self.instanceId UTF8String], "enter");
  }
}

- (void)mouseExited:(NSEvent *)event {
  (void)event;
  if (g_hoverCallback) {
    g_hoverCallback([self.instanceId UTF8String], "leave");
  }
}

@end

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Push the instance's role-based sizes onto the view as concrete per-line
/// sizes for the current layout. This is the single place that decides which
/// line is the emphasized one.
static void apply_layout_sizes(MenubarInstance *inst) {
  CGFloat top;
  CGFloat bottom;
  switch (inst.layoutMode) {
    case MenubarLayoutEqual:
      top = inst.equalFontSize;
      bottom = inst.equalFontSize;
      break;
    case MenubarLayoutEmphasisTop:
      top = inst.largeFontSize;
      bottom = inst.smallFontSize;
      break;
    case MenubarLayoutEmphasisBottom:
    default:
      top = inst.smallFontSize;
      bottom = inst.largeFontSize;
      break;
  }
  inst.view.layoutMode = inst.layoutMode;
  inst.view.topFontSize = top;
  inst.view.bottomFontSize = bottom;
}

/// Force the status item to repaint. Marking the hosting `NSStatusBarButton`
/// dirty as well matters when the change is triggered from another window
/// (e.g. the popup): the custom view alone does not always get a display pass.
static void redraw_instance(MenubarInstance *inst) {
  [inst.view setNeedsDisplay:YES];
  if (inst.statusItem.button) {
    inst.statusItem.button.needsDisplay = YES;
  }
  [inst updateWidth];
}

static MenubarInstance *ensure_instance(NSString *key) {
  if (!g_instances) {
    g_instances = [NSMutableDictionary dictionary];
  }
  MenubarInstance *inst = g_instances[key];
  if (inst) {
    return inst;
  }

  inst = [[MenubarInstance alloc] init];
  inst.instanceId = key;
  inst.smallFontSize = kDefaultSmallSize;
  inst.largeFontSize = kDefaultLargeSize;
  inst.equalFontSize = kDefaultEqualSize;
  inst.layoutMode = MenubarLayoutEmphasisBottom;
  inst.menu = nil;

  inst.statusItem = [[NSStatusBar systemStatusBar]
      statusItemWithLength:NSVariableStatusItemLength];
  inst.statusItem.button.title = @"";
  inst.statusItem.button.image = nil;

  inst.view = [[MultilineMenubarView alloc] initWithFrame:NSMakeRect(0, 0, 60, 22)];
  inst.view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  apply_layout_sizes(inst);
  [inst.statusItem.button addSubview:inst.view];

  // Per-instance click/hover handler. Retained by the instance (see the
  // property declaration) because AppKit only holds it weakly.
  MenubarHandler *handler = [[MenubarHandler alloc] init];
  handler.instanceId = key;
  inst.handler = handler;
  inst.statusItem.button.target = handler;
  inst.statusItem.button.action = @selector(handleClick:);
  [inst.statusItem.button
      sendActionOn:(NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown)];

  // Hover tracking area. `NSTrackingInVisibleRect` makes AppKit keep the rect
  // in sync with the button, which matters because the status item is resized
  // every time the text or font size changes.
  NSTrackingArea *tracking = [[NSTrackingArea alloc]
      initWithRect:inst.statusItem.button.bounds
           options:(NSTrackingMouseEnteredAndExited | NSTrackingActiveAlways |
                    NSTrackingInVisibleRect)
             owner:handler
          userInfo:nil];
  [inst.statusItem.button addTrackingArea:tracking];

  g_instances[key] = inst;
  return inst;
}

static NSString *key_from_id(const char *id) {
  return [NSString stringWithUTF8String:(id ? id : "")];
}

/// Run a block on the main thread and wait for it.
///
/// The click and hover callbacks are invoked *from* the main thread, and the
/// Rust side calls back into `get_rect` / `is_visible` while handling them.
/// A plain `dispatch_sync` to the main queue would deadlock in that case, so
/// run the block inline when we are already there.
static void run_on_main_sync(dispatch_block_t block) {
  if ([NSThread isMainThread]) {
    block();
  } else {
    dispatch_sync(dispatch_get_main_queue(), block);
  }
}

// ---------------------------------------------------------------------------
// C API
// ---------------------------------------------------------------------------

/// Show or hide a status item without releasing its slot in the bar.
///
/// `button.hidden` only hides the view but leaves a reserved (blank) gap in
/// the menu bar, because the item is still part of the system's status items.
/// `NSStatusItem.visible` (macOS 13+) removes the item from display AND frees
/// the space, while keeping it in the array so its position is preserved when
/// shown again. Fall back to `hidden` on older systems (where the gap remains
/// — the only alternative pre-13 is removeStatusItem, which loses position).
static void set_instance_visible(MenubarInstance *inst, BOOL visible) {
  if (!inst || !inst.statusItem) return;
  if (@available(macOS 13.0, *)) {
    inst.statusItem.visible = visible;
  } else {
    inst.statusItem.button.hidden = !visible;
  }
}

/// Current visibility, matching `set_instance_visible`.
static BOOL instance_is_visible(MenubarInstance *inst) {
  if (!inst || !inst.statusItem) return NO;
  if (@available(macOS 13.0, *)) {
    return inst.statusItem.visible;
  }
  return !inst.statusItem.button.hidden;
}

void multiline_menubar_create(const char *id) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    set_instance_visible(inst, YES);
  });
}

void multiline_menubar_destroy(const char *id) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = g_instances[key];
    if (inst) {
      [[NSStatusBar systemStatusBar] removeStatusItem:inst.statusItem];
      [g_instances removeObjectForKey:key];
    }
  });
}

void multiline_menubar_show(const char *id) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    set_instance_visible(inst, YES);
  });
}

void multiline_menubar_hide(const char *id) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = g_instances[key];
    if (inst) {
      set_instance_visible(inst, NO);
    }
  });
}

void multiline_menubar_set_text(const char *id, const char *top,
                                const char *bottom) {
  NSString *key = key_from_id(id);
  NSString *topText = top ? [NSString stringWithUTF8String:top] : @"";
  NSString *bottomText = bottom ? [NSString stringWithUTF8String:bottom] : @"";
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    inst.view.topText = topText;
    inst.view.bottomText = bottomText;
    [inst.view setNeedsDisplay:YES];
    [inst updateWidth];
  });
}

/// Set the font sizes of the two lines. `top_size`/`bottom_size` always refer
/// to the top and bottom line *as displayed*, so their meaning depends on the
/// layout: in `emphasis-top` the top line is the large one, in
/// `emphasis-bottom` it is the small one, and in `equal` both lines share
/// `top_size` (`bottom_size` is ignored). Values are clamped to the range of
/// the role the line currently plays.
void multiline_menubar_set_style(const char *id, double top_size,
                                 double bottom_size) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    switch (inst.layoutMode) {
      case MenubarLayoutEqual:
        inst.equalFontSize =
            clamp_size((CGFloat)top_size, kMinEqualSize, kMaxEqualSize);
        break;
      case MenubarLayoutEmphasisTop:
        inst.largeFontSize =
            clamp_size((CGFloat)top_size, kMinLargeSize, kMaxLargeSize);
        inst.smallFontSize =
            clamp_size((CGFloat)bottom_size, kMinSmallSize, kMaxSmallSize);
        break;
      case MenubarLayoutEmphasisBottom:
      default:
        inst.smallFontSize =
            clamp_size((CGFloat)top_size, kMinSmallSize, kMaxSmallSize);
        inst.largeFontSize =
            clamp_size((CGFloat)bottom_size, kMinLargeSize, kMaxLargeSize);
        break;
    }
    apply_layout_sizes(inst);
    redraw_instance(inst);
  });
}

/// Switch the vertical layout of a menubar instance.
///
/// `layout`: 0 = emphasis-bottom (default, small light label on top / large
/// value below), 1 = emphasis-top (the vertical mirror), 2 = equal (both lines
/// share one size, vertically centered and symmetric).
///
/// Sizes are stored per role, so switching layouts never loses a value: the
/// two asymmetric layouts swap which line is large, and the equal layout keeps
/// its own remembered size (9 pt by default). Out-of-range values are
/// impossible here, so the item never renders in a half-migrated state.
void multiline_menubar_set_layout(const char *id, int layout) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    int mode = layout;
    if (mode < MenubarLayoutEmphasisBottom || mode > MenubarLayoutEqual) {
      mode = MenubarLayoutEmphasisBottom;
    }
    inst.layoutMode = (MenubarLayoutMode)mode;
    apply_layout_sizes(inst);
    redraw_instance(inst);
  });
}

void multiline_menubar_set_color(const char *id, const char *top_json,
                                 const char *bottom_json) {
  NSString *key = key_from_id(id);
  NSString *topJson = top_json ? [NSString stringWithUTF8String:top_json] : @"";
  NSString *bottomJson =
      bottom_json ? [NSString stringWithUTF8String:bottom_json] : @"";
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);

    NSColor *topColor = parse_color_style(topJson);
    NSColor *botColor = parse_color_style(bottomJson);

    inst.view.topColor = topColor;
    inst.view.bottomColor = botColor;

    // Force a redraw. Unlike set_text/set_style, we don't change the text, so
    // `updateWidth` (which mutates the status item length/frame) would be a
    // no-op for the geometry — but assigning `statusItem.length` still makes
    // NSStatusBarButton re-lay-out and repaint the menu-bar slot. Plain
    // `[view setNeedsDisplay:]` alone is NOT enough: the status-bar host does
    // not reliably propagate a subview's dirty flag, so the icon would never
    // repaint and the new color would be invisible.
    [inst.view setNeedsDisplay:YES];
    inst.statusItem.button.needsDisplay = YES;
    [inst updateWidth];
  });
}

void multiline_menubar_set_tooltip(const char *id, const char *tooltip) {
  NSString *key = key_from_id(id);
  NSString *tip = tooltip ? [NSString stringWithUTF8String:tooltip] : @"";
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    inst.statusItem.button.toolTip = tip;
  });
}

void multiline_menubar_set_menu(const char *id, void *ns_menu) {
  NSString *key = key_from_id(id);
  // `__bridge` does not transfer ownership; assigning to the strong `menu`
  // property is what retains the NSMenu under ARC.
  NSMenu *menu = ns_menu ? (__bridge NSMenu *)ns_menu : nil;
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    inst.menu = menu;
  });
}

int multiline_menubar_get_rect(const char *id, double *x, double *y,
                               double *width, double *height) {
  __block int result = 0;
  NSString *key = key_from_id(id);
  run_on_main_sync(^{
    MenubarInstance *inst = g_instances[key];
    if (!inst || !inst.statusItem.button) {
      result = 0;
      return;
    }
    NSView *button = inst.statusItem.button;
    NSRect frameInWindow = [button convertRect:button.bounds toView:nil];
    NSRect frameOnScreen = [[button window] convertRectToScreen:frameInWindow];
    if (x) *x = frameOnScreen.origin.x;
    if (y) *y = frameOnScreen.origin.y;
    if (width) *width = frameOnScreen.size.width;
    if (height) *height = frameOnScreen.size.height;
    result = 1;
  });
  return result;
}

int multiline_menubar_is_visible(const char *id) {
  __block int result = 0;
  NSString *key = key_from_id(id);
  run_on_main_sync(^{
    MenubarInstance *inst = g_instances[key];
    result = instance_is_visible(inst) ? 1 : 0;
  });
  return result;
}

void multiline_menubar_set_click_handler(MultilineMenubarClickCallback callback) {
  g_clickCallback = callback;
}

void multiline_menubar_set_hover_handler(MultilineMenubarHoverCallback callback) {
  g_hoverCallback = callback;
}

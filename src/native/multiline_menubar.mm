#import <Cocoa/Cocoa.h>
#import <CoreText/CoreText.h>
#include <vector>
#include "multiline_menubar.h"

// Forward declarations for the file-local color/gradient helpers defined
// further down (after the view implementation).
static NSColor *color_from_hex(NSString *hex);
static void parse_color_style(NSString *json, NSColor **outColor,
                              NSGradient **outGradient, CGFloat *outAngle);
static NSBezierPath *textPathForString(NSString *string, NSFont *font);

// Supported font-size range (points). The menu bar is only ~22pt tall, so the
// two lines must stay small enough to avoid overlapping.
static const CGFloat kMinTopSize = 5.0;
static const CGFloat kMaxTopSize = 11.0;
static const CGFloat kMinBottomSize = 8.0;
static const CGFloat kMaxBottomSize = 16.0;

// Custom NSView that renders two lines of text in the macOS menu bar.
// Layout mirrors the reference Mini.swift widget from Stats:
//   - top line: small label (default 7 pt, light weight)
//   - bottom line: larger value (default 12 pt, regular weight)
@interface MultilineMenubarView : NSView
@property (copy) NSString *topText;
@property (copy) NSString *bottomText;
@property CGFloat topFontSize;
@property CGFloat bottomFontSize;
// Per-line text paint. `nil` color + `nil` gradient => system `textColor`.
// A non-nil gradient takes precedence over the solid color.
@property (copy, nonatomic) NSColor *topColor;
@property (strong, nonatomic) NSGradient *topGradient;
@property CGFloat topGradientAngle;
@property (copy, nonatomic) NSColor *bottomColor;
@property (strong, nonatomic) NSGradient *bottomGradient;
@property CGFloat bottomGradientAngle;
@end

@implementation MultilineMenubarView

- (instancetype)initWithFrame:(NSRect)frame {
  self = [super initWithFrame:frame];
  if (self) {
    _topText = @"";
    _bottomText = @"";
    _topFontSize = 7.0;
    _bottomFontSize = 12.0;
  }
  return self;
}

- (void)drawRect:(NSRect)dirtyRect {
  [super drawRect:dirtyRect];

  // Top label
  if (self.topText.length > 0) {
    NSFont *topFont = [NSFont systemFontOfSize:self.topFontSize
                                         weight:NSFontWeightLight];
    NSRect topRect = NSMakeRect(0, 12, self.bounds.size.width, self.topFontSize + 1);
    [self drawLine:self.topText
              font:topFont
              rect:topRect
             color:self.topColor
         gradient:self.topGradient
             angle:self.topGradientAngle];
  }

  // Bottom value
  if (self.bottomText.length > 0) {
    NSFont *bottomFont = [NSFont systemFontOfSize:self.bottomFontSize
                                           weight:NSFontWeightRegular];
    NSRect bottomRect = NSMakeRect(0, 1, self.bounds.size.width, self.bottomFontSize + 1);
    [self drawLine:self.bottomText
              font:bottomFont
              rect:bottomRect
             color:self.bottomColor
         gradient:self.bottomGradient
             angle:self.bottomGradientAngle];
  }
}

/// Draw a single line of text, painted either with a solid `NSColor` or, when
/// `gradient` is non-nil, by clipping to the glyph outline and filling it with
/// the `NSGradient`. With neither set, the system `textColor` is used (which
/// follows light/dark mode automatically).
- (void)drawLine:(NSString *)text
            font:(NSFont *)font
            rect:(NSRect)rect
           color:(NSColor *)color
        gradient:(NSGradient *)gradient
           angle:(CGFloat)angle {
  if (text.length == 0) return;

  if (gradient != nil) {
    NSBezierPath *path = textPathForString(text, font);
    if (path != nil) {
      // Position the glyph path: its origin is the baseline, so translate it
      // to the line's baseline (rect bottom + font ascender).
      CGFloat baseline = rect.origin.y + font.ascender;
      NSAffineTransform *t = [NSAffineTransform transform];
      [t translateXBy:rect.origin.x yBy:baseline];
      [path transformUsingAffineTransform:t];

      // Fill a rect that tightly bounds the glyphs (with a small margin so the
      // baseline approximation never clips a sliver off the top/bottom).
      NSRect fillRect = NSInsetRect(path.bounds, 0, -2);
      [NSGraphicsContext saveGraphicsState];
      [path addClip];
      [gradient drawInRect:fillRect angle:angle];
      [NSGraphicsContext restoreGraphicsState];
      return;
    }
    // Empty path (no renderable glyphs) — fall through to solid paint.
  }

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
// Color / gradient helpers
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

/// Decode a color-style JSON object into a solid `NSColor` and/or an
/// `NSGradient`. `default` (or anything unrecognized) leaves both nil so the
/// view keeps using the system text color.
static void parse_color_style(NSString *json, NSColor **outColor,
                              NSGradient **outGradient, CGFloat *outAngle) {
  *outColor = nil;
  *outGradient = nil;
  *outAngle = 90.0;
  if (json == nil || json.length == 0) return;

  NSData *data = [json dataUsingEncoding:NSUTF8StringEncoding];
  if (data == nil) return;
  NSError *err = nil;
  id obj = [NSJSONSerialization JSONObjectWithData:data options:0 error:&err];
  if (err != nil || ![obj isKindOfClass:[NSDictionary class]]) return;

  NSDictionary *dict = (NSDictionary *)obj;
  NSString *type = dict[@"type"];
  if ([type isEqualToString:@"solid"]) {
    *outColor = color_from_hex(dict[@"value"]);
  } else if ([type isEqualToString:@"gradient"]) {
    NSColor *from = color_from_hex(dict[@"from"]);
    NSColor *to = color_from_hex(dict[@"to"]);
    if (from != nil && to != nil) {
      *outGradient = [[NSGradient alloc] initWithStartingColor:from
                                                    endingColor:to];
      NSNumber *angle = dict[@"angle"];
      if (angle != nil) *outAngle = [angle doubleValue];
    }
  }
  // "default" or unknown => leave both nil (system textColor).
}

/// Build an `NSBezierPath` outlining the glyphs of `string` using `font`.
/// The path's origin is the text baseline (y = 0); glyphs extend upward.
static NSBezierPath *textPathForString(NSString *string, NSFont *font) {
  if (string.length == 0) return nil;
  CTFontRef ctFont = (__bridge CTFontRef)font;

  CFAttributedStringRef attr = (__bridge CFAttributedStringRef)
      [[NSAttributedString alloc] initWithString:string
                                      attributes:@{NSFontAttributeName : font}];
  CTLineRef line = CTLineCreateWithAttributedString(attr);
  CGMutablePathRef letters = CGPathCreateMutable();

  CFArrayRef runs = CTLineGetGlyphRuns(line);
  for (CFIndex i = 0; i < CFArrayGetCount(runs); i++) {
    CTRunRef run = (CTRunRef)CFArrayGetValueAtIndex(runs, i);
    CFIndex count = CTRunGetGlyphCount(run);
    if (count == 0) continue;
    std::vector<CGGlyph> glyphs(count);
    std::vector<CGPoint> positions(count);
    CTRunGetGlyphs(run, CFRangeMake(0, count), glyphs.data());
    CTRunGetPositions(run, CFRangeMake(0, count), positions.data());
    for (CFIndex j = 0; j < count; j++) {
      CGPathRef glyphPath = CTFontCreatePathForGlyph(ctFont, glyphs[j], NULL);
      if (glyphPath) {
        CGAffineTransform t =
            CGAffineTransformMakeTranslation(positions[j].x, positions[j].y);
        CGPathAddPath(letters, &t, glyphPath);
        CGPathRelease(glyphPath);
      }
    }
  }
  CFRelease(line);

  NSBezierPath *path = [NSBezierPath bezierPathWithCGPath:letters];
  CGPathRelease(letters);
  return path;
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
@property (nonatomic, assign) CGFloat topFontSize;
@property (nonatomic, assign) CGFloat bottomFontSize;
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

  NSDictionary *topAttributes = @{
    NSFontAttributeName : [NSFont systemFontOfSize:self.topFontSize weight:NSFontWeightLight],
  };
  NSDictionary *bottomAttributes = @{
    NSFontAttributeName : [NSFont systemFontOfSize:self.bottomFontSize weight:NSFontWeightRegular],
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
  inst.topFontSize = 7.0;
  inst.bottomFontSize = 12.0;
  inst.menu = nil;

  inst.statusItem = [[NSStatusBar systemStatusBar]
      statusItemWithLength:NSVariableStatusItemLength];
  inst.statusItem.button.title = @"";
  inst.statusItem.button.image = nil;

  inst.view = [[MultilineMenubarView alloc] initWithFrame:NSMakeRect(0, 0, 60, 22)];
  inst.view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  inst.view.topFontSize = inst.topFontSize;
  inst.view.bottomFontSize = inst.bottomFontSize;
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

void multiline_menubar_set_style(const char *id, double top_size,
                                 double bottom_size) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    inst.topFontSize = MAX(kMinTopSize, MIN(kMaxTopSize, (CGFloat)top_size));
    inst.bottomFontSize =
        MAX(kMinBottomSize, MIN(kMaxBottomSize, (CGFloat)bottom_size));
    inst.view.topFontSize = inst.topFontSize;
    inst.view.bottomFontSize = inst.bottomFontSize;
    [inst.view setNeedsDisplay:YES];
    [inst updateWidth];
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

    NSColor *topColor = nil;
    NSGradient *topGrad = nil;
    CGFloat topAngle = 90;
    NSColor *botColor = nil;
    NSGradient *botGrad = nil;
    CGFloat botAngle = 90;

    parse_color_style(topJson, &topColor, &topGrad, &topAngle);
    parse_color_style(bottomJson, &botColor, &botGrad, &botAngle);

    inst.view.topColor = topColor;
    inst.view.topGradient = topGrad;
    inst.view.topGradientAngle = topAngle;
    inst.view.bottomColor = botColor;
    inst.view.bottomGradient = botGrad;
    inst.view.bottomGradientAngle = botAngle;

    [inst.view setNeedsDisplay:YES];
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

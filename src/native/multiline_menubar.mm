#import <Cocoa/Cocoa.h>
#include "multiline_menubar.h"

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

  // NSColor.textColor automatically follows light/dark mode and
  // accessibility settings such as high contrast.
  NSColor *textColor = [NSColor textColor];

  // Top label
  if (self.topText.length > 0) {
    NSDictionary *topAttributes = @{
      NSFontAttributeName : [NSFont systemFontOfSize:self.topFontSize
                                              weight:NSFontWeightLight],
      NSForegroundColorAttributeName : textColor,
    };
    NSRect topRect = NSMakeRect(0, 12, self.bounds.size.width, self.topFontSize + 1);
    [self.topText drawWithRect:topRect
                       options:NSStringDrawingUsesLineFragmentOrigin
                    attributes:topAttributes];
  }

  // Bottom value
  NSDictionary *bottomAttributes = @{
    NSFontAttributeName : [NSFont systemFontOfSize:self.bottomFontSize
                                            weight:NSFontWeightRegular],
    NSForegroundColorAttributeName : textColor,
  };
  NSRect bottomRect = NSMakeRect(0, 1, self.bounds.size.width, self.bottomFontSize + 1);
  [self.bottomText drawWithRect:bottomRect
                        options:NSStringDrawingUsesLineFragmentOrigin
                     attributes:bottomAttributes];
}

@end

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

void multiline_menubar_create(const char *id) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = ensure_instance(key);
    inst.statusItem.button.hidden = NO;
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
    inst.statusItem.button.hidden = NO;
  });
}

void multiline_menubar_hide(const char *id) {
  NSString *key = key_from_id(id);
  dispatch_async(dispatch_get_main_queue(), ^{
    MenubarInstance *inst = g_instances[key];
    if (inst) {
      inst.statusItem.button.hidden = YES;
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
    result = (inst && !inst.statusItem.button.hidden) ? 1 : 0;
  });
  return result;
}

void multiline_menubar_set_click_handler(MultilineMenubarClickCallback callback) {
  g_clickCallback = callback;
}

void multiline_menubar_set_hover_handler(MultilineMenubarHoverCallback callback) {
  g_hoverCallback = callback;
}

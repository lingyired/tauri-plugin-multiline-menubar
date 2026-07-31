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

// Forward declaration so the helper functions below can call the width helper.
@interface MultilineMenubarView (Width)
+ (void)multiline_menubar_update_width;
@end

static NSStatusItem *g_statusItem = nil;
static MultilineMenubarView *g_menubarView = nil;
static CGFloat g_topFontSize = 7.0;
static CGFloat g_bottomFontSize = 12.0;

static void ensure_status_item(void) {
  if (g_statusItem) {
    return;
  }

  g_statusItem = [[NSStatusBar systemStatusBar] statusItemWithLength:NSVariableStatusItemLength];
  // Hide the default title so the custom view is the only visible content.
  g_statusItem.button.title = @"";
  g_statusItem.button.image = nil;

  g_menubarView = [[MultilineMenubarView alloc] initWithFrame:NSMakeRect(0, 0, 60, 22)];
  g_menubarView.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
  g_menubarView.topFontSize = g_topFontSize;
  g_menubarView.bottomFontSize = g_bottomFontSize;
  [g_statusItem.button addSubview:g_menubarView];
}

void multiline_menubar_show(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    ensure_status_item();
    g_statusItem.button.hidden = NO;
  });
}

void multiline_menubar_hide(void) {
  dispatch_async(dispatch_get_main_queue(), ^{
    if (g_statusItem) {
      g_statusItem.button.hidden = YES;
    }
  });
}

void multiline_menubar_set_text(const char *top, const char *bottom) {
  NSString *topText = top ? [NSString stringWithUTF8String:top] : @"";
  NSString *bottomText = bottom ? [NSString stringWithUTF8String:bottom] : @"";

  dispatch_async(dispatch_get_main_queue(), ^{
    ensure_status_item();

    g_menubarView.topText = topText;
    g_menubarView.bottomText = bottomText;
    [g_menubarView setNeedsDisplay:YES];

    [MultilineMenubarView multiline_menubar_update_width];
  });
}

void multiline_menubar_set_style(double top_size, double bottom_size) {
  dispatch_async(dispatch_get_main_queue(), ^{
    g_topFontSize = MAX(kMinTopSize, MIN(kMaxTopSize, (CGFloat)top_size));
    g_bottomFontSize = MAX(kMinBottomSize, MIN(kMaxBottomSize, (CGFloat)bottom_size));

    if (g_menubarView) {
      g_menubarView.topFontSize = g_topFontSize;
      g_menubarView.bottomFontSize = g_bottomFontSize;
      [g_menubarView setNeedsDisplay:YES];
      [MultilineMenubarView multiline_menubar_update_width];
    }
  });
}

// Recompute the status item width from the current text and font sizes.
@implementation MultilineMenubarView (Width)
+ (void)multiline_menubar_update_width {
  if (!g_statusItem || !g_menubarView) {
    return;
  }

  NSString *topText = g_menubarView.topText;
  NSString *bottomText = g_menubarView.bottomText;

  NSDictionary *topAttributes = @{
    NSFontAttributeName : [NSFont systemFontOfSize:g_topFontSize weight:NSFontWeightLight],
  };
  NSDictionary *bottomAttributes = @{
    NSFontAttributeName : [NSFont systemFontOfSize:g_bottomFontSize weight:NSFontWeightRegular],
  };

  NSSize topSize = [topText sizeWithAttributes:topAttributes];
  NSSize bottomSize = [bottomText sizeWithAttributes:bottomAttributes];
  CGFloat contentWidth = MAX(topSize.width, bottomSize.width);
  // Add horizontal padding (4 pt on each side) and enforce a minimum width.
  CGFloat width = MAX(contentWidth + 8, 32);

  NSRect frame = g_menubarView.frame;
  frame.size.width = width;
  g_menubarView.frame = frame;

  g_statusItem.length = width;
}
@end

int multiline_menubar_is_visible(void) {
  return (g_statusItem && !g_statusItem.button.hidden) ? 1 : 0;
}

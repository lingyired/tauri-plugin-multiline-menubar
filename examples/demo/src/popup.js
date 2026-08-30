const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// Which menubar instance opened this popup. Set when the plugin emits the
// "open" event; used so Update/Close target the right instance in the
// multi-instance case.
let currentInstanceId = null;

window.addEventListener("DOMContentLoaded", () => {
  const headerEl = document.querySelector("#popup-header");
  const topEl = document.querySelector("#popup-top");
  const bottomEl = document.querySelector("#popup-bottom");
  const layoutBottomEl = document.querySelector("#popup-layout-bottom");
  const layoutTopEl = document.querySelector("#popup-layout-top");
  const layoutEqualEl = document.querySelector("#popup-layout-equal");
  const sizeAsymEl = document.querySelector("#popup-size-asym");
  const sizeEqualEl = document.querySelector("#popup-size-equal");
  const largeSizeEl = document.querySelector("#popup-large-size");
  const smallSizeEl = document.querySelector("#popup-small-size");
  const largeSizeValueEl = document.querySelector("#popup-large-size-value");
  const smallSizeValueEl = document.querySelector("#popup-small-size-value");
  const sizeEl = document.querySelector("#popup-size");
  const sizeValueEl = document.querySelector("#popup-size-value");
  const topBoldEl = document.querySelector("#popup-top-bold");
  const bottomBoldEl = document.querySelector("#popup-bottom-bold");
  const topFamilyEl = document.querySelector("#popup-top-family");
  const bottomFamilyEl = document.querySelector("#popup-bottom-family");
  const topMonoEl = document.querySelector("#popup-top-monospaced");
  const bottomMonoEl = document.querySelector("#popup-bottom-monospaced");
  const topAlignEl = document.querySelector("#popup-top-align");
  const bottomAlignEl = document.querySelector("#popup-bottom-align");
  const topColorEl = document.querySelector("#popup-top-color");
  const topHexEl = document.querySelector("#popup-top-hex");
  const bottomColorEl = document.querySelector("#popup-bottom-color");
  const bottomHexEl = document.querySelector("#popup-bottom-hex");
  const topSolidRowEl = document.querySelector("#popup-top-solid-row");
  const bottomSolidRowEl = document.querySelector("#popup-bottom-solid-row");
  const topCmodeEl = document.querySelector("#popup-top-cmode");
  const bottomCmodeEl = document.querySelector("#popup-bottom-cmode");

  // Per-line color mode: "default" = follow the system light/dark theme,
  // "solid" = a fixed hex color. Mirrors the ColorStyle tagged union.
  const colorMode = { top: "default", bottom: "default" };

  // Highlight the active System/Custom button in a segmented group.
  const setColorModePressed = (groupEl, mode) => {
    for (const btn of groupEl.querySelectorAll("button[data-cmode]")) {
      btn.setAttribute("aria-pressed", String(btn.dataset.cmode === mode));
    }
  };

  // Sync one line's color section to a ColorStyle object from the plugin.
  const fillColorStyle = (line, color) => {
    const mode = color && color.type === "solid" ? "solid" : "default";
    colorMode[line] = mode;
    const groupEl = line === "top" ? topCmodeEl : bottomCmodeEl;
    const rowEl = line === "top" ? topSolidRowEl : bottomSolidRowEl;
    setColorModePressed(groupEl, mode);
    rowEl.hidden = mode !== "solid";
    if (mode === "solid" && color && /^#[0-9a-fA-F]{6}$/.test(color.value)) {
      const colorEl = line === "top" ? topColorEl : bottomColorEl;
      const hexEl = line === "top" ? topHexEl : bottomHexEl;
      colorEl.value = color.value;
      hexEl.value = color.value;
    }
  };

  // Picking a color in the native picker auto-fills the hex field (which the
  // user may still edit by hand); the hex field is what gets applied.
  topColorEl.addEventListener("input", () => {
    topHexEl.value = topColorEl.value;
  });
  bottomColorEl.addEventListener("input", () => {
    bottomHexEl.value = bottomColorEl.value;
  });

  // Role-based font sizes, mirrored from the native side. The two asymmetric
  // layouts are exact vertical mirrors of one another, so we keep the
  // *emphasized* (large) and *de-emphasized* (small) sizes as separate values
  // rather than per-position sizes. Switching layouts then just moves which
  // line is large, without ever losing a value.
  let curSmall = 7;
  let curLarge = 12;
  let curEqual = 9;

  // Which layout radio is currently selected: 0 = emphasis-bottom (default),
  // 1 = emphasis-top, 2 = equal.
  const currentLayoutValue = () => {
    if (layoutTopEl.checked) return 1;
    if (layoutEqualEl.checked) return 2;
    return 0;
  };

  // Compute the *displayed* top/bottom sizes for a layout from the role-based
  // values. The native `set_font_sizes` API always takes displayed positions,
  // so this is the single place that maps roles -> positions.
  const displayedForLayout = (l) => {
    if (l === 2) return [curEqual, curEqual];
    if (l === 1) return [curLarge, curSmall]; // top large, bottom small
    return [curSmall, curLarge]; // top small, bottom large (default)
  };

  // Sync the visible slider group + values to the role-based state.
  const seedSizeSliders = (l) => {
    if (l === 2) {
      sizeEl.value = curEqual;
      sizeValueEl.textContent = curEqual;
    } else {
      largeSizeEl.value = curLarge;
      largeSizeValueEl.textContent = curLarge;
      smallSizeEl.value = curSmall;
      smallSizeValueEl.textContent = curSmall;
    }
  };

  // The plugin sends the instance id and its current text whenever the popup
  // opens. Re-render so each instance shows its own content.
  listen("multiline-menubar://popup//open", (event) => {
    const {
      id,
      top,
      bottom,
      topSize,
      bottomSize,
      layout,
      topBold,
      bottomBold,
      topFontFamily,
      bottomFontFamily,
      topMonospaced,
      bottomMonospaced,
      topAlign,
      bottomAlign,
      topColor,
      bottomColor,
    } = event.payload;
    currentInstanceId = id;
    if (headerEl) headerEl.textContent = `Instance settings — ${id}`;
    if (top !== undefined && top !== null) topEl.value = top;
    if (bottom !== undefined && bottom !== null) bottomEl.value = bottom;
    if (topBold !== undefined && topBold !== null) topBoldEl.checked = !!topBold;
    if (bottomBold !== undefined && bottomBold !== null) bottomBoldEl.checked = !!bottomBold;
    topFamilyEl.value =
      topFontFamily !== undefined && topFontFamily !== null ? topFontFamily : "";
    bottomFamilyEl.value =
      bottomFontFamily !== undefined && bottomFontFamily !== null ? bottomFontFamily : "";
    if (topMonospaced !== undefined && topMonospaced !== null)
      topMonoEl.checked = !!topMonospaced;
    if (bottomMonospaced !== undefined && bottomMonospaced !== null)
      bottomMonoEl.checked = !!bottomMonospaced;
    if (topAlign !== undefined && topAlign !== null)
      topAlignEl.value = String(topAlign);
    // Show the instance's current color mode: a `solid` ColorStyle selects
    // "Custom" and pre-fills the swatch/hex; `default` (or absence) selects
    // "System" and hides the custom row.
    if (topColor !== undefined && topColor !== null) fillColorStyle("top", topColor);
    if (bottomAlign !== undefined && bottomAlign !== null)
      bottomAlignEl.value = String(bottomAlign);
    if (bottomColor !== undefined && bottomColor !== null) fillColorStyle("bottom", bottomColor);
    if (layout !== undefined && layout !== null) {
      const l = layout;
      layoutBottomEl.checked = l === 0;
      layoutTopEl.checked = l === 1;
      layoutEqualEl.checked = l === 2;
      sizeAsymEl.style.display = l === 2 ? "none" : "";
      sizeEqualEl.style.display = l === 2 ? "" : "none";

      // The emitted top/bottom sizes are *displayed* values, so reverse-map
      // them onto the role-based state using the layout that produced them.
      const ts = topSize !== undefined && topSize !== null ? Number(topSize) : 7;
      const bs =
        bottomSize !== undefined && bottomSize !== null ? Number(bottomSize) : 12;
      if (l === 0) {
        curSmall = ts;
        curLarge = bs;
      } else if (l === 1) {
        curLarge = ts;
        curSmall = bs;
      } else {
        curEqual = ts;
      }
      seedSizeSliders(l);
    }
  }).catch((err) => console.error("Failed to listen for popup open:", err));

  document.querySelector("#popup-text-update").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup update ignored: no instance is targeted.");
      return;
    }
    invoke("plugin:multiline-menubar|set_text", {
      payload: {
        id: currentInstanceId,
        top: topEl.value,
        bottom: bottomEl.value,
      },
    }).catch((err) => console.error("Failed to update menubar:", err));
  });

  // Reset the text to the instance's own id (mb-N on both lines) — the demo
  // default that makes instances easy to identify.
  document.querySelector("#popup-text-reset").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup text reset ignored: no instance is targeted.");
      return;
    }
    topEl.value = currentInstanceId;
    bottomEl.value = currentInstanceId;
    invoke("plugin:multiline-menubar|set_text", {
      payload: {
        id: currentInstanceId,
        top: currentInstanceId,
        bottom: currentInstanceId,
      },
    }).catch((err) => console.error("Failed to reset text:", err));
  });

  // Resolve the effective color for a line: prefer the hex text field (works
  // even when the native <input type=color> picker can't open in this window),
  // fall back to the color input value.
  const resolveColor = (line) => {
    const hex = document.querySelector(`#popup-${line}-hex`).value.trim();
    if (hex) return hex;
    return document.querySelector(`#popup-${line}-color`).value;
  };

  // Build a ColorStyle for one line: "Custom" pins a hex value, "System"
  // falls back to the theme-following default.
  const buildStyle = (line) => {
    return colorMode[line] === "solid"
      ? { type: "solid", value: resolveColor(line) }
      : { type: "default" };
  };

  // System/Custom segmented buttons: switching only updates the UI — the
  // chosen style is sent when "Apply colors" is pressed.
  for (const line of ["top", "bottom"]) {
    const groupEl = line === "top" ? topCmodeEl : bottomCmodeEl;
    const rowEl = line === "top" ? topSolidRowEl : bottomSolidRowEl;
    groupEl.addEventListener("click", (e) => {
      const btn = e.target.closest("button[data-cmode]");
      if (!btn || btn.dataset.cmode === colorMode[line]) return;
      colorMode[line] = btn.dataset.cmode;
      setColorModePressed(groupEl, colorMode[line]);
      rowEl.hidden = colorMode[line] !== "solid";
    });
  }

  // Apply the chosen colors to whichever instance opened this popup.
  document.querySelector("#popup-colors").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup colors ignored: no instance is targeted.");
      return;
    }
    invoke("plugin:multiline-menubar|set_colors", {
      payload: {
        id: currentInstanceId,
        top: buildStyle("top"),
        bottom: buildStyle("bottom"),
      },
    }).catch((err) => console.error("Failed to set colors:", err));
  });

  // Revert to the system default text color for whichever instance opened
  // this popup, and reset the System/Custom UI back to System.
  document.querySelector("#popup-reset").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup reset ignored: no instance is targeted.");
      return;
    }
    colorMode.top = "default";
    colorMode.bottom = "default";
    setColorModePressed(topCmodeEl, "default");
    setColorModePressed(bottomCmodeEl, "default");
    topSolidRowEl.hidden = true;
    bottomSolidRowEl.hidden = true;
    invoke("plugin:multiline-menubar|set_colors", {
      payload: {
        id: currentInstanceId,
        top: { type: "default" },
        bottom: { type: "default" },
      },
    }).catch((err) => console.error("Failed to reset colors:", err));
  });

  // Apply the per-line bold toggle to whichever instance opened this popup.
  // top/bottom are independent; false leaves that line's weight to the layout.
  document.querySelector("#popup-bold").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup bold ignored: no instance is targeted.");
      return;
    }
    invoke("plugin:multiline-menubar|set_bold", {
      payload: {
        id: currentInstanceId,
        top: topBoldEl.checked,
        bottom: bottomBoldEl.checked,
      },
    }).catch((err) => console.error("Failed to set bold:", err));
  });

  // Clear both bold toggles and revert the lines to their layout weights.
  document.querySelector("#popup-bold-reset").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup bold reset ignored: no instance is targeted.");
      return;
    }
    topBoldEl.checked = false;
    bottomBoldEl.checked = false;
    invoke("plugin:multiline-menubar|set_bold", {
      payload: { id: currentInstanceId, top: false, bottom: false },
    }).catch((err) => console.error("Failed to reset bold:", err));
  });

  // Apply the per-line font family to whichever instance opened this popup.
  // Empty fields mean "system font".
  const applyPopupFontFamily = () => {
    if (!currentInstanceId) {
      console.warn("Popup font family ignored: no instance is targeted.");
      return;
    }
    invoke("plugin:multiline-menubar|set_font_family", {
      payload: {
        id: currentInstanceId,
        top: topFamilyEl.value.trim() || null,
        bottom: bottomFamilyEl.value.trim() || null,
      },
    }).catch((err) => console.error("Failed to set font family:", err));
  };

  document
    .querySelector("#popup-font-family")
    .addEventListener("click", applyPopupFontFamily);

  document
    .querySelector("#popup-font-family-reset")
    .addEventListener("click", () => {
      topFamilyEl.value = "";
      bottomFamilyEl.value = "";
      applyPopupFontFamily();
    });

  // Apply the per-line monospaced-digit toggle to whichever instance opened
  // this popup. An explicit font family takes precedence over this toggle.
  const applyPopupMonospaced = () => {
    if (!currentInstanceId) {
      console.warn("Popup monospaced ignored: no instance is targeted.");
      return;
    }
    invoke("plugin:multiline-menubar|set_monospaced", {
      payload: {
        id: currentInstanceId,
        top: topMonoEl.checked,
        bottom: bottomMonoEl.checked,
      },
    }).catch((err) => console.error("Failed to set monospaced:", err));
  };

  document
    .querySelector("#popup-monospaced")
    .addEventListener("click", applyPopupMonospaced);

  document
    .querySelector("#popup-monospaced-reset")
    .addEventListener("click", () => {
      topMonoEl.checked = false;
      bottomMonoEl.checked = false;
      applyPopupMonospaced();
    });

  // Apply the per-line horizontal alignment to whichever instance opened
  // this popup. 0 = left, 1 = center, 2 = right. Alignment does not change
  // the item width, so a plain repaint is enough.
  const applyPopupAlignment = () => {
    if (!currentInstanceId) {
      console.warn("Popup alignment ignored: no instance is targeted.");
      return;
    }
    invoke("plugin:multiline-menubar|set_alignment", {
      payload: {
        id: currentInstanceId,
        top: parseInt(topAlignEl.value, 10) || 0,
        bottom: parseInt(bottomAlignEl.value, 10) || 0,
      },
    }).catch((err) => console.error("Failed to set alignment:", err));
  };

  document
    .querySelector("#popup-alignment")
    .addEventListener("click", applyPopupAlignment);

  document
    .querySelector("#popup-alignment-reset")
    .addEventListener("click", () => {
      topAlignEl.value = "0";
      bottomAlignEl.value = "0";
      applyPopupAlignment();
    });

  // Live-update the font size readouts as the sliders move.
  largeSizeEl.addEventListener("input", () => {
    largeSizeValueEl.textContent = largeSizeEl.value;
  });
  smallSizeEl.addEventListener("input", () => {
    smallSizeValueEl.textContent = smallSizeEl.value;
  });
  sizeEl.addEventListener("input", () => {
    sizeValueEl.textContent = sizeEl.value;
  });

  // Switching the layout radio swaps the visible slider group and persists the
  // layout for the instance that opened this popup. We also re-push the
  // role-based sizes: the native side stores sizes per role and a layout
  // switch re-routes which line is large, so re-sending keeps the JS state and
  // the native state in lock-step (and forces a repaint even when triggered
  // from this popup window).
  const applyLayout = () => {
    if (!currentInstanceId) return;
    const l = currentLayoutValue();
    sizeAsymEl.style.display = l === 2 ? "none" : "";
    sizeEqualEl.style.display = l === 2 ? "" : "none";
    seedSizeSliders(l);

    // Switch the layout FIRST: the native side clamps font sizes with a
    // different range per layout, so pushing sizes while still in the old
    // layout would briefly render them clamped to the old ranges.
    invoke("plugin:multiline-menubar|set_layout", {
      payload: { id: currentInstanceId, layout: l },
    })
      .then(() => {
        const [top, bottom] = displayedForLayout(l);
        return invoke("plugin:multiline-menubar|set_font_sizes", {
          payload: { id: currentInstanceId, top, bottom },
        });
      })
      .catch((err) => console.error("Failed to switch layout:", err));
  };
  layoutBottomEl.addEventListener("change", applyLayout);
  layoutTopEl.addEventListener("change", applyLayout);
  layoutEqualEl.addEventListener("change", applyLayout);

  // Apply the chosen font sizes to whichever instance opened this popup.
  // Because the popup always targets `currentInstanceId`, this works for both
  // the main instance and any secondary instance (e.g. one opened by
  // left-clicking its own menu-bar item). The displayed top/bottom are derived
  // from the role sliders for the current layout (see `displayedForLayout`).
  document.querySelector("#popup-sizes").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup sizes ignored: no instance is targeted.");
      return;
    }
    const l = currentLayoutValue();
    let top;
    let bottom;
    if (l === 2) {
      curEqual = Number(sizeEl.value);
      sizeValueEl.textContent = curEqual;
      top = curEqual;
      bottom = curEqual;
    } else {
      curLarge = Number(largeSizeEl.value);
      curSmall = Number(smallSizeEl.value);
      largeSizeValueEl.textContent = curLarge;
      smallSizeValueEl.textContent = curSmall;
      [top, bottom] = displayedForLayout(l);
    }
    invoke("plugin:multiline-menubar|set_font_sizes", {
      payload: {
        id: currentInstanceId,
        top,
        bottom,
      },
    }).catch((err) => console.error("Failed to set font sizes:", err));
  });

  // Restore the default role sizes (small 7 / large 12 / equal 9), sync the
  // sliders and re-apply.
  document.querySelector("#popup-sizes-reset").addEventListener("click", () => {
    if (!currentInstanceId) {
      console.warn("Popup sizes reset ignored: no instance is targeted.");
      return;
    }
    curSmall = 7;
    curLarge = 12;
    curEqual = 9;
    const l = currentLayoutValue();
    seedSizeSliders(l);
    const [top, bottom] = displayedForLayout(l);
    invoke("plugin:multiline-menubar|set_font_sizes", {
      payload: { id: currentInstanceId, top, bottom },
    }).catch((err) => console.error("Failed to reset font sizes:", err));
  });

  document.querySelector("#popup-close").addEventListener("click", () => {
    const payload = currentInstanceId
      ? { id: currentInstanceId }
      : {};
    invoke("plugin:multiline-menubar|close_popup", { payload }).catch((err) =>
      console.error("Failed to close popup:", err)
    );
  });
});

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const ID = "main";
const SECOND_ID = "second";
let secondVisible = false;
let secondMenuUnlisten = null;

let greetInputEl;
let greetMsgEl;

async function greet() {
  greetMsgEl.textContent = await invoke("greet", { name: greetInputEl.value });
}

async function updateMenubar(top, bottom) {
  await invoke("plugin:multiline-menubar|set_text", {
    payload: { id: ID, top, bottom },
  });
  await refreshMenubarStatus();
}

async function updateFontSizes(top, bottom) {
  await invoke("plugin:multiline-menubar|set_font_sizes", {
    payload: { id: ID, top, bottom },
  });
}

async function updateBold(topBold, bottomBold) {
  await invoke("plugin:multiline-menubar|set_bold", {
    payload: { id: ID, top: topBold, bottom: bottomBold },
  });
}

async function showMenubar() {
  await invoke("plugin:multiline-menubar|set_visible", {
    payload: { id: ID, visible: true },
  });
  await refreshMenubarStatus();
}

async function hideMenubar() {
  await invoke("plugin:multiline-menubar|set_visible", {
    payload: { id: ID, visible: false },
  });
  await refreshMenubarStatus();
}

async function refreshMenubarStatus() {
  const result = await invoke("plugin:multiline-menubar|is_visible", {
    payload: { id: ID },
  });
  const statusEl = document.querySelector("#menubar-status");
  statusEl.textContent = result.visible
    ? "Menu bar item is visible"
    : "Menu bar item is hidden";
}

async function togglePopup() {
  await invoke("plugin:multiline-menubar|toggle_popup", {
    payload: { id: ID },
  });
}

async function toggleSecondInstance() {
  const btn = document.querySelector("#toggle-second-btn");
  if (!secondVisible) {
    await invoke("plugin:multiline-menubar|create", {
      payload: { id: SECOND_ID, top: "Net", bottom: "5G" },
    });
    await invoke("plugin:multiline-menubar|set_font_sizes", {
      payload: { id: SECOND_ID, top: 8, bottom: 14 },
    });
    await invoke("plugin:multiline-menubar|set_menu", {
      payload: {
        id: SECOND_ID,
        items: [
          { type: "item", id: "ping", text: "Ping" },
          {
            type: "submenu",
            text: "Speed",
            items: [
              { type: "item", id: "speed-fast", text: "Fast" },
              { type: "item", id: "speed-slow", text: "Slow" },
            ],
          },
          { type: "separator" },
          { type: "item", id: "quit2", text: "Quit" },
        ],
      },
    });
    // Showcase: the second instance uses solid colors on both lines.
    await invoke("plugin:multiline-menubar|set_colors", {
      payload: {
        id: SECOND_ID,
        top: { type: "solid", value: "#34d399" },
        bottom: { type: "solid", value: "#60a5fa" },
      },
    });
    if (!secondMenuUnlisten) {
      secondMenuUnlisten = await listenMenu(SECOND_ID);
    }
    secondVisible = true;
    btn.textContent = "Remove 2nd instance";
  } else {
    await invoke("plugin:multiline-menubar|remove", {
      payload: { id: SECOND_ID },
    });
    if (secondMenuUnlisten) {
      secondMenuUnlisten();
      secondMenuUnlisten = null;
    }
    secondVisible = false;
    btn.textContent = "Add 2nd instance";
  }
}

// Handle a context-menu selection from any instance.
function handleMenuSelection(instanceId, itemId, checked) {
  const menuLog = document.querySelector("#menu-log");
  menuLog.textContent =
    `Menu event: ${instanceId} -> ${itemId}` +
    (checked === undefined ? "" : ` (checked=${checked})`);

  if (itemId === "quit" || itemId === "quit2") {
    // Quit is handled in Rust (the plugin's `on_menu_event` calls
    // `AppHandle::exit`). `window.__TAURI__.app.exit` does not exist in the
    // base `app` module, so a JS-side quit would silently fail.
    return;
  } else if (itemId === "auto-popup") {
    // `checked` reflects the state after the toggle.
    invoke("plugin:multiline-menubar|set_auto_popup", {
      payload: { enabled: Boolean(checked) },
    }).catch((err) => console.error("Failed to set auto popup:", err));
  }
}

// Subscribe to the per-instance menu channel. The plugin re-emits muda menu
// selections here; `@tauri-apps/api/menu`'s onMenuEvent does NOT cover them,
// since that channel only carries menus built by Tauri's own menu plugin.
function listenMenu(instanceId) {
  return listen(`multiline-menubar://${instanceId}//menu`, (event) => {
    const { id, itemId, checked } = event.payload;
    handleMenuSelection(id, itemId, checked);
  }).catch((err) =>
    console.error(`Failed to listen for ${instanceId} menu events:`, err)
  );
}

// Build the right-click menu for the main instance as a real Tauri/muda menu.
async function setupMainMenu() {
  let version = "0.0.0";
  try {
    version = await window.__TAURI__.app.getVersion();
  } catch (_) {}

  await invoke("plugin:multiline-menubar|set_menu", {
    payload: {
      id: ID,
      items: [
        { type: "item", id: "version", text: `Version ${version}`, disabled: true },
        { type: "separator" },
        { type: "check", id: "auto-popup", text: "Popup on left click", checked: true },
        { type: "separator" },
        { type: "item", id: "quit", text: "Quit", accelerator: "CmdOrCtrl+Q" },
      ],
    },
  });

  await listenMenu(ID);
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  document.querySelector("#greet-form").addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  const topInput = document.querySelector("#top-input");
  const bottomInput = document.querySelector("#bottom-input");
  const largeSizeInput = document.querySelector("#large-size-input");
  const smallSizeInput = document.querySelector("#small-size-input");
  const largeSizeValue = document.querySelector("#large-size-value");
  const smallSizeValue = document.querySelector("#small-size-value");
  const layoutBottomEl = document.querySelector("#layout-bottom");
  const layoutTopEl = document.querySelector("#layout-top");
  const layoutEqualEl = document.querySelector("#layout-equal");
  const sizeAsymEl = document.querySelector("#size-asym");
  const sizeEqualEl = document.querySelector("#size-equal");
  const sizeInput = document.querySelector("#size-input");
  const sizeValue = document.querySelector("#size-value");

  // Role-based font sizes, mirrored from the native side. The two asymmetric
  // layouts are vertical mirrors, so we track the emphasized (large) and
  // de-emphasized (small) sizes separately; switching layouts just moves which
  // line is large without losing a value.
  let curSmall = 7;
  let curLarge = 12;
  let curEqual = 9;

  // Which layout radio is selected: 0 = emphasis-bottom (default),
  // 1 = emphasis-top, 2 = equal.
  const mainLayoutValue = () => {
    if (layoutTopEl.checked) return 1;
    if (layoutEqualEl.checked) return 2;
    return 0;
  };

  // Compute the *displayed* top/bottom sizes from the role-based values. The
  // native `setFontSizes` API always takes displayed positions.
  const displayedForLayout = (l) => {
    if (l === 2) return [curEqual, curEqual];
    if (l === 1) return [curLarge, curSmall]; // top large, bottom small
    return [curSmall, curLarge]; // top small, bottom large (default)
  };

  // Sync the visible slider group + values to the role-based state.
  const seedSizeSliders = (l) => {
    if (l === 2) {
      sizeInput.value = curEqual;
      sizeValue.textContent = curEqual;
    } else {
      largeSizeInput.value = curLarge;
      largeSizeValue.textContent = curLarge;
      smallSizeInput.value = curSmall;
      smallSizeValue.textContent = curSmall;
    }
  };

  document.querySelector("#menubar-form").addEventListener("submit", (e) => {
    e.preventDefault();
    updateMenubar(topInput.value, bottomInput.value);
  });

  // Show the slider group that matches the current layout and seed the
  // role-based sliders so they reflect the current sizes.
  const applyLayoutVisibility = () => {
    const l = mainLayoutValue();
    sizeAsymEl.style.display = l === 2 ? "none" : "";
    sizeEqualEl.style.display = l === 2 ? "" : "none";
    seedSizeSliders(l);
  };

  const syncFontSizes = () => {
    const l = mainLayoutValue();
    if (l === 2) {
      curEqual = Number(sizeInput.value);
      sizeValue.textContent = curEqual;
      updateFontSizes(curEqual, curEqual);
    } else {
      curLarge = Number(largeSizeInput.value);
      curSmall = Number(smallSizeInput.value);
      largeSizeValue.textContent = curLarge;
      smallSizeValue.textContent = curSmall;
      const [top, bottom] = displayedForLayout(l);
      updateFontSizes(top, bottom);
    }
  };

  largeSizeInput.addEventListener("input", syncFontSizes);
  smallSizeInput.addEventListener("input", syncFontSizes);
  sizeInput.addEventListener("input", syncFontSizes);

  // Layout toggle: persist the layout and re-push the role-based sizes so the
  // native side stays in sync (it stores sizes per role, not per position).
  const onLayoutChange = () => {
    applyLayoutVisibility();
    const l = mainLayoutValue();
    // Switch the layout FIRST: the native side clamps font sizes with a
    // different range per layout, so pushing sizes while still in the old
    // layout would briefly render them clamped to the old ranges.
    invoke("plugin:multiline-menubar|set_layout", {
      payload: { id: ID, layout: l },
    })
      .then(() => {
        const [top, bottom] = displayedForLayout(l);
        return updateFontSizes(top, bottom);
      })
      .catch((err) => console.error("Failed to switch layout:", err));
  };
  layoutBottomEl.addEventListener("change", onLayoutChange);
  layoutTopEl.addEventListener("change", onLayoutChange);
  layoutEqualEl.addEventListener("change", onLayoutChange);

  document.querySelector("#show-btn").addEventListener("click", showMenubar);
  document.querySelector("#hide-btn").addEventListener("click", hideMenubar);
  document
    .querySelector("#toggle-popup-btn")
    .addEventListener("click", togglePopup);
  document
    .querySelector("#toggle-second-btn")
    .addEventListener("click", toggleSecondInstance);

  // Color controls: one solid picker per line. There is no mode selector —
  // picking a color and applying sends a solid `ColorStyle`. The "Reset"
  // button sends `default`, which lets the OS draw the standard menu-bar
  // text color (adapts to light/dark mode).
  const buildStyle = (line) => {
    const hex = document.querySelector(`#${line}-color`).value;
    return { type: "solid", value: hex };
  };

  document
    .querySelector("#colors-form")
    .addEventListener("submit", (e) => {
      e.preventDefault();
      invoke("plugin:multiline-menubar|set_colors", {
        payload: {
          id: ID,
          top: buildStyle("top"),
          bottom: buildStyle("bottom"),
        },
      })
        .then(() => {
          document.querySelector("#color-log").textContent = "Colors applied";
        })
        .catch((err) => {
          document.querySelector("#color-log").textContent = `Error: ${err}`;
        });
    });

  document.querySelector("#reset-colors").addEventListener("click", () => {
    invoke("plugin:multiline-menubar|set_colors", {
      payload: {
        id: ID,
        top: { type: "default" },
        bottom: { type: "default" },
      },
    })
      .then(() => {
        document.querySelector("#color-log").textContent =
          "Reverted to system color";
      })
      .catch((err) => {
        document.querySelector("#color-log").textContent = `Error: ${err}`;
      });
  });

  // Bold controls: one checkbox per line, independent of layout.
  const topBoldEl = document.querySelector("#top-bold");
  const bottomBoldEl = document.querySelector("#bottom-bold");

  const applyBold = () => {
    const top = topBoldEl.checked;
    const bottom = bottomBoldEl.checked;
    updateBold(top, bottom)
      .then(() => {
        const parts = [];
        if (top) parts.push("top");
        if (bottom) parts.push("bottom");
        document.querySelector("#bold-log").textContent = parts.length
          ? `Bold: ${parts.join(" + ")}`
          : "Bold cleared (layout weights)";
      })
      .catch((err) => {
        document.querySelector("#bold-log").textContent = `Error: ${err}`;
      });
  };

  document
    .querySelector("#bold-form")
    .addEventListener("submit", (e) => {
      e.preventDefault();
      applyBold();
    });

  document.querySelector("#reset-bold").addEventListener("click", () => {
    topBoldEl.checked = false;
    bottomBoldEl.checked = false;
    applyBold();
  });

  // Font-family controls: one text field per line; empty = system font.
  const topFamilyEl = document.querySelector("#top-font-family");
  const bottomFamilyEl = document.querySelector("#bottom-font-family");

  const applyFontFamily = () => {
    const top = topFamilyEl.value.trim() || null;
    const bottom = bottomFamilyEl.value.trim() || null;
    invoke("plugin:multiline-menubar|set_font_family", {
      payload: { id: ID, top, bottom },
    })
      .then(() => {
        const parts = [];
        if (top) parts.push(`top: ${top}`);
        if (bottom) parts.push(`bottom: ${bottom}`);
        document.querySelector("#font-family-log").textContent = parts.length
          ? `Families: ${parts.join(", ")}`
          : "System font restored";
      })
      .catch((err) => {
        document.querySelector("#font-family-log").textContent = `Error: ${err}`;
      });
  };

  document
    .querySelector("#font-family-form")
    .addEventListener("submit", (e) => {
      e.preventDefault();
      applyFontFamily();
    });

  document.querySelector("#reset-font-family").addEventListener("click", () => {
    topFamilyEl.value = "";
    bottomFamilyEl.value = "";
    applyFontFamily();
  });

  // Create the "main" instance and wire events.
  invoke("plugin:multiline-menubar|create", {
    payload: { id: ID, top: topInput.value, bottom: bottomInput.value },
  })
    .then(() =>
      // Default layout is emphasis-bottom: displayed top = small line,
      // displayed bottom = large line. Seed the native side accordingly.
      updateFontSizes(
        Number(smallSizeInput.value),
        Number(largeSizeInput.value)
      )
    )
    .then(setupMainMenu)
    .then(refreshMenubarStatus)
    .catch((err) => console.error("Failed to initialize menubar:", err));

  // Click events (per-instance, aligned with Tauri's TrayIconEvent::Click).
  listen(`multiline-menubar://${ID}//click`, (event) => {
    const { button, position, rect } = event.payload;
    const logEl = document.querySelector("#click-log");
    logEl.textContent =
      `Last click: ${button} @ cursor (${Math.round(position.x)}, ${Math.round(
        position.y
      )}) rect ${Math.round(rect.width)}x${Math.round(rect.height)}`;
  }).catch((err) => console.error("Failed to listen for clicks:", err));

  // Hover events (per-instance, aligned with Tauri's Enter/Leave).
  listen(`multiline-menubar://${ID}//enter`, (event) => {
    document.querySelector("#hover-log").textContent =
      `Entered ${event.payload.id} (rect ${Math.round(
        event.payload.rect.width
      )}x${Math.round(event.payload.rect.height)})`;
  }).catch((err) => console.error("Failed to listen for enter:", err));

  listen(`multiline-menubar://${ID}//leave`, () => {
    document.querySelector("#hover-log").textContent = "Left the menu bar item";
  }).catch((err) => console.error("Failed to listen for leave:", err));
});

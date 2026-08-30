const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ---------------------------------------------------------------------------
// 5 fixed menubar instances (mb-1 … mb-5). Each item just shows its own id on
// both lines, so we can talk about "mb-3" without ambiguity. Click an item in
// the menu bar to open its settings popup (text, fonts, sizes, bold, colors,
// alignment, layout — all editable per instance there).
//
// v1.6.0 semantics:
//   - hide  = set_visible(false) → visible=NO (programmaticHide guard, no
//     remove event fires)
//   - show  = set_visible(true)  → visible=YES (resets removedByUser, so a
//     previously ⌘-dragged-out item comes back)
//   - ⌘-drag-out = the SYSTEM removes that single item → KVO detects it →
//     `{id}//remove` event → we only flip that instance's toggle off; the
//     other 4 stay untouched (no app-wide banner, no Rebuild shown).
//
// The shown/hidden state of each instance is persisted to localStorage, so a
// restart keeps the same arrangement.
// ---------------------------------------------------------------------------
const INSTANCES = ["mb-1", "mb-2", "mb-3", "mb-4", "mb-5"];
const STORAGE_KEY = "multiline-menubar-demo-new:shown-v1";

const created = new Set();
let shown = loadShown(); // { id: boolean }

function loadShown() {
  const map = {};
  for (const id of INSTANCES) map[id] = true;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const obj = JSON.parse(raw);
      for (const id of INSTANCES) {
        if (typeof obj[id] === "boolean") map[id] = obj[id];
      }
    }
  } catch (_) {}
  return map;
}

function saveShown() {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(shown));
  } catch (_) {}
}

// ---------------------------------------------------------------------------
// Instance lifecycle
// ---------------------------------------------------------------------------

async function createInstance(id) {
  await invoke("plugin:multiline-menubar|create", {
    payload: { id, top: id, bottom: id },
  });
  created.add(id);
  await setupInstanceMenu(id);
  // Honor the persisted state: a previously hidden instance is created
  // invisible right away (set_visible(false) → visible=NO, no remove event).
  if (shown[id] === false) {
    await invoke("plugin:multiline-menubar|set_visible", {
      payload: { id, visible: false },
    }).catch((err) => console.error(`set_visible failed for ${id}:`, err));
  }
}

// Right-click context menu for every instance: version line (disabled) + Quit.
// "quit" is a built-in id — the plugin's Rust side exits the whole app, so no
// JS handling is needed here.
async function setupInstanceMenu(id) {
  let version = "0.0.0";
  try {
    version = await window.__TAURI__.app.getVersion();
  } catch (_) {}
  await invoke("plugin:multiline-menubar|set_menu", {
    payload: {
      id,
      items: [
        { type: "item", id: "version", text: `multiline-menubar-demo-new v${version}`, enabled: false },
        { type: "separator" },
        { type: "item", id: "quit", text: "Quit", accelerator: "CmdOrCtrl+Q" },
      ],
    },
  }).catch((err) => console.error(`Failed to set menu for ${id}:`, err));
}

async function setInstanceVisible(id, visible) {
  shown[id] = visible;
  saveShown();
  if (created.has(id)) {
    await invoke("plugin:multiline-menubar|set_visible", {
      payload: { id, visible },
    }).catch((err) => console.error(`set_visible failed for ${id}:`, err));
  }
  renderList();
  updateStatus();
}

async function setAllVisible(visible) {
  for (const id of INSTANCES) {
    await setInstanceVisible(id, visible);
  }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

function renderList() {
  const ul = document.querySelector("#instance-list");
  if (!ul) return;
  ul.innerHTML = "";
  for (const id of INSTANCES) {
    if (!created.has(id)) continue;
    const li = document.createElement("li");
    li.className = "instance-row";

    const name = document.createElement("span");
    name.className = "instance-name";
    name.textContent = id;

    const text = document.createElement("span");
    text.className = "instance-text muted";
    text.textContent = `"${id}" / "${id}"`;

    const switchLabel = document.createElement("label");
    switchLabel.className = "switch";
    switchLabel.title = "Show / hide this menu bar item";
    const toggle = document.createElement("input");
    toggle.type = "checkbox";
    toggle.checked = shown[id] !== false;
    toggle.addEventListener("change", () =>
      setInstanceVisible(id, toggle.checked)
    );
    const slider = document.createElement("span");
    slider.className = "slider";
    switchLabel.appendChild(toggle);
    switchLabel.appendChild(slider);

    li.appendChild(name);
    li.appendChild(text);
    li.appendChild(switchLabel);
    ul.appendChild(li);
  }
}

function updateStatus() {
  const el = document.querySelector("#instance-status");
  if (!el) return;
  const shownCount = INSTANCES.filter((id) => shown[id] !== false).length;
  el.textContent = `${created.size} instances · showing ${shownCount} / hidden ${INSTANCES.length - shownCount}`;
}

// ---------------------------------------------------------------------------
// Events (light logging)
// ---------------------------------------------------------------------------

function listenInstanceEvents(id) {
  listen(`multiline-menubar://${id}//click`, () => {
    const el = document.querySelector("#click-log");
    if (el) el.textContent = `Click: ${id} — settings popup opened`;
  }).catch(() => {});

  // v1.6.0: a ⌘-drag-out removes ONLY this single item (KVO-detected). We
  // flip just this instance's toggle off and persist it; the other 4 stay
  // untouched. Re-opening the toggle (set_visible(true)) brings it back.
  listen(`multiline-menubar://${id}//remove`, () => {
    shown[id] = false;
    saveShown();
    renderList();
    updateStatus();
    const el = document.querySelector("#click-log");
    if (el) {
      el.textContent = `${id} 被拖出菜单栏 — toggle 已关闭，重新打开可恢复显示`;
    }
  }).catch(() => {});
}

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

window.addEventListener("DOMContentLoaded", async () => {
  document.querySelector("#show-all-btn").addEventListener("click", () => setAllVisible(true));
  document.querySelector("#hide-all-btn").addEventListener("click", () => setAllVisible(false));

  // Create all 5 instances up front (macOS 26: create early at startup;
  // runtime-created items may not show).
  for (const id of INSTANCES) {
    await createInstance(id).catch((err) =>
      console.error(`Failed to create ${id}:`, err)
    );
    listenInstanceEvents(id);
  }
  renderList();
  updateStatus();
});

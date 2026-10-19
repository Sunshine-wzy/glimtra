import "./style.css";
import * as bridge from "./bridge";
import { defaultSettings, languages } from "./types";
import type { Settings, SettingsSnapshot } from "./types";

const icons = {
  translate: '<path d="m4 5 8 0m-4-2v2m-3 0c.5 4 3 7 7 9M11 5c-.5 4-3 7-7 9m9 5 4-11 4 11m-6.6-4h5.2"/>',
  settings: '<path d="M4 7h16M4 17h16"/><circle cx="8" cy="7" r="3"/><circle cx="16" cy="17" r="3"/>',
  copy: '<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V4H4v12h4"/>',
  paste: '<path d="M9 5H5v16h14V5h-4"/><rect x="9" y="3" width="6" height="4" rx="1"/>',
  arrow: '<path d="M5 12h14m-6-6 6 6-6 6"/>',
  swap: '<path d="M4 8h15m-4-4 4 4-4 4M20 16H5m4-4-4 4 4 4"/>',
  hide: '<path d="M5 12h14"/>',
  clear: '<path d="m6 6 12 12M6 18 18 6"/>',
  leaf: '<path d="M19 4C8 3 3 8 6 15c4 7 13 2 13-11ZM5 20l9-10"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
};
const icon = (name: keyof typeof icons) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name]}</svg>`;
const languageOptions = languages.map(([value, label]) => `<option value="${value}">${label}</option>`).join("");

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <div class="shell">
    <header class="app-header">
      <a class="brand" href="#" aria-label="Glimtra 翻译首页"><span class="brand-mark">g<span>·</span></span><span>glimtra<small>轻一点，译近一点</small></span></a>
      <nav class="tabs" aria-label="主导航">
        <button id="translation-tab" class="tab active" aria-current="page">${icon("translate")}翻译</button>
        <button id="settings-tab" class="tab">${icon("settings")}设置</button>
      </nav>
      <button id="hide" class="icon-button" title="隐藏到托盘（Esc）" aria-label="隐藏到托盘">${icon("hide")}</button>
    </header>

    <div id="preview-notice" class="notice" hidden>浏览器界面预览 · 翻译、快捷键与自启动请在桌面应用中使用。</div>
    <main>
      <section id="translation-view" aria-label="翻译">
        <div class="intro"><div><p class="eyebrow">LESS FRICTION. MORE UNDERSTANDING.</p><h1>让语言，不再成为距离。</h1></div><span class="quiet-badge">${icon("leaf")}轻盈，随时在</span></div>
        <div id="setup-notice" class="setup-notice" hidden><span>接入你的模型，开始第一段翻译。</span><button id="setup-link" class="text-button">前往设置 ${icon("arrow")}</button></div>
        <div class="translation-card">
          <div class="language-bar">
            <label><span class="sr-only">原文语言</span><select id="source-language"><option value="auto">自动检测</option>${languageOptions}</select></label>
            <button id="swap" class="icon-button" title="交换语言和文本" aria-label="交换语言和文本">${icon("swap")}</button>
            <label><span class="sr-only">目标语言</span><select id="target-language">${languageOptions}</select></label>
          </div>
          <div class="translation-panes">
            <div class="pane source-pane">
              <div class="pane-heading"><span>原文</span><div class="pane-actions"><button id="paste" class="subtle-button">${icon("paste")}粘贴</button><button id="clear" class="icon-button small" title="清空" aria-label="清空原文和译文">${icon("clear")}</button></div></div>
              <textarea id="source-text" maxlength="20000" spellcheck="false" aria-label="待翻译文本" placeholder="在这里写下，或粘贴一段文字…"></textarea>
              <div class="pane-footer"><span id="character-count">0 / 20,000</span><span>保留段落与格式</span></div>
            </div>
            <div class="pane result-pane">
              <div class="pane-heading"><span>译文</span><button id="copy" class="subtle-button" disabled>${icon("copy")}复制</button></div>
              <div id="result-empty" class="result-empty"><span class="empty-mark">Aa<span>あ</span></span><p>另一种语言，同样的意思。</p><small>译文将在这里逐步呈现</small></div>
              <div id="result-text" class="result-text" role="region" aria-label="翻译结果" hidden></div>
              <div class="pane-footer"><span id="result-state">等待翻译</span><span id="model-label"></span></div>
            </div>
          </div>
        </div>
        <div class="translation-actions"><p><kbd>Ctrl</kbd> + <kbd>Enter</kbd><span>开始翻译</span></p><button id="translate" class="primary-button" disabled>翻译 ${icon("arrow")}</button><button id="cancel" class="secondary-button" hidden>停止翻译</button></div>
      </section>

      <section id="settings-view" aria-label="设置" hidden>
        <div class="intro settings-intro"><div><p class="eyebrow">MAKE IT YOURS.</p><h1>刚刚好的，个人设置。</h1></div></div>
        <form id="settings-form">
          <fieldset class="settings-card"><legend>翻译服务</legend><p class="field-description">连接支持 Chat Completions 接口的服务，使用你自己的模型。</p>
            <label class="field">API 请求地址<input id="endpoint" type="url" placeholder="https://your-provider.com/v1/chat/completions" autocomplete="off" maxlength="2048" /><small>填写完整请求地址；本地服务支持 http://localhost。</small></label>
            <div class="field-grid"><label class="field">模型名称<input id="model" placeholder="填写服务商提供的模型 ID" autocomplete="off" maxlength="200" /></label><label class="field">API Key<input id="api-key" type="password" placeholder="输入密钥（本地服务可留空）" autocomplete="new-password" maxlength="4096" /><small id="key-hint">密钥保存在系统凭据库中。</small></label></div>
            <label id="remove-key-row" class="checkbox-row" hidden><input id="remove-key" type="checkbox" />删除当前服务已保存的密钥</label>
          </fieldset>
          <fieldset class="settings-card"><legend>使用习惯</legend>
            <div class="field-grid"><label class="field">唤起 / 隐藏快捷键<input id="shortcut" value="Ctrl+Alt+T" autocomplete="off" required maxlength="80" /><small>例如 Ctrl+Alt+T，冲突时会提示。</small></label><label class="field">默认目标语言<select id="default-language">${languageOptions}</select></label></div>
            <div class="setting-row"><div><strong>开机自启动</strong><p>登录系统后安静地驻留托盘。</p></div><label class="switch"><input id="autostart" type="checkbox" aria-label="开机自启动" /><span></span></label></div>
          </fieldset>
          <div class="settings-actions"><span>原文仅在主动翻译时发送至所选服务。</span><button id="save" class="primary-button" type="submit">${icon("check")}保存设置</button></div>
        </form>
      </section>
      <div id="status" class="status-message" role="status" aria-live="polite" hidden></div>
    </main>
    <footer class="app-footer"><span><i class="status-dot"></i><span id="footer-state">准备就绪</span></span><span><kbd id="shortcut-label">Ctrl + Alt + T</kbd> 随时唤起<span class="footer-version">v0.1.0</span></span></footer>
  </div>`;

function el<T extends HTMLElement>(id: string): T { return document.getElementById(id) as T; }
const source = el<HTMLTextAreaElement>("source-text");
const result = el<HTMLDivElement>("result-text");
const sourceLanguage = el<HTMLSelectElement>("source-language");
const targetLanguage = el<HTMLSelectElement>("target-language");
const translateButton = el<HTMLButtonElement>("translate");
let snapshot: SettingsSnapshot = { settings: { ...defaultSettings }, hasApiKey: false, autostart: false, warning: null, openSettings: false };
let activeRequest: string | null = null;
let stopping = false;
let initialized = false;
let output = "";
let renderFrame = 0;

function showStatus(message: string, error = false): void {
  const status = el("status");
  status.textContent = message;
  status.classList.toggle("error", error);
  status.hidden = false;
}
function clearStatus(): void { el("status").hidden = true; }
function reportError(error: unknown): void { showStatus(error instanceof Error ? error.message : String(error), true); }
function openView(settings: boolean): void {
  el("translation-view").hidden = settings;
  el("settings-view").hidden = !settings;
  el("translation-tab").classList.toggle("active", !settings);
  el("settings-tab").classList.toggle("active", settings);
  el("translation-tab").setAttribute("aria-current", settings ? "false" : "page");
  el("settings-tab").setAttribute("aria-current", settings ? "page" : "false");
  clearStatus();
  if (!settings) source.focus();
}
function updateControls(): void {
  const busy = activeRequest !== null;
  translateButton.disabled = busy || !initialized || !source.value.trim();
  translateButton.hidden = busy;
  el("cancel").hidden = !busy;
  el<HTMLButtonElement>("cancel").disabled = stopping;
  el("cancel").textContent = stopping ? "正在停止…" : "停止翻译";
  el<HTMLButtonElement>("copy").disabled = !output;
  for (const id of ["clear", "swap", "paste", "save"]) el<HTMLButtonElement>(id).disabled = busy || !initialized;
  source.disabled = busy;
  sourceLanguage.disabled = busy;
  targetLanguage.disabled = busy;
  el("character-count").textContent = `${source.value.length.toLocaleString()} / 20,000`;
  el("footer-state").textContent = busy ? "翻译进行中" : bridge.desktop ? "准备就绪" : "界面预览";
}
function renderOutput(): void {
  renderFrame = 0;
  result.textContent = output;
  result.hidden = !output;
  el("result-empty").hidden = Boolean(output);
  el<HTMLButtonElement>("copy").disabled = !output;
}
function scheduleRender(): void {
  if (!renderFrame) renderFrame = requestAnimationFrame(renderOutput);
}
function populateSettings(): void {
  const { settings } = snapshot;
  el<HTMLInputElement>("endpoint").value = settings.endpoint;
  el<HTMLInputElement>("model").value = settings.model;
  el<HTMLInputElement>("shortcut").value = settings.shortcut;
  el<HTMLInputElement>("api-key").value = "";
  el<HTMLInputElement>("remove-key").checked = false;
  el("remove-key-row").hidden = !snapshot.hasApiKey;
  el("key-hint").textContent = snapshot.hasApiKey ? "密钥已保存；留空保持不变。" : "密钥保存在系统凭据库中。";
  el<HTMLInputElement>("autostart").checked = snapshot.autostart;
  el<HTMLSelectElement>("default-language").value = settings.targetLanguage;
  sourceLanguage.value = settings.sourceLanguage;
  targetLanguage.value = settings.targetLanguage;
  el("shortcut-label").textContent = settings.shortcut.split("+").join(" + ");
  el("model-label").textContent = settings.model || "尚未配置模型";
  el("model-label").title = settings.model;
  el("setup-notice").hidden = Boolean(settings.endpoint && settings.model);
}

async function startTranslation(): Promise<void> {
  if (activeRequest || !source.value.trim()) return;
  if (!bridge.desktop) { reportError("请在桌面应用中配置模型并翻译；浏览器仅供界面预览。"); return; }
  if (!snapshot.settings.endpoint || !snapshot.settings.model) {
    openView(true); showStatus("先填写 API 请求地址与模型名称。"); return;
  }
  clearStatus();
  const id = crypto.randomUUID();
  activeRequest = id;
  stopping = false;
  output = "";
  renderOutput();
  el("result-state").textContent = "正在连接模型…";
  updateControls();
  try {
    await bridge.translate({ id, text: source.value, sourceLanguage: sourceLanguage.value, targetLanguage: targetLanguage.value }, (delta) => {
      if (activeRequest !== id || stopping) return;
      output += delta;
      el("result-state").textContent = "正在翻译…";
      scheduleRender();
    });
    el("result-state").textContent = stopping ? "已停止" : "翻译完成";
  } catch (error) {
    if (stopping || String(error) === "已取消翻译") el("result-state").textContent = "已停止";
    else { el("result-state").textContent = "翻译未完成"; reportError(error); }
  } finally {
    if (renderFrame) cancelAnimationFrame(renderFrame);
    renderOutput();
    activeRequest = null;
    stopping = false;
    updateControls();
  }
}

el("translation-tab").addEventListener("click", () => openView(false));
el("settings-tab").addEventListener("click", () => openView(true));
el("setup-link").addEventListener("click", () => openView(true));
document.querySelector(".brand")!.addEventListener("click", (event) => { event.preventDefault(); openView(false); });
source.addEventListener("input", updateControls);
translateButton.addEventListener("click", () => void startTranslation());
el("cancel").addEventListener("click", async () => {
  if (!activeRequest) return;
  stopping = true;
  updateControls();
  try { await bridge.cancelTranslation(activeRequest); }
  catch (error) { stopping = false; updateControls(); reportError(error); }
});
el("clear").addEventListener("click", () => { source.value = ""; output = ""; renderOutput(); updateControls(); clearStatus(); el("result-state").textContent = "等待翻译"; source.focus(); });
el("paste").addEventListener("click", async () => {
  try {
    const text = await bridge.readClipboard();
    source.value = text.slice(0, 20000);
    updateControls(); source.focus();
    if (text.length > 20000) showStatus("已粘贴前 20,000 个字符，请分段翻译。");
  } catch { reportError("无法读取剪贴板，请使用 Ctrl+V 粘贴文本。"); }
});
el("copy").addEventListener("click", async () => {
  try { await bridge.writeClipboard(output); showStatus("译文已复制。"); }
  catch (error) { reportError(error); }
});
el("swap").addEventListener("click", () => {
  if (sourceLanguage.value === "auto") { showStatus("先指定原文语言，再交换翻译方向。"); return; }
  const previous = sourceLanguage.value;
  sourceLanguage.value = targetLanguage.value;
  targetLanguage.value = previous;
  if (output) { source.value = output.slice(0, 20000); output = ""; renderOutput(); }
  el("result-state").textContent = "等待翻译";
  updateControls();
});
el("hide").addEventListener("click", () => void bridge.hideWindow().catch(reportError));
document.addEventListener("keydown", (event) => {
  if (event.isComposing) return;
  if ((event.ctrlKey || event.metaKey) && event.key === "Enter" && !el("translation-view").hidden) { event.preventDefault(); void startTranslation(); }
  if (event.key === "Escape" && bridge.desktop) void bridge.hideWindow().catch(reportError);
});
document.addEventListener("visibilitychange", () => { if (!document.hidden) renderOutput(); });

el("settings-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  if (activeRequest) { showStatus("请等待翻译完成后再修改设置。"); return; }
  const button = el<HTMLButtonElement>("save");
  button.disabled = true;
  clearStatus();
  const settings: Settings = {
    endpoint: el<HTMLInputElement>("endpoint").value.trim(),
    model: el<HTMLInputElement>("model").value.trim(),
    shortcut: el<HTMLInputElement>("shortcut").value.trim(),
    sourceLanguage: sourceLanguage.value,
    targetLanguage: el<HTMLSelectElement>("default-language").value,
  };
  try {
    const key = el<HTMLInputElement>("api-key").value.trim();
    snapshot = await bridge.saveSettings(settings, key || null, el<HTMLInputElement>("remove-key").checked);
    populateSettings();
    showStatus(snapshot.warning ?? (settings.endpoint && settings.model ? "设置已保存，可以开始翻译了。" : "设置已保存。配置翻译服务后即可开始翻译。"), Boolean(snapshot.warning));
  } catch (error) { reportError(error); }
  finally { button.disabled = false; }
});
el("endpoint").addEventListener("input", () => {
  const changed = el<HTMLInputElement>("endpoint").value.trim() !== snapshot.settings.endpoint;
  el("key-hint").textContent = changed ? "更换地址后，请为新服务重新填写密钥。" : snapshot.hasApiKey ? "密钥已保存；留空保持不变。" : "密钥保存在系统凭据库中。";
  el("remove-key-row").hidden = changed || !snapshot.hasApiKey;
  el<HTMLInputElement>("remove-key").checked = false;
});
el("autostart").addEventListener("change", async () => {
  const checkbox = el<HTMLInputElement>("autostart");
  checkbox.disabled = true;
  try { await bridge.setAutostart(checkbox.checked); snapshot.autostart = checkbox.checked; showStatus(checkbox.checked ? "已开启开机自启动。" : "已关闭开机自启动。"); }
  catch (error) { checkbox.checked = snapshot.autostart; reportError(error); }
  finally { checkbox.disabled = false; }
});

async function initialize(): Promise<void> {
  el("preview-notice").hidden = bridge.desktop;
  try {
    await bridge.onOpenSettings(() => openView(true));
    snapshot = await bridge.loadSettings();
    populateSettings();
    if (snapshot.openSettings) openView(true);
    if (snapshot.warning) showStatus(snapshot.warning, true);
    initialized = true;
  } catch (error) { reportError(error); }
  updateControls();
  if (!snapshot.openSettings) source.focus();
}
void initialize();

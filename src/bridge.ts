import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { defaultSettings } from "./types";
import type { Settings, SettingsSnapshot, TranslationRequest } from "./types";

export const desktop = isTauri();

export async function loadSettings(): Promise<SettingsSnapshot> {
  if (!desktop) {
    return { settings: { ...defaultSettings }, hasApiKey: false, autostart: false, warning: null, openSettings: false };
  }
  return invoke("load_settings");
}

export async function saveSettings(settings: Settings, apiKey: string | null, clearApiKey: boolean): Promise<SettingsSnapshot> {
  requireDesktop();
  return invoke("save_settings", { settings, apiKey, clearApiKey });
}

export async function translate(request: TranslationRequest, onDelta: (text: string) => void): Promise<void> {
  requireDesktop();
  const onChunk = new Channel<string>();
  onChunk.onmessage = onDelta;
  await invoke("translate", { request, onChunk });
}

export async function cancelTranslation(id: string): Promise<void> {
  if (desktop) await invoke("cancel_translation", { id });
}

export async function readClipboard(): Promise<string> {
  if (desktop) return invoke("read_clipboard");
  return navigator.clipboard.readText();
}

export async function writeClipboard(text: string): Promise<void> {
  if (desktop) await invoke("write_clipboard", { text });
  else await navigator.clipboard.writeText(text);
}

export async function hideWindow(): Promise<void> {
  requireDesktop();
  await invoke("hide_window");
}

export async function setAutostart(enabled: boolean): Promise<void> {
  requireDesktop();
  await invoke("set_autostart", { enabled });
}

export async function onOpenSettings(callback: () => void): Promise<void> {
  if (desktop) await listen("open-settings", callback);
}

function requireDesktop(): void {
  if (!desktop) throw new Error("当前是浏览器预览。请在 Glimtra 桌面应用中使用此功能。");
}

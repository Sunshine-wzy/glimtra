export interface Settings {
  endpoint: string;
  model: string;
  shortcut: string;
  sourceLanguage: string;
  targetLanguage: string;
}

export interface SettingsSnapshot {
  settings: Settings;
  hasApiKey: boolean;
  autostart: boolean;
  warning: string | null;
  openSettings: boolean;
}

export interface TranslationRequest {
  id: string;
  text: string;
  sourceLanguage: string;
  targetLanguage: string;
}

export const languages = [
  ["zh-CN", "简体中文"],
  ["zh-TW", "繁體中文"],
  ["en", "英语"],
  ["ja", "日语"],
  ["ko", "韩语"],
  ["fr", "法语"],
  ["de", "德语"],
  ["es", "西班牙语"],
] as const;

export const defaultSettings: Settings = {
  endpoint: "",
  model: "",
  shortcut: "Ctrl+Alt+T",
  sourceLanguage: "auto",
  targetLanguage: "zh-CN",
};

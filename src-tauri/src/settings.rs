use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+T";
const KEYRING_SERVICE: &str = "app.glimtra.desktop";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub endpoint: String,
    pub model: String,
    pub shortcut: String,
    pub source_language: String,
    pub target_language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            model: String::new(),
            shortcut: DEFAULT_SHORTCUT.into(),
            source_language: "auto".into(),
            target_language: "zh-CN".into(),
        }
    }
}

pub fn load(path: &Path) -> Result<Settings, String> {
    match fs::read_to_string(path) {
        Ok(json) => {
            serde_json::from_str(&json).map_err(|_| "设置文件损坏，请在设置页重新保存。".into())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(_) => Err("无法读取设置文件，请检查目录权限。".into()),
    }
}

pub fn persist(path: &Path, settings: &Settings) -> Result<(), String> {
    let directory = path.parent().ok_or("设置目录无效")?;
    fs::create_dir_all(directory).map_err(|_| "无法创建设置目录")?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|_| "无法写入设置目录")?;
    let json = serde_json::to_vec_pretty(settings).map_err(|_| "无法序列化设置")?;
    file.write_all(&json).map_err(|_| "无法写入设置文件")?;
    file.as_file().sync_all().map_err(|_| "无法保存设置文件")?;
    file.persist(path)
        .map_err(|_| "无法替换设置文件，请检查目录权限")?;
    Ok(())
}

pub fn validate_endpoint(endpoint: &str) -> Result<reqwest::Url, String> {
    if endpoint.len() > 2048 {
        return Err("API 请求地址过长。".into());
    }
    let url = reqwest::Url::parse(endpoint).map_err(|_| "请输入有效的完整 API 请求地址。")?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("远程 API 请使用 HTTPS；只有本地回环地址允许 HTTP。".into());
    }
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "API 地址不能包含用户名、密码、查询参数或片段；密钥请填写在 API Key 中。".into(),
        );
    }
    Ok(url)
}

pub fn language_name(code: &str) -> Option<&'static str> {
    match code {
        "auto" => Some("automatically detected language"),
        "zh-CN" => Some("Simplified Chinese"),
        "zh-TW" => Some("Traditional Chinese"),
        "en" => Some("English"),
        "ja" => Some("Japanese"),
        "ko" => Some("Korean"),
        "fr" => Some("French"),
        "de" => Some("German"),
        "es" => Some("Spanish"),
        _ => None,
    }
}

pub fn validate(settings: &Settings) -> Result<(), String> {
    // Desktop preferences can be saved before a provider has been configured.
    if !settings.endpoint.is_empty() || !settings.model.is_empty() {
        validate_endpoint(&settings.endpoint)?;
        if settings.model.trim().is_empty() || settings.model.len() > 200 {
            return Err("请填写有效的模型名称（不超过 200 字节）。".into());
        }
    }
    if settings.shortcut.len() > 80 || settings.shortcut.is_empty() {
        return Err("请填写有效的快捷键。".into());
    }
    if language_name(&settings.source_language).is_none()
        || language_name(&settings.target_language).is_none()
        || settings.target_language == "auto"
    {
        return Err("语言选项无效。".into());
    }
    Ok(())
}

fn entry(endpoint: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, endpoint).map_err(|_| "系统凭据库不可用。".into())
}

pub fn read_key(endpoint: &str) -> Result<Option<String>, String> {
    if endpoint.is_empty() {
        return Ok(None);
    }
    match entry(endpoint)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("无法读取系统凭据库中的 API Key，请检查凭据库是否已解锁。".into()),
    }
}

pub fn write_key(endpoint: &str, key: Option<&str>) -> Result<(), String> {
    let entry = entry(endpoint)?;
    match key {
        Some(value) => entry
            .set_password(value)
            .map_err(|_| "无法将 API Key 写入系统凭据库。".into()),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("无法删除系统凭据库中的 API Key。".into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_requires_tls_except_loopback_and_rejects_embedded_secrets() {
        for valid in [
            "https://api.example.com/v1/chat/completions",
            "http://localhost:11434/v1/chat/completions",
            "http://127.0.0.1:8080/chat/completions",
            "http://[::1]:8080/chat/completions",
        ] {
            assert!(validate_endpoint(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "http://api.example.com/v1",
            "https://user:secret@example.com/v1",
            "https://example.com/v1?key=secret",
            "file:///etc/passwd",
            "not a URL",
        ] {
            assert!(validate_endpoint(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn settings_roundtrip_and_atomic_overwrite() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        assert_eq!(load(&path).unwrap().shortcut, DEFAULT_SHORTCUT);
        let mut settings = Settings::default();
        persist(&path, &settings).unwrap();
        settings.model = "updated-model".into();
        persist(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap().model, "updated-model");
        assert!(!fs::read_to_string(path).unwrap().contains("apiKey"));
    }
}

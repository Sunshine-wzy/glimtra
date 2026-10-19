mod settings;
mod stream;
mod translation;

use settings::Settings;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};
use tauri::{
    ipc::Channel,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WebviewWindow, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tokio_util::sync::CancellationToken;
use translation::TranslationRequest;

struct ActiveRequest {
    id: String,
    cancel: CancellationToken,
}

struct AppState {
    settings: Mutex<Settings>,
    path: PathBuf,
    client: reqwest::Client,
    active: Mutex<Option<ActiveRequest>>,
    warning: Mutex<Option<String>>,
    registered_shortcut: Mutex<Option<Shortcut>>,
    open_settings: AtomicBool,
    window_actions: Mutex<()>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SettingsSnapshot {
    settings: Settings,
    has_api_key: bool,
    autostart: bool,
    warning: Option<String>,
    open_settings: bool,
}

fn snapshot(app: &AppHandle, state: &AppState) -> Result<SettingsSnapshot, String> {
    let settings = state.settings.lock().map_err(|_| "设置状态不可用")?.clone();
    let (has_api_key, key_warning) = match settings::read_key(&settings.endpoint) {
        Ok(key) => (key.is_some(), None),
        Err(error) => (false, Some(error)),
    };
    let (autostart, autostart_warning) = match app.autolaunch().is_enabled() {
        Ok(enabled) => (enabled, None),
        Err(_) => (false, Some("无法读取开机自启动状态。".to_owned())),
    };
    let warning = state
        .warning
        .lock()
        .map_err(|_| "状态不可用")?
        .clone()
        .or(key_warning)
        .or(autostart_warning);
    Ok(SettingsSnapshot {
        settings,
        has_api_key,
        autostart,
        warning,
        open_settings: state.open_settings.swap(false, Ordering::SeqCst),
    })
}

#[tauri::command]
fn load_settings(app: AppHandle, state: State<'_, AppState>) -> Result<SettingsSnapshot, String> {
    snapshot(&app, &state)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
    api_key: Option<String>,
    clear_api_key: bool,
) -> Result<SettingsSnapshot, String> {
    settings::validate(&settings)?;
    let shortcut: Shortcut = settings
        .shortcut
        .parse()
        .map_err(|_| "快捷键格式无效，例如 Ctrl+Alt+T。")?;
    if shortcut.mods.is_empty() {
        return Err("全局快捷键必须包含 Ctrl、Alt、Shift 或 Super 修饰键。".into());
    }
    if api_key
        .as_ref()
        .is_some_and(|key| key.len() > 4096 || key.contains(['\r', '\n']))
    {
        return Err("API Key 格式无效或过长。".into());
    }
    if clear_api_key && api_key.is_some() {
        return Err("请勿同时填写新密钥和勾选删除密钥。".into());
    }
    if (clear_api_key || api_key.is_some()) && settings.endpoint.is_empty() {
        return Err("请先填写 API 请求地址，再保存或删除密钥。".into());
    }
    let mut current = state.settings.lock().map_err(|_| "设置状态不可用")?;
    let mut registered = state
        .registered_shortcut
        .lock()
        .map_err(|_| "快捷键状态不可用")?;
    let changed_shortcut = registered.as_ref() != Some(&shortcut);
    if changed_shortcut {
        // Register first, preserving the previous shortcut if there is a conflict.
        app.global_shortcut()
            .register(shortcut)
            .map_err(|_| "快捷键已被占用或不受系统支持，请换一个组合。")?;
    }
    let mut previous_key = None;
    let changing_key = api_key.is_some() || clear_api_key;
    let save_result = (|| -> Result<(), String> {
        if changing_key {
            previous_key = settings::read_key(&settings.endpoint)?;
            settings::write_key(&settings.endpoint, api_key.as_deref())?;
        }
        if let Err(error) = settings::persist(&state.path, &settings) {
            if changing_key {
                settings::write_key(&settings.endpoint, previous_key.as_deref())
                    .map_err(|_| "设置保存失败，密钥回滚失败；请重新填写并保存密钥。")?;
            }
            return Err(error);
        }
        Ok(())
    })();
    if let Err(error) = save_result {
        if changed_shortcut {
            let _ = app.global_shortcut().unregister(shortcut);
        }
        return Err(error);
    }
    if changed_shortcut {
        if let Some(old) = *registered {
            let _ = app.global_shortcut().unregister(old);
        }
        *registered = Some(shortcut);
    }
    *current = settings;
    *state.warning.lock().map_err(|_| "状态不可用")? = None;
    drop(registered);
    drop(current);
    snapshot(&app, &state)
}

#[tauri::command]
async fn translate(
    state: State<'_, AppState>,
    request: TranslationRequest,
    on_chunk: Channel<String>,
) -> Result<(), String> {
    request.validate()?;
    let settings = state.settings.lock().map_err(|_| "设置状态不可用")?.clone();
    let key = settings::read_key(&settings.endpoint)?;
    let cancel = CancellationToken::new();
    {
        let mut active = state.active.lock().map_err(|_| "翻译状态不可用")?;
        if active.is_some() {
            return Err("已有翻译正在进行，请先停止或等待完成。".into());
        }
        *active = Some(ActiveRequest {
            id: request.id.clone(),
            cancel: cancel.clone(),
        });
    }
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err("已取消翻译".to_owned()),
        result = translation::run(&state.client, &settings, key.as_deref(), &request, |text| {
            on_chunk.send(text).map_err(|_| "翻译窗口已关闭。".to_owned())
        }) => result,
    };
    let mut active = state.active.lock().map_err(|_| "翻译状态不可用")?;
    if active
        .as_ref()
        .is_some_and(|active| active.id == request.id)
    {
        *active = None;
    }
    result
}

#[tauri::command]
fn cancel_translation(state: State<'_, AppState>, id: String) -> Result<(), String> {
    if let Some(active) = state.active.lock().map_err(|_| "翻译状态不可用")?.as_ref() {
        if active.id == id {
            active.cancel.cancel();
        }
    }
    Ok(())
}

#[tauri::command]
fn read_clipboard(app: AppHandle) -> Result<String, String> {
    app.clipboard()
        .read_text()
        .map_err(|_| "剪贴板中没有可读取的文本。".into())
}

#[tauri::command]
fn write_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|_| "无法写入剪贴板。".into())
}

#[tauri::command]
fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|_| "无法隐藏窗口。".into())
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|_| "无法更新开机自启动，请检查系统权限。".into())
}

fn show_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let window = if let Some(window) = app.get_webview_window("main") {
        window
    } else {
        WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?.build()?
    };
    window.unminimize()?;
    window.show()?;
    window.set_focus()?;
    Ok(window)
}

fn request_main(app: &AppHandle, toggle: bool, open_settings: bool) {
    let app = app.clone();
    // WebView2 creation from synchronous event handlers can deadlock on Windows.
    // Serialize on a worker so rapid tray/hotkey events cannot create duplicates.
    tauri::async_runtime::spawn_blocking(move || {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let Ok(_guard) = state.window_actions.lock() else {
            return;
        };
        if open_settings {
            state.open_settings.store(true, Ordering::SeqCst);
        }
        if toggle {
            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
                    let _ = window.hide();
                    return;
                }
            }
        }
        match show_main(&app) {
            Ok(window) => {
                if open_settings {
                    let _ = window.emit("open-settings", ());
                }
            }
            Err(_) => {
                if let Ok(mut warning) = state.warning.lock() {
                    *warning = Some("窗口打开失败，请确认 WebView2 已安装后重试。".into());
                }
            }
        }
    });
}

fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开 Glimtra", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .tooltip("Glimtra · 轻量翻译")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => request_main(app, false, false),
            "settings" => request_main(app, false, true),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                request_main(tray.app_handle(), false, false);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            request_main(app, false, false);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let Some(state) = app.try_state::<AppState>() else {
                        return;
                    };
                    let is_active = state
                        .registered_shortcut
                        .lock()
                        .is_ok_and(|registered| registered.as_ref() == Some(shortcut));
                    if is_active {
                        request_main(app, true, false);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("settings.json");
            let (settings, mut warning) = match settings::load(&path) {
                Ok(settings) => (settings, None),
                Err(error) => (Settings::default(), Some(error)),
            };
            let shortcut = settings.shortcut.parse::<Shortcut>().ok();
            let registered_shortcut =
                shortcut.filter(|shortcut| app.global_shortcut().register(*shortcut).is_ok());
            if registered_shortcut.is_none() {
                warning = Some("全局快捷键注册失败，请从托盘打开设置并更换快捷键。".into());
            }
            app.manage(AppState {
                settings: Mutex::new(settings),
                path,
                client: reqwest::Client::builder()
                    .connect_timeout(Duration::from_secs(15))
                    .timeout(Duration::from_secs(120))
                    .redirect(reqwest::redirect::Policy::none())
                    .user_agent("Glimtra/0.1.0")
                    .build()?,
                active: Mutex::new(None),
                warning: Mutex::new(warning),
                registered_shortcut: Mutex::new(registered_shortcut),
                open_settings: AtomicBool::new(false),
                window_actions: Mutex::new(()),
            });
            create_tray(app.handle())?;
            // A login launch creates only the native tray; initialize WebView on first use.
            if !std::env::args().any(|arg| arg == "--background") {
                show_main(app.handle())?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            translate,
            cancel_translation,
            read_clipboard,
            write_clipboard,
            hide_window,
            set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 Glimtra");
}

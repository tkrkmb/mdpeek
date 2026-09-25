//! テーマに合わせて、アプリのアイコンをライト用とダーク用で差し替える。
//! 差し替える先は、macOSでは Dock のアイコン、Linuxでは窓のアイコン。

use serde::Deserialize;
use tauri::{AppHandle, Manager};

const LIGHT: &[u8] = include_bytes!("../icons/icon.png");
const DARK: &[u8] = include_bytes!("../icons/icon-dark.png");

/// 実際に使っている色
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    fn png(self) -> &'static [u8] {
        match self {
            Appearance::Light => LIGHT,
            Appearance::Dark => DARK,
        }
    }
}

/// OSのテーマに合うほうのアイコンにする（起動直後、フロントエンドが決める前に使う）
pub fn follow_os(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let appearance = match window.theme() {
        Ok(tauri::Theme::Dark) => Appearance::Dark,
        _ => Appearance::Light,
    };
    apply(app, appearance);
}

/// アイコンを差し替える。ネイティブの操作は、メインスレッドからしか行えない
pub fn apply(app: &AppHandle, appearance: Appearance) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || set_native(&handle, appearance.png()));
}

#[cfg(target_os = "macos")]
fn set_native(_app: &AppHandle, png: &'static [u8]) {
    use objc2::{AllocAnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(png);
    let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
        return;
    };
    // SAFETY: メインスレッドで、有効な NSImage を渡している
    unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
}

#[cfg(target_os = "linux")]
fn set_native(app: &AppHandle, png: &'static [u8]) {
    use gtk::prelude::GtkWindowExt;
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    // Waylandでは、窓のアイコンが使われないことがある
    if let Ok(pixbuf) = gtk::gdk_pixbuf::Pixbuf::from_read(std::io::Cursor::new(png)) {
        gtk_window.set_icon(Some(&pixbuf));
    }
}

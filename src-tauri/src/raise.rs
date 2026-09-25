//! 同じファイルを開いている窓を、後からの `mdsight <file>` で前面に出す。
//! 窓ごとに、表示中の文書から決まる名前のUnixドメインソケットで待ち受ける。
//! 受け付けるのは「前面に出る」の1種類だけ。

use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::report;

/// 前面化の要求。これ以外のデータは読み捨てる
const REQUEST: &[u8] = b"raise\n";

/// いま待ち受けているソケット。差し替えるときは、古い方の待ち受けを止めてファイルを消す。
#[derive(Default)]
pub struct Listening(Mutex<Option<Owned>>);

struct Owned {
    socket: PathBuf,
    task: tauri::async_runtime::JoinHandle<()>,
}

/// ソケットを置く、自分のユーザーだけが読み書きできるディレクトリ。
/// 他人が先に作っていたり、権限が広かったりしたら使わない。
fn socket_dir() -> Result<PathBuf, String> {
    // SAFETY: getuid は引数を取らず、常に成功する
    let uid = unsafe { libc::getuid() };
    let dir = std::env::temp_dir().join(format!("mdsight-{uid}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(err) => return Err(format!("cannot create {}: {err}", dir.display())),
    }
    let meta = std::fs::symlink_metadata(&dir)
        .map_err(|err| format!("cannot inspect {}: {err}", dir.display()))?;
    if !meta.is_dir() || meta.uid() != uid || meta.permissions().mode() & 0o077 != 0 {
        return Err(format!("{} is not private to this user", dir.display()));
    }
    Ok(dir)
}

/// 正規化済みの絶対パスから決まるソケットの名前。
/// 版の違うmdsightの間でも同じ名前になるように、FNV-1a（64bit）を使う。
fn socket_name(document: &Path) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in document.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}.sock")
}

fn socket_path(document: &Path) -> Result<PathBuf, String> {
    Ok(socket_dir()?.join(socket_name(document)))
}

/// その文書を開いている窓に、前面に出るよう頼む。頼めたら true。
/// つながらなければ、残っているソケットのファイルを消して false を返す。
pub fn request(document: &Path) -> bool {
    let Ok(socket) = socket_path(document) else {
        return false;
    };
    match UnixStream::connect(&socket) {
        Ok(mut stream) => stream.write_all(REQUEST).is_ok(),
        Err(_) => {
            let _ = std::fs::remove_file(&socket);
            false
        }
    }
}

/// 表示中の文書の名前で待ち受ける（差し替える）。
/// 別の窓がすでにその名前で待ち受けていたら、待ち受けない。
pub fn listen(app: &AppHandle, document: &Path) {
    stop(app);
    let socket = match socket_path(document) {
        Ok(socket) => socket,
        Err(message) => {
            report(&message);
            return;
        }
    };
    if UnixStream::connect(&socket).is_ok() {
        return;
    }
    let _ = std::fs::remove_file(&socket);
    let listener = match UnixListener::bind(&socket).and_then(|listener| {
        listener.set_nonblocking(true)?;
        Ok(listener)
    }) {
        Ok(listener) => listener,
        Err(err) => {
            report(&format!("cannot listen on {}: {err}", socket.display()));
            return;
        }
    };
    let handle = app.clone();
    let task = tauri::async_runtime::spawn(async move {
        let listener = match tokio::net::UnixListener::from_std(listener) {
            Ok(listener) => listener,
            Err(err) => {
                report(&format!("cannot listen: {err}"));
                return;
            }
        };
        while let Ok((stream, _)) = listener.accept().await {
            let handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                if receive(stream).await {
                    bring_to_front(&handle);
                }
            });
        }
    });
    *app.state::<Listening>().0.lock().expect("listening lock") = Some(Owned { socket, task });
}

/// 要求を1つだけ読む。決まった要求でなければ false（接続は閉じる）
async fn receive(stream: tokio::net::UnixStream) -> bool {
    use tokio::io::AsyncReadExt;
    let mut buffer = [0u8; REQUEST.len()];
    let mut stream = stream.take(REQUEST.len() as u64);
    stream.read_exact(&mut buffer).await.is_ok() && buffer == REQUEST
}

/// 最小化を解いて、窓を前面に出し、フォーカスを移す
fn bring_to_front(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// フォーカスを移さずに、窓を他の窓より前に出す（Neovimでの入力を妨げない）。
/// Tauriには、アプリを切り替えずに重なり順だけを上げる操作が無いので、OSの機能を直接使う。
pub fn show_without_focus(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }
    // ネイティブのウィンドウは、メインスレッドからしか触れない
    let _ = app.run_on_main_thread(move || raise_native(&window));
}

#[cfg(target_os = "macos")]
fn raise_native(window: &tauri::WebviewWindow) {
    use objc2_app_kit::NSWindow;
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    // SAFETY: Tauriが返すのは、この窓の有効な NSWindow。メインスレッドで呼んでいる
    let ns_window = unsafe { &*pointer.cast::<NSWindow>() };
    // アプリを切り替えずに（フォーカスを移さずに）、窓だけを最前面に出す
    ns_window.orderFrontRegardless();
}

#[cfg(target_os = "linux")]
fn raise_native(window: &tauri::WebviewWindow) {
    use gtk::prelude::WidgetExt;
    let Ok(gtk_window) = window.gtk_window() else {
        return;
    };
    // フォーカスは移さずに、重なり順だけを上げる（Waylandでは効かないことがある）
    if let Some(gdk_window) = gtk_window.window() {
        gdk_window.raise();
    }
}

/// 待ち受けを止めて、自分のソケットを消す
pub fn stop(app: &AppHandle) {
    let owned = app.state::<Listening>().0.lock().expect("listening lock").take();
    if let Some(owned) = owned {
        owned.task.abort();
        let _ = std::fs::remove_file(&owned.socket);
    }
}

#[cfg(test)]
mod tests {
    use super::socket_name;
    use std::path::Path;

    #[test]
    fn names_the_same_document_the_same_way() {
        assert_eq!(
            socket_name(Path::new("/tmp/a.md")),
            socket_name(Path::new("/tmp/a.md"))
        );
    }

    #[test]
    fn names_different_documents_differently() {
        assert_ne!(
            socket_name(Path::new("/tmp/a.md")),
            socket_name(Path::new("/tmp/b.md"))
        );
    }

    #[test]
    fn keeps_the_name_stable_across_builds() {
        // FNV-1a の既知の値。変わると、版の違うmdsightの窓を見つけられなくなる
        assert_eq!(socket_name(Path::new("")), "cbf29ce484222325.sock");
        assert_eq!(socket_name(Path::new("a")), "af63dc4c8601ec8c.sock");
    }
}

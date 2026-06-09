// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  apply_linux_webkit_workarounds();
  oceanln_desktop_lib::run();
}

/// Work around WebKitGTK crashing on Linux before the webview is created.
///
/// Since WebKitGTK 2.42 the webview composites frames into DMA-BUF buffers
/// allocated through GBM/EGL. On NVIDIA, AArch64 and many virtualized GPUs that
/// path fails to construct a framebuffer and WebKit calls `abort()`, taking the
/// whole app down (the stack runs entirely through `libwebkit2gtk-4.1.so.0`).
/// See tauri-apps/tauri#9304 and #9394.
///
/// We disable the DMA-BUF renderer (the targeted fix) and accelerated
/// compositing (the broader fallback that also fixes crash-on-resize). These
/// must be set before GTK/WebKit initializes, so this runs first thing in
/// `main()` — before `run()` builds any window.
///
/// Notes for production:
/// - Linux-only: the vars are meaningless elsewhere and the branch is compiled
///   out on other targets, yet still type-checked everywhere (CI catches breaks).
/// - We never override a value the user/operator already set, so acceleration
///   can be force-enabled via the real environment when a machine supports it.
/// - Safe for this app: the only known regression is black rounded corners on
///   *transparent* windows; ours are opaque.
fn apply_linux_webkit_workarounds() {
  // `cfg!(...)` (runtime) rather than `#[cfg(...)]` (attribute) so the body is
  // compiled and type-checked on macOS/Windows CI too, then dead-code-eliminated
  // off Linux. `std::env` exists on every target, so there is no downside.
  if !cfg!(target_os = "linux") {
    return;
  }

  for (key, value) in [
    ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
    ("WEBKIT_DISABLE_COMPOSITING_MODE", "1"),
  ] {
    if std::env::var_os(key).is_none() {
      std::env::set_var(key, value);
    }
  }
}

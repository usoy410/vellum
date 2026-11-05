mod ipc;
use gdk::RGBA;
use gtk::prelude::*;
use gtk_layer_shell::LayerShell;
use std::cell::RefCell;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::thread;
use webkit2gtk::{SettingsExt, WebContextExt, WebViewExt};

/// Formats and displays the command-line usage and options for Vellum.
fn print_help() {
    println!(
        "\
Vellum {version} - Lightweight HTML wallpaper runner for Linux

USAGE:
    vellum [OPTIONS] [PATH_OR_URL]
    vellum <COMMAND> [ARGS]

COMMANDS:
    set [-c|--cache] <PATH_OR_URL>    Set wallpaper to local HTML file or web URL
    refresh, reload [-c] [PATH_OR_URL] Reload wallpaper and bypass cache to show latest changes
    restart                           Restart the Vellum wallpaper service / daemon
    daemon [PATH_OR_URL]              Run Vellum wallpaper runner in daemon mode
    help                              Print this help message

OPTIONS:
    -r, --refresh                     Reload current wallpaper bypassing cache
    -c, --cache, --clear-cache        Bypass and clear WebKit cache (used with refresh/set)
    --config <PATH>                   Path to HTML wallpaper file or URL (daemon mode)
    -v, --version                     Print version information
    -h, --help                        Print help information

EXAMPLES:
    vellum ~/htmlwall/index.html      Set wallpaper (updates running instance, or starts daemon)
    vellum refresh                    Reload wallpaper bypassing cache to see HTML/CSS/JS edits
    vellum refresh -c                 Reload wallpaper and explicitly purge WebKit cache
    vellum set ~/wallpapers/bg.html   Set wallpaper on running instance
    vellum restart                    Restart Vellum service",
        version = env!("CARGO_PKG_VERSION")
    );
}

/// Resolves a file path or URL into a fully-qualified URL string.
///
/// If `input` is already an HTTP, HTTPS, or file URL, it is returned as-is.
/// Otherwise, tilde (`~`) prefixes are expanded and the path is resolved to an absolute `file://` URL.
fn resolve_url(input: &str) -> Result<String, Box<dyn std::error::Error>> {
    if input.starts_with("http://") || input.starts_with("https://") || input.starts_with("file://") {
        return Ok(input.to_string());
    }

    let expanded = if input.starts_with("~/") || input == "~" {
        if let Some(home) = std::env::var_os("HOME") {
            let mut p = std::path::PathBuf::from(home);
            if input.len() > 1 {
                p.push(&input[2..]);
            }
            p
        } else {
            std::path::PathBuf::from(input)
        }
    } else {
        std::path::PathBuf::from(input)
    };

    let canonical = match expanded.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            if expanded.is_absolute() {
                expanded
            } else {
                std::env::current_dir()?.join(expanded)
            }
        }
    };

    Ok(format!("file://{}", canonical.to_string_lossy()))
}

/// Transmits a request payload across the local Unix domain IPC socket to the active daemon.
///
/// Bounded to `MAX_REQUEST_BYTES` and governed by `REQUEST_TIMEOUT` to prevent hanging callers.
fn run_client(payload: String) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = ipc::socket_path();
    let mut stream = UnixStream::connect(&socket_path).map_err(|e| {
        format!(
            "Failed to connect to Vellum IPC socket at {}: {}. Is Vellum running?",
            socket_path.display(),
            e
        )
    })?;

    if payload.len() > ipc::MAX_REQUEST_BYTES {
        return Err("request payload exceeds maximum limit of 64 KiB".into());
    }

    stream.set_write_timeout(Some(ipc::REQUEST_TIMEOUT))?;
    stream.write_all(payload.as_bytes())?;
    Ok(())
}

/// Handles the `refresh` (or `reload`, `-r`) command.
///
/// Sends a cache-bypassing reload command to the running daemon. If an optional path is given,
/// that wallpaper is loaded while purging the WebKit cache.
fn handle_refresh(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut target_path: Option<&str> = None;

    for arg in args {
        match arg.as_str() {
            "-c" | "--cache" | "--clear-cache" => {
                // Cache bypass is default on refresh; flag accepted for explicit clarity
            }
            _ if !arg.starts_with('-') => {
                target_path = Some(arg.as_str());
            }
            _ => {
                eprintln!("Unknown flag for refresh: {}", arg);
                eprintln!("Usage: vellum refresh [-c|--cache] [path_or_url]");
                std::process::exit(1);
            }
        }
    }

    if !ipc::is_daemon_running() {
        eprintln!("Error: No running Vellum instance found.");
        eprintln!("Start Vellum with 'systemctl --user start vellum.service' or 'vellum <path>'.");
        std::process::exit(1);
    }

    if let Some(path) = target_path {
        let url = resolve_url(path)?;
        run_client(format!("refresh:{}", url))?;
        println!("Wallpaper updated and refreshed (cache cleared): {}", path);
    } else {
        run_client("refresh".to_string())?;
        println!("Wallpaper refreshed (cache cleared).");
    }

    Ok(())
}

/// Handles the `set` command to change the wallpaper on a running instance.
fn handle_set(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut clear_cache = false;
    let mut target_path: Option<&str> = None;

    for arg in args {
        match arg.as_str() {
            "-c" | "--cache" | "--clear-cache" => {
                clear_cache = true;
            }
            _ if !arg.starts_with('-') => {
                target_path = Some(arg.as_str());
            }
            _ => {
                eprintln!("Unknown flag for set: {}", arg);
                eprintln!("Usage: vellum set [-c|--cache] <path_or_url>");
                std::process::exit(1);
            }
        }
    }

    let path_str = match target_path {
        Some(p) => p,
        None => {
            eprintln!("Error: 'vellum set' requires a path or URL.");
            eprintln!("Usage: vellum set [-c|--cache] <path_or_url>");
            std::process::exit(1);
        }
    };

    let url = resolve_url(path_str)?;

    if !ipc::is_daemon_running() {
        eprintln!("Error: No running Vellum instance found.");
        eprintln!("Start Vellum with 'systemctl --user start vellum.service' or 'vellum <path>'.");
        std::process::exit(1);
    }

    if clear_cache {
        run_client(format!("refresh:{}", url))?;
        println!("Wallpaper updated (cache cleared): {}", path_str);
    } else {
        run_client(url)?;
        println!("Wallpaper updated: {}", path_str);
    }

    Ok(())
}

/// Restarts the Vellum service using the systemd user manager if active, or reloads running daemon.
fn handle_restart() -> Result<(), Box<dyn std::error::Error>> {
    let systemctl_status = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "vellum.service"])
        .status();

    if let Ok(status) = systemctl_status {
        if status.success() {
            println!("Restarting Vellum service via systemd...");
            let restart = std::process::Command::new("systemctl")
                .args(["--user", "restart", "vellum.service"])
                .status()?;
            if restart.success() {
                println!("Vellum service restarted successfully.");
                return Ok(());
            } else {
                eprintln!("Failed to restart vellum.service via systemctl.");
                std::process::exit(1);
            }
        }
    }

    if ipc::is_daemon_running() {
        println!("Vellum daemon is active. Refreshing current wallpaper...");
        run_client("refresh".to_string())?;
        println!("Wallpaper refreshed.");
        return Ok(());
    }

    eprintln!("No running Vellum instance found.");
    eprintln!("Start Vellum with 'systemctl --user start vellum.service' or 'vellum <path>'.");
    std::process::exit(1);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        // No arguments: default to starting daemon mode
        return run_daemon(args);
    }

    match args[1].as_str() {
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        "-v" | "-V" | "--version" => {
            println!("vellum {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "refresh" | "reload" | "-r" | "--refresh" => handle_refresh(&args[2..]),
        "restart" => handle_restart(),
        "set" => handle_set(&args[2..]),
        "daemon" => run_daemon(args),
        _ => {
            if args[1] == "-c" || args[1] == "--config" {
                if ipc::is_daemon_running() && args.len() >= 3 {
                    let url = resolve_url(&args[2])?;
                    run_client(url)?;
                    println!("Wallpaper updated: {}", args[2]);
                    return Ok(());
                }
                return run_daemon(args);
            }

            // If args[1] is a wallpaper path or URL:
            if !args[1].starts_with('-') {
                if ipc::is_daemon_running() {
                    let url = resolve_url(&args[1])?;
                    run_client(url)?;
                    println!("Wallpaper updated: {}", args[1]);
                    return Ok(());
                } else {
                    return run_daemon(args);
                }
            }

            eprintln!("Unknown argument: {}", args[1]);
            eprintln!("Run 'vellum --help' for usage instructions.");
            std::process::exit(1);
        }
    }
}

/// Initializes GTK, creates the borderless desktop window (Layer Shell or X11 EWMH),
/// binds the Unix IPC socket, and runs the main event loop.
fn run_daemon(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GTK (fails if DISPLAY and WAYLAND_DISPLAY are absent)
    gtk::init()?;

    let (_socket_owner, listener) = match ipc::SocketOwner::bind(&ipc::socket_path()) {
        Ok(bound) => bound,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!("Error: Another Vellum instance is already running.");
            eprintln!(
                "Use 'vellum --help' for options, 'vellum <path>' to set wallpaper, or 'vellum refresh' to reload."
            );
            std::process::exit(1);
        }
        Err(e) => return Err(e.into()),
    };

    // Create a new GTK window
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Vellum Wallpaper");

    // Try to use GTK Layer Shell (Wayland)
    if gtk_layer_shell::is_supported() {
        window.init_layer_shell();
        window.set_layer(gtk_layer_shell::Layer::Background);

        // Anchor to all edges to cover the screen
        window.set_anchor(gtk_layer_shell::Edge::Top, true);
        window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
        window.set_anchor(gtk_layer_shell::Edge::Left, true);
        window.set_anchor(gtk_layer_shell::Edge::Right, true);

        // Don't reserve space for other windows
        window.set_exclusive_zone(-1);
    } else {
        // X11 Fallback
        window.set_type_hint(gdk::WindowTypeHint::Desktop);
        window.set_keep_below(true);
        window.stick(); // Show on all workspaces
        window.fullscreen();
    }

    // Make window transparent
    if let Some(screen) = GtkWindowExt::screen(&window) {
        if let Some(visual) = screen.rgba_visual() {
            window.set_visual(Some(&visual));
        }
    }
    window.set_app_paintable(true);
    window.set_decorated(false); // No borders or title bar

    // Create and configure WebKit WebView
    let webview = webkit2gtk::WebView::new();

    // Configure WebKit settings
    if let Some(settings) = WebViewExt::settings(&webview) {
        settings.set_enable_developer_extras(false);
        settings.set_enable_resizable_text_areas(false);
    }

    // Disable context menu
    webview.connect_context_menu(|_, _, _, _| glib::Propagation::Stop.into());

    // Set transparent background
    webview.set_background_color(&RGBA::new(0.0, 0.0, 0.0, 0.0));

    // Get initial URL if provided
    let mut config_path = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "daemon" => {} // Ignore daemon command keyword
            _ => {
                if config_path.is_none() && !args[i].starts_with('-') {
                    config_path = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let mut initial_url = None;
    if let Some(path_str) = config_path {
        let url = resolve_url(&path_str)?;
        webview.load_uri(&url);
        initial_url = Some(url);
    }

    let last_url = Rc::new(RefCell::new(initial_url));
    let last_url_clone = last_url.clone();

    let (tx, rx) = glib::MainContext::channel(glib::Priority::default());

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                if let Ok(payload) = ipc::read_request(&mut stream, ipc::REQUEST_TIMEOUT) {
                    if tx.send(payload).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let webview_clone = webview.clone();
    rx.attach(None, move |msg| {
        if msg == "refresh" || msg == "reload" || msg == "cmd:refresh" || msg == "cmd:reload" {
            if let Some(context) = webview_clone.web_context() {
                context.clear_cache();
            }
            if webview_clone.uri().is_some() {
                webview_clone.reload_bypass_cache();
            } else if let Some(ref url) = *last_url_clone.borrow() {
                webview_clone.load_uri(url);
            }
        } else if let Some(target_url) = msg
            .strip_prefix("refresh:")
            .or_else(|| msg.strip_prefix("cmd:refresh:"))
        {
            if let Some(context) = webview_clone.web_context() {
                context.clear_cache();
            }
            *last_url_clone.borrow_mut() = Some(target_url.to_string());
            webview_clone.load_uri(target_url);
        } else {
            *last_url_clone.borrow_mut() = Some(msg.clone());
            webview_clone.load_uri(&msg);
        }
        glib::ControlFlow::Continue
    });

    // Add webview to window
    window.add(&webview);

    // Handle window close
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        glib::Propagation::Proceed.into()
    });

    // Show everything
    window.show_all();

    // Start GTK main loop
    let signals: Vec<_> = [libc::SIGTERM, libc::SIGINT]
        .into_iter()
        .map(|signal| {
            glib::source::unix_signal_add_local(signal, || {
                gtk::main_quit();
                glib::ControlFlow::Continue
            })
        })
        .collect();
    gtk::main();
    for source in signals {
        source.remove();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url_schemes() {
        assert_eq!(
            resolve_url("https://example.com/index.html").unwrap(),
            "https://example.com/index.html"
        );
        assert_eq!(
            resolve_url("http://localhost:8080").unwrap(),
            "http://localhost:8080"
        );
        assert_eq!(
            resolve_url("file:///etc/hosts").unwrap(),
            "file:///etc/hosts"
        );
    }

    #[test]
    fn test_resolve_url_home_expansion() {
        let home = std::env::var("HOME").unwrap();
        let expected = format!("file://{}/test.html", home);
        assert_eq!(resolve_url("~/test.html").unwrap(), expected);
    }

    #[test]
    fn test_resolve_url_relative_path() {
        let current_dir = std::env::current_dir().unwrap();
        let resolved = resolve_url("Cargo.toml").unwrap();
        assert_eq!(
            resolved,
            format!("file://{}/Cargo.toml", current_dir.to_str().unwrap())
        );
    }
}

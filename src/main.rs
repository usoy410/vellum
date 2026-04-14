use gtk::prelude::*;
use webkit2gtk::{WebViewExt, SettingsExt};
use gtk_layer_shell::LayerShell;
use gdk::RGBA;
use std::process::Command;
use std::os::unix::net::{UnixListener, UnixStream};
use std::io::{Read, Write};
use std::thread;

fn get_socket_path() -> String {
    let uid = unsafe { libc::getuid() };
    format!("/run/user/{}/vellum.sock", uid)
}

fn kill_competitors() {
    let runners = [
        "swww",
        "swww-daemon",
        "hanabi",
        "hidamari",
        "linux-wallpaperengine",
        "awww",
    ];
    for runner in runners {
        // Use pkill to terminate other runners. Silent failure if not found.
        let _ = Command::new("pkill").arg("-x").arg(runner).status();
    }
}

fn run_client(url: String) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = get_socket_path();
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(url.as_bytes())?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Check for "set" command
    if args.len() >= 3 && args[1] == "set" {
        let path_str = &args[2];
        let url = if path_str.starts_with("http") {
            path_str.clone()
        } else {
            let path = std::path::Path::new(path_str).canonicalize()?;
            format!("file://{}", path.to_str().unwrap())
        };
        return run_client(url);
    }

    // Default to daemon mode
    run_daemon(args)
}

fn run_daemon(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Terminate competing runners first
    kill_competitors();

    // Initialize GTK
    gtk::init()?;

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
    webview.connect_context_menu(|_, _, _, _| {
        glib::Propagation::Stop.into()
    });

    // Set transparent background
    webview.set_background_color(&RGBA::new(0.0, 0.0, 0.0, 0.0));
    
    // Get initial URL if provided
    let mut config_path = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = Some(args[i+1].clone());
                    i += 1;
                }
            }
            "daemon" => {} // Ignore daemon command
            _ => {
                if config_path.is_none() && !args[i].starts_with('-') {
                    config_path = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    if let Some(path_str) = config_path {
        let url = if path_str.starts_with("http") {
            path_str
        } else {
            let path = std::path::Path::new(&path_str).canonicalize()?;
            format!("file://{}", path.to_str().unwrap())
        };
        webview.load_uri(&url);
    }

    // Setup IPC Listener
    let socket_path = get_socket_path();
    let _ = std::fs::remove_file(&socket_path); // Remove old socket if exists
    let listener = UnixListener::bind(&socket_path)?;

    let (tx, rx) = glib::MainContext::channel(glib::Priority::default());
    
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buffer = String::new();
                if stream.read_to_string(&mut buffer).is_ok() {
                    let _ = tx.send(buffer);
                }
            }
        }
    });

    let webview_clone = webview.clone();
    rx.attach(None, move |url| {
        webview_clone.load_uri(&url);
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
    gtk::main();

    Ok(())
}

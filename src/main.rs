use gtk::prelude::*;
use webkit2gtk::{WebViewExt, SettingsExt};
use gtk_layer_shell::LayerShell;
use gdk::RGBA;
use std::process::Command;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Get the path/URL from arguments
    let args: Vec<String> = std::env::args().collect();
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
            _ => {
                if config_path.is_none() && !args[i].starts_with('-') {
                    config_path = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let url = if let Some(path_str) = config_path {
        if path_str.starts_with("http") {
            path_str
        } else {
            let path = std::path::Path::new(&path_str).canonicalize()?;
            format!("file://{}", path.to_str().unwrap())
        }
    } else {
        eprintln!("Vellum: A web-based wallpaper runner.");
        eprintln!("Usage: vellum -c <path_to_html_or_url>");
        std::process::exit(1);
    };

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
    
    // Load the target URL
    webview.load_uri(&url);

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

//! GInjector - GS2 Development Environment
//!
//! A graphical IDE for GS2 scripting with integrated compilation and Frida injection.

mod app;
mod config;

use app::GInjectorApp;

fn main() -> eframe::Result {
    // Install panic handler to show errors
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC: {}", panic_info);
    }));

    // Setup options for the egui window
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "GInjector - GS2 Development Environment",
        options,
        Box::new(|cc| {
            // Setup dark theme
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            // Enable persistent state
            cc.egui_ctx.set_pixels_per_point(1.0);

            Ok(Box::new(GInjectorApp::new(cc)))
        }),
    )
}

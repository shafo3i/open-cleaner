pub mod app;
pub mod banner;
pub mod bloat_view;
pub mod conductor;
pub mod dialogs;
pub mod grid_view;
pub mod header;
pub mod logs_view;
pub mod model;
pub mod sidebar;
pub mod table_view;
pub mod theme;
pub mod toolbar;
pub mod uninstaller_view;
pub mod views;
pub mod widgets;

pub use app::SystemCleanerApp;

pub fn launch_gui() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([960.0, 600.0])
            .with_decorations(false) // Frameless window: custom 48px header acts as title bar
            .with_transparent(false)
            .with_title("System Cleaner"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "System Cleaner",
        native_options,
        Box::new(|cc| Ok(Box::new(SystemCleanerApp::new(cc)))),
    );
}

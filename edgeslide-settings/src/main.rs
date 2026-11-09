mod multifinger_ui;
mod ui;
use adw::prelude::*;
fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("io.github.edgeslide.Settings")
        .build();
    let quit = gio::SimpleAction::new("quit", None);
    let weak = app.downgrade();
    quit.connect_activate(move |_, _| {
        if let Some(app) = weak.upgrade() {
            app.quit();
        }
    });
    app.add_action(&quit);
    app.set_accels_for_action("app.quit", &["<Primary>q"]);
    app.connect_activate(ui::build);
    app.run()
}

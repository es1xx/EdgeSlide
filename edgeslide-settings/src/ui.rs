use adw::prelude::*;
use edgeslide_core::{Action, Config, Edge, IndicatorAccent, IndicatorPosition};
use std::{cell::RefCell, rc::Rc};
type Model = Rc<RefCell<Config>>;

pub fn build(app: &adw::Application) {
    if let Some(window) = app.active_window() {
        window.present();
        return;
    }
    new_window(app);
}
fn new_window(app: &adw::Application) -> gtk::Label {
    let loaded = Config::load();
    let error = loaded.as_ref().err().map(ToString::to_string);
    let model = Rc::new(RefCell::new(loaded.unwrap_or_default()));
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("EdgeSlide")
        .default_width(660)
        .default_height(780)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&adw::WindowTitle::new(
        "EdgeSlide",
        "Touchpad edge gestures",
    )));
    root.append(&header);
    let stack = adw::ViewStack::new();
    let general = adw::PreferencesPage::new();
    let gestures = adw::PreferencesPage::new();
    let advanced = adw::PreferencesPage::new();
    let feedback = gtk::Label::builder()
        .wrap(true)
        .margin_start(20)
        .margin_end(20)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    general_page(&general, &model, &feedback);
    edge_group(&gestures, &model, false);
    edge_group(&gestures, &model, true);
    action_group(&gestures, &model);
    advanced_page(&advanced, &model, &window, &feedback);
    stack.add_titled_with_icon(
        &general,
        Some("general"),
        "General",
        "input-touchpad-symbolic",
    );
    stack.add_titled_with_icon(
        &gestures,
        Some("gestures"),
        "Gestures",
        "preferences-system-symbolic",
    );
    stack.add_titled_with_icon(
        &advanced,
        Some("advanced"),
        "Advanced",
        "applications-engineering-symbolic",
    );
    let switcher = adw::ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();
    root.append(&switcher);
    stack.set_vexpand(true);
    root.append(&stack);
    root.append(&feedback);
    let apply = gtk::Button::with_label("Save and apply");
    apply.add_css_class("suggested-action");
    apply.set_margin_start(24);
    apply.set_margin_end(24);
    apply.set_margin_bottom(16);
    let state = model.clone();
    let message = feedback.clone();
    apply.connect_clicked(move |_| match state.borrow().save() {
        Ok(()) => service(
            vec!["try-restart"],
            &message,
            "Saved. Active daemon restarted.",
        ),
        Err(error) => message.set_text(&format!("Could not save: {error}")),
    });
    root.append(&apply);
    window.set_content(Some(&root));
    window.present();
    if let Some(error) = error {
        feedback.set_text(&format!(
            "{error}. Defaults shown; the original file is untouched until you save."
        ));
    }
    feedback
}
fn make_group(
    page: &adw::PreferencesPage,
    title: &str,
    description: &str,
) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder()
        .title(title)
        .description(description)
        .build();
    page.add(&group);
    group
}
fn toggle(
    group: &adw::PreferencesGroup,
    title: &str,
    value: bool,
    change: impl Fn(bool) + 'static,
) {
    let row = adw::SwitchRow::builder().title(title).active(value).build();
    row.connect_active_notify(move |row| change(row.is_active()));
    group.add(&row);
}
fn number(
    group: &adw::PreferencesGroup,
    title: &str,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    change: impl Fn(f64) + 'static,
) {
    let row = adw::ActionRow::builder().title(title).build();
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, min, max, step);
    scale.set_width_request(220);
    scale.set_draw_value(true);
    scale.set_digits(if step >= 1.0 { 0 } else { 1 });
    scale.set_value(value);
    scale.connect_value_changed(move |scale| change(scale.value()));
    row.add_suffix(&scale);
    group.add(&row);
}
fn general_page(page: &adw::PreferencesPage, model: &Model, message: &gtk::Label) {
    let section = make_group(
        page,
        "General",
        "The daemon works independently of this settings window.",
    );
    let state = model.clone();
    toggle(
        &section,
        "Enable edge gestures",
        model.borrow().enabled,
        move |v| state.borrow_mut().enabled = v,
    );
    touchpad_selector(&section, model, message);
    let status = gtk::Label::new(Some("Checking service…"));
    status.set_wrap(true);
    let row = adw::ActionRow::builder().title("Daemon status").build();
    row.add_suffix(&status);
    section.add(&row);
    refresh_status(&status);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (label, action) in [("Start", "start"), ("Stop", "stop"), ("Refresh", "status")] {
        let button = gtk::Button::with_label(label);
        let status = status.clone();
        let message = message.clone();
        button.connect_clicked(move |_| {
            if action == "status" {
                refresh_status(&status);
            } else {
                service(
                    vec![action],
                    &message,
                    "Service request completed. Refresh status to check.",
                );
            }
        });
        controls.append(&button);
    }
    let row = adw::ActionRow::builder().title("Service controls").build();
    row.add_suffix(&controls);
    section.add(&row);
    let row = adw::ActionRow::builder()
        .title("Start automatically at login")
        .subtitle("Applies immediately; does not start or stop the running daemon")
        .build();
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (label, action) in [("Enable", "enable"), ("Disable", "disable")] {
        let button = gtk::Button::with_label(label);
        let message = message.clone();
        button.connect_clicked(move |_| service(vec![action], &message, "Autostart updated."));
        controls.append(&button);
    }
    row.add_suffix(&controls);
    section.add(&row);
    let appearance = make_group(
        page,
        "Appearance",
        "A click-through overlay exits after 0.9 seconds without updates.",
    );
    let state = model.clone();
    toggle(
        &appearance,
        "Show on-screen feedback",
        model.borrow().indicator,
        move |v| state.borrow_mut().indicator = v,
    );
    let positions = gtk::StringList::new(&["Top", "Bottom", "Left", "Right"]);
    let position = adw::ComboRow::builder()
        .title("Indicator position")
        .model(&positions)
        .selected(match model.borrow().indicator_position {
            IndicatorPosition::Top => 0,
            IndicatorPosition::Bottom => 1,
            IndicatorPosition::Left => 2,
            IndicatorPosition::Right => 3,
        })
        .build();
    let state = model.clone();
    position.connect_selected_notify(move |row| {
        state.borrow_mut().indicator_position = match row.selected() {
            0 => IndicatorPosition::Top,
            2 => IndicatorPosition::Left,
            3 => IndicatorPosition::Right,
            _ => IndicatorPosition::Bottom,
        }
    });
    appearance.add(&position);
    let accents = gtk::StringList::new(&["Blue", "Amber", "Green"]);
    let accent = adw::ComboRow::builder()
        .title("Accent color")
        .model(&accents)
        .selected(match model.borrow().indicator_accent {
            IndicatorAccent::Blue => 0,
            IndicatorAccent::Amber => 1,
            IndicatorAccent::Green => 2,
        })
        .build();
    let state = model.clone();
    accent.connect_selected_notify(move |row| {
        state.borrow_mut().indicator_accent = match row.selected() {
            1 => IndicatorAccent::Amber,
            2 => IndicatorAccent::Green,
            _ => IndicatorAccent::Blue,
        }
    });
    appearance.add(&accent);
    let preview = gtk::Button::with_label("Preview saved appearance");
    let row = adw::ActionRow::builder().title("Indicator preview").build();
    row.add_suffix(&preview);
    appearance.add(&row);
    let message = message.clone();
    preview.connect_clicked(move |_| {
        let message = message.clone();
        glib::spawn_future_local(async move {
            let binary = std::env::current_exe()
                .unwrap_or_default()
                .with_file_name("edgeslide-indicator");
            if let Err(error) =
                run(vec![binary.to_string_lossy().into_owned(), "0:0.55".into()]).await
            {
                message.set_text(&format!("Preview failed: {error}"));
            }
        });
    });
}
fn edge_group(page: &adw::PreferencesPage, model: &Model, right: bool) {
    let group = make_group(
        page,
        if right { "Right edge" } else { "Left edge" },
        "Start at the edge and slide vertically. Lift your finger to reset a cancelled gesture.",
    );
    let edge = if right {
        model.borrow().right.clone()
    } else {
        model.borrow().left.clone()
    };
    let state = model.clone();
    toggle(&group, "Enabled", edge.enabled, move |v| {
        mutate_edge(&state, right, |e| e.enabled = v)
    });
    let choices = gtk::StringList::new(&["Disabled", "Volume", "Brightness"]);
    let row = adw::ComboRow::builder()
        .title("Action")
        .model(&choices)
        .selected(match edge.action {
            Action::Disabled => 0,
            Action::Volume => 1,
            Action::Brightness => 2,
        })
        .build();
    let state = model.clone();
    row.connect_selected_notify(move |row| {
        mutate_edge(&state, right, |edge| {
            edge.action = match row.selected() {
                1 => Action::Volume,
                2 => Action::Brightness,
                _ => Action::Disabled,
            }
        })
    });
    group.add(&row);
    let state = model.clone();
    number(
        &group,
        "Edge width (%)",
        edge.width * 100.0,
        1.0,
        20.0,
        1.0,
        move |v| mutate_edge(&state, right, |e| e.width = v / 100.0),
    );
    let state = model.clone();
    number(
        &group,
        "Activation distance (%)",
        edge.threshold * 100.0,
        0.5,
        30.0,
        0.5,
        move |v| mutate_edge(&state, right, |e| e.threshold = v / 100.0),
    );
    let state = model.clone();
    number(
        &group,
        "Sensitivity",
        edge.sensitivity,
        0.1,
        5.0,
        0.1,
        move |v| mutate_edge(&state, right, |e| e.sensitivity = v),
    );
    let state = model.clone();
    toggle(&group, "Invert direction", edge.invert, move |v| {
        mutate_edge(&state, right, |e| e.invert = v)
    });
}
fn mutate_edge(model: &Model, right: bool, change: impl FnOnce(&mut Edge)) {
    let mut config = model.borrow_mut();
    change(if right {
        &mut config.right
    } else {
        &mut config.left
    });
}
fn action_group(page: &adw::PreferencesPage, model: &Model) {
    let group = make_group(
        page,
        "Action limits and speed",
        "Brightness has a hard 5% safety floor. Volume above 100% may distort.",
    );
    let c = model.borrow().clone();
    let state = model.clone();
    number(
        &group,
        "Minimum volume (%)",
        c.volume_min * 100.0,
        0.0,
        100.0,
        1.0,
        move |v| state.borrow_mut().volume_min = v / 100.0,
    );
    let state = model.clone();
    number(
        &group,
        "Maximum volume (%)",
        c.volume_max * 100.0,
        1.0,
        150.0,
        1.0,
        move |v| state.borrow_mut().volume_max = v / 100.0,
    );
    let state = model.clone();
    number(
        &group,
        "Volume speed",
        c.volume_speed,
        0.1,
        5.0,
        0.1,
        move |v| state.borrow_mut().volume_speed = v,
    );
    let state = model.clone();
    number(
        &group,
        "Minimum brightness (%)",
        c.brightness_min * 100.0,
        5.0,
        80.0,
        1.0,
        move |v| state.borrow_mut().brightness_min = v / 100.0,
    );
    let state = model.clone();
    number(
        &group,
        "Brightness speed",
        c.brightness_speed,
        0.1,
        5.0,
        0.1,
        move |v| state.borrow_mut().brightness_speed = v,
    );
}
fn advanced_page(
    page: &adw::PreferencesPage,
    model: &Model,
    window: &adw::ApplicationWindow,
    message: &gtk::Label,
) {
    let group = make_group(
        page,
        "Maintenance",
        "Import and restore save immediately and refresh the settings window.",
    );
    for label in [
        "Restart service",
        "Import configuration",
        "Export configuration",
        "Restore defaults",
    ] {
        let button = gtk::Button::with_label(label);
        let row = adw::ActionRow::new();
        row.add_suffix(&button);
        group.add(&row);
        let model = model.clone();
        let window = window.clone();
        let message = message.clone();
        button.connect_clicked(move |_| match label {
            "Restart service" => service(vec!["restart"], &message, "Service restarted."),
            "Import configuration" => import(&window, &model, &message),
            "Export configuration" => export(&window, &model, &message),
            _ => {
                let dialog = adw::MessageDialog::builder()
                    .transient_for(&window)
                    .heading("Restore defaults?")
                    .body("Your saved EdgeSlide configuration will be replaced.")
                    .build();
                dialog.add_responses(&[("cancel", "Cancel"), ("restore", "Restore")]);
                dialog.set_response_appearance("restore", adw::ResponseAppearance::Destructive);
                let model = model.clone();
                let message = message.clone();
                let window = window.clone();
                dialog.connect_response(None, move |_, response| {
                    if response == "restore" {
                        let defaults = Config::default();
                        match defaults.save() {
                            Ok(()) => {
                                *model.borrow_mut() = defaults;
                                let message = reload_window(&window);
                                service(vec!["try-restart"], &message, "Defaults restored.");
                            }
                            Err(e) => message.set_text(&e.to_string()),
                        }
                    }
                });
                dialog.present();
            }
        });
    }
    let diagnostics = make_group(
        page,
        "Diagnostics",
        "Device access failures are reported in the user journal. No input events are recorded.",
    );
    let output = gtk::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .height_request(180)
        .build();
    let button = gtk::Button::with_label("Refresh diagnostics");
    diagnostics.add(&button);
    diagnostics.add(&output);
    button.connect_clicked(move |_| {
        let output = output.clone();
        glib::spawn_future_local(async move {
            let binary = std::env::current_exe()
                .unwrap_or_default()
                .with_file_name("edgeslide-daemon");
            let mut text = String::new();
            for args in [
                vec![
                    binary.to_string_lossy().into_owned(),
                    "--list-devices".into(),
                ],
                vec![
                    "journalctl".into(),
                    "--user".into(),
                    "-u".into(),
                    "edgeslide.service".into(),
                    "-n".into(),
                    "20".into(),
                    "--no-pager".into(),
                ],
            ] {
                text.push_str(&run(args).await.unwrap_or_else(|e| e.to_string()));
                text.push('\n');
            }
            output.buffer().set_text(&text);
        });
    });
}
async fn run(args: Vec<String>) -> anyhow::Result<String> {
    let refs: Vec<_> = args.iter().map(std::ffi::OsStr::new).collect();
    let process = gio::Subprocess::newv(
        &refs,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )?;
    let (stdout, stderr) = process.communicate_utf8_future(None).await?;
    let text = format!(
        "{}{}",
        stdout.unwrap_or_default(),
        stderr.unwrap_or_default()
    );
    if !process.is_successful() {
        anyhow::bail!("{}: {}", args[0], text.trim());
    }
    Ok(text)
}
fn service(actions: Vec<&'static str>, message: &gtk::Label, success: &'static str) {
    let message = message.clone();
    message.set_text("Working…");
    glib::spawn_future_local(async move {
        let mut args = vec!["systemctl".into(), "--user".into()];
        args.extend(actions.iter().map(|s| s.to_string()));
        args.push("edgeslide.service".into());
        match run(args).await {
            Ok(_) => message.set_text(success),
            Err(e) => message.set_text(&format!("{e}")),
        }
    });
}
fn refresh_status(status: &gtk::Label) {
    let status = status.clone();
    glib::spawn_future_local(async move {
        let args = [
            "systemctl",
            "--user",
            "show",
            "edgeslide.service",
            "--property=ActiveState,SubState,UnitFileState",
            "--value",
        ]
        .map(String::from)
        .to_vec();
        status.set_text(&run(args).await.unwrap_or_else(|e| e.to_string()));
    });
}
fn import(window: &adw::ApplicationWindow, model: &Model, message: &gtk::Label) {
    let dialog = gtk::FileDialog::builder()
        .title("Import EdgeSlide configuration")
        .build();
    let model = model.clone();
    let message = message.clone();
    let parent = window.clone();
    dialog.open(Some(window), gio::Cancellable::NONE, move |result| {
        let Ok(file) = result else {
            return;
        };
        let result = (|| -> anyhow::Result<()> {
            let path = file
                .path()
                .ok_or_else(|| anyhow::anyhow!("Select a local file"))?;
            if std::fs::metadata(&path)?.len() > 65536 {
                anyhow::bail!("Configuration exceeds 64 KiB");
            }
            let config = Config::parse(&std::fs::read_to_string(path)?)?;
            config.save()?;
            *model.borrow_mut() = config;
            Ok(())
        })();
        match result {
            Ok(()) => {
                let message = reload_window(&parent);
                service(vec!["try-restart"], &message, "Configuration imported.");
            }
            Err(e) => message.set_text(&format!("Import failed: {e}")),
        }
    });
}
fn export(window: &adw::ApplicationWindow, model: &Model, message: &gtk::Label) {
    let dialog = gtk::FileDialog::builder()
        .title("Export configuration")
        .initial_name("edgeslide.toml")
        .build();
    let config = model.borrow().clone();
    let message = message.clone();
    dialog.save(Some(window), gio::Cancellable::NONE, move |result| {
        let Ok(file) = result else {
            return;
        };
        let result = (|| -> anyhow::Result<()> {
            let path = file
                .path()
                .ok_or_else(|| anyhow::anyhow!("Select a local file"))?;
            config.validate()?;
            std::fs::write(path, config.to_toml()?)?;
            Ok(())
        })();
        message.set_text(&match result {
            Ok(()) => "Configuration exported.".into(),
            Err(e) => format!("Export failed: {e}"),
        });
    });
}

fn touchpad_selector(group: &adw::PreferencesGroup, model: &Model, message: &gtk::Label) {
    let names = gtk::StringList::new(&["Automatic"]);
    let row = adw::ComboRow::builder()
        .title("Touchpad")
        .model(&names)
        .build();
    group.add(&row);
    let state = model.clone();
    let message = message.clone();
    glib::spawn_future_local(async move {
        let binary = std::env::current_exe()
            .unwrap_or_default()
            .with_file_name("edgeslide-daemon");
        let result = run(vec![
            binary.to_string_lossy().into_owned(),
            "--list-devices".into(),
        ])
        .await;
        let mut devices = vec![String::new()];
        match result {
            Ok(output) => {
                for line in output.lines() {
                    if let Some((_, name)) = line.split_once('\t') {
                        devices.push(name.into());
                        names.append(name);
                    }
                }
            }
            Err(error) => message.set_text(&format!("Device discovery failed: {error}")),
        }
        let selected = state.borrow().touchpad.clone();
        if !devices.contains(&selected) {
            names.append(&selected);
            devices.push(selected.clone());
        }
        row.set_selected(
            devices
                .iter()
                .position(|name| *name == selected)
                .unwrap_or(0) as u32,
        );
        row.connect_selected_notify(move |row| {
            if let Some(name) = devices.get(row.selected() as usize) {
                state.borrow_mut().touchpad = name.clone();
            }
        });
    });
}

fn reload_window(window: &adw::ApplicationWindow) -> gtk::Label {
    let app = window
        .application()
        .unwrap()
        .downcast::<adw::Application>()
        .unwrap();
    let message = new_window(&app);
    window.close();
    message
}

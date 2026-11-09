//! Additive section: its own model, save button and defaults; existing controls are untouched.
use adw::prelude::*;
use edgeslide_core::{MultiConfig, Swipe, SwipeAction};
use std::{cell::RefCell, rc::Rc};
type Model = Rc<RefCell<MultiConfig>>;
pub fn add(page: &adw::PreferencesPage) {
    match MultiConfig::load() {
        Ok(config) => populate(page, config),
        Err(error) => {
            let group=adw::PreferencesGroup::builder().title("Multi-finger gestures").description(format!("Cannot load gestures.toml: {error}. Repair this file to enable editing; existing settings are untouched.")).build();
            page.add(&group);
        }
    }
}
fn mapping(config: &mut MultiConfig, index: usize) -> &mut Swipe {
    match index {
        0 => &mut config.four_left,
        1 => &mut config.four_right,
        2 => &mut config.three_down,
        _ => &mut config.three_up,
    }
}
fn populate(page: &adw::PreferencesPage, config: MultiConfig) {
    let group=adw::PreferencesGroup::builder().title("Multi-finger gestures").description("Native Hyprland gestures · separate from edge controls. Save below to apply. Existing compositor conflicts are preserved.").build();
    page.add(&group);
    let model = Rc::new(RefCell::new(config));
    let enabled = adw::SwitchRow::builder()
        .title("Enable multi-finger gestures")
        .active(model.borrow().enabled)
        .build();
    let state = model.clone();
    enabled.connect_active_notify(move |row| state.borrow_mut().enabled = row.is_active());
    group.add(&enabled);
    for (index, title) in [
        "4 fingers · swipe left",
        "4 fingers · swipe right",
        "3 fingers · swipe down",
        "3 fingers · swipe up",
    ]
    .into_iter()
    .enumerate()
    {
        add_mapping(&group, &model, index, title);
    }
    let message = gtk::Label::builder()
        .wrap(true)
        .xalign(0.0)
        .margin_top(8)
        .margin_bottom(8)
        .build();
    message.set_text("Overview is inactive by default: supply a command for an installed overview provider. Commands run as your user when the gesture completes.");
    group.add(&message);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let save = gtk::Button::with_label("Save multi-finger gestures");
    save.add_css_class("suggested-action");
    buttons.append(&save);
    let state = model.clone();
    let feedback = message.clone();
    save.connect_clicked(move |_| {
        if let Err(error) = state.borrow().save() {
            feedback.set_text(&error.to_string());
            return;
        }
        let feedback = feedback.clone();
        feedback.set_text("Applying native gestures…");
        glib::spawn_future_local(async move {
            let result = apply().await;
            feedback.set_text(&match result {
                Ok(message) => message,
                Err(error) => format!("Saved, but could not apply: {error}"),
            });
        });
    });
    let restore = gtk::Button::with_label("Restore gesture mappings");
    buttons.append(&restore);
    // A weak page avoids retaining the settings window after it closes.
    let weak = page.downgrade();
    let old = group.downgrade();
    restore.connect_clicked(move |_| {
        if let (Some(page), Some(old)) = (weak.upgrade(), old.upgrade()) {
            page.remove(&old);
            populate(&page, MultiConfig::default());
        }
    });
    group.add(&buttons);
}
fn add_mapping(group: &adw::PreferencesGroup, model: &Model, index: usize, title: &str) {
    let current = mapping(&mut model.borrow_mut(), index).clone();
    let row = adw::ExpanderRow::builder().title(title).build();
    group.add(&row);
    let enabled = adw::SwitchRow::builder()
        .title("Enabled")
        .active(current.enabled)
        .build();
    let state = model.clone();
    enabled.connect_active_notify(move |row| {
        mapping(&mut state.borrow_mut(), index).enabled = row.is_active()
    });
    row.add_row(&enabled);
    let choices = gtk::StringList::new(&[
        "Disabled",
        "Previous workspace",
        "Next workspace",
        "Area screenshot",
        "Workspace overview (requires provider)",
        "Custom command",
    ]);
    let actions = [
        SwipeAction::Disabled,
        SwipeAction::PreviousWorkspace,
        SwipeAction::NextWorkspace,
        SwipeAction::Screenshot,
        SwipeAction::Overview,
        SwipeAction::Custom,
    ];
    let action = adw::ComboRow::builder()
        .title("Action")
        .model(&choices)
        .selected(actions.iter().position(|v| *v == current.action).unwrap() as u32)
        .build();
    let state = model.clone();
    action.connect_selected_notify(move |row| {
        mapping(&mut state.borrow_mut(), index).action = actions[row.selected() as usize]
    });
    row.add_row(&action);
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.25, 4.0, 0.25);
    scale.set_width_request(200);
    scale.set_draw_value(true);
    scale.set_digits(2);
    scale.set_value(current.sensitivity);
    let state = model.clone();
    scale.connect_value_changed(move |scale| {
        mapping(&mut state.borrow_mut(), index).sensitivity = scale.value()
    });
    let sensitivity = adw::ActionRow::builder()
        .title("Sensitivity")
        .subtitle("Higher values require a shorter swipe")
        .build();
    sensitivity.add_suffix(&scale);
    row.add_row(&sensitivity);
    let command = adw::EntryRow::builder()
        .title("Command (overview/custom, optional screenshot override)")
        .text(&current.command)
        .build();
    let state = model.clone();
    command.connect_changed(move |row| {
        mapping(&mut state.borrow_mut(), index).command = row.text().to_string()
    });
    row.add_row(&command);
}
async fn apply() -> anyhow::Result<String> {
    let binary = std::env::current_exe()?.with_file_name("edgeslide-gestures");
    let process = gio::Subprocess::newv(
        &[binary.as_os_str(), std::ffi::OsStr::new("apply")],
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )?;
    let (output, error) = process.communicate_utf8_future(None).await?;
    let message = format!(
        "{}{}",
        output.unwrap_or_default(),
        error.unwrap_or_default()
    );
    if !process.is_successful() {
        anyhow::bail!("{message}");
    }
    Ok(message)
}

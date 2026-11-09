//! Read the same configuration and action types as COSMIC. User overrides are
//! applied by binding, including Disable and bindings reassigned to other actions.

use cosmic::{
    cosmic_config::{Config, ConfigGet, Error},
    iced::{
        Subscription,
        futures::{SinkExt, channel::mpsc},
        stream,
    },
};
use cosmic_settings_config::shortcuts::{
    self, Action, Binding, Shortcuts, SystemActions, action::System,
};
use std::{collections::BTreeMap, time::Duration};
use xkbcommon::xkb;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutRow {
    pub label: String,
    /// Each entry is an alternative chord. Keys sharing modifiers are compacted.
    pub chords: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub title: &'static str,
    pub rows: Vec<ShortcutRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cheatsheet {
    pub groups: Vec<Group>,
}

impl Default for Cheatsheet {
    fn default() -> Self {
        Self::from_bindings(Shortcuts::default(), Shortcuts::default(), &BTreeMap::new())
    }
}

impl Cheatsheet {
    pub fn load() -> Result<Self, String> {
        let context = shortcuts::context().map_err(|error| error.to_string())?;
        Self::load_from(&context)
    }

    fn load_from(context: &Config) -> Result<Self, String> {
        let defaults = context
            .get::<Shortcuts>("defaults")
            .map_err(|error| format!("Could not read COSMIC shortcut defaults: {error}"))?;
        let custom = optional::<Shortcuts>(context.get("custom"))
            .map_err(|error| format!("Could not read custom shortcuts: {error}"))?;
        let mut commands = optional::<SystemActions>(context.get_system_default("system_actions"))
            .map_err(|error| format!("Could not read system app commands: {error}"))?;
        commands.extend(
            optional::<SystemActions>(context.get_local("system_actions"))
                .map_err(|error| format!("Could not read preferred app commands: {error}"))?,
        );
        Ok(Self::from_bindings(defaults, custom, &commands))
    }

    fn from_bindings(mut defaults: Shortcuts, custom: Shortcuts, commands: &SystemActions) -> Self {
        defaults.0.extend(custom.0);
        let bindings = defaults;
        let row = |label: &str, action| ShortcutRow {
            label: label.to_owned(),
            chords: format_bindings(bindings.iter().filter_map(|(binding, candidate)| {
                (visible_binding(binding, candidate)
                    && matches_action(candidate, &action, commands))
                .then_some(binding)
            })),
        };

        let apps = Group {
            title: "Open apps",
            rows: vec![
                row("Terminal", Action::System(System::Terminal)),
                row("Browser", Action::System(System::WebBrowser)),
                row("File manager", Action::System(System::HomeFolder)),
                row("Launcher", Action::System(System::Launcher)),
                row("Screenshot", Action::System(System::Screenshot)),
            ],
        };
        let windows = Group {
            title: "Window management",
            rows: vec![
                row("Maximize window", Action::Maximize),
                row("Close window", Action::Close),
                row("Float / tile window", Action::ToggleWindowFloating),
                row("Swap positions", Action::SwapWindow),
                row("Window switcher", Action::System(System::WindowSwitcher)),
            ],
        };

        let mut workspaces = workspace_rows(&bindings);
        workspaces.push(row("Next workspace", Action::NextWorkspace));
        workspaces.push(row("Previous workspace", Action::PreviousWorkspace));
        workspaces.push(row(
            "Workspace overview",
            Action::System(System::WorkspaceOverview),
        ));
        Self {
            groups: vec![
                apps,
                windows,
                Group {
                    title: "Workspace management",
                    rows: workspaces,
                },
            ],
        }
    }

    pub fn print(&self) {
        for group in &self.groups {
            println!("{}", group.title);
            for row in &group.rows {
                let chords = if row.chords.is_empty() {
                    "Not set".to_owned()
                } else {
                    row.chords.join("  or  ")
                };
                println!("  {:<22} {chords}", row.label);
            }
        }
    }
}

fn optional<T: Default>(result: Result<T, Error>) -> Result<T, Error> {
    match result {
        Err(Error::NotFound | Error::NoConfigDirectory) => Ok(T::default()),
        other => other,
    }
}

fn matches_action(candidate: &Action, target: &Action, commands: &SystemActions) -> bool {
    if candidate == target {
        return true;
    }
    // Custom command shortcuts count only when they exactly match COSMIC's
    // configured command for this app. Never infer an app from arbitrary text.
    match (candidate, target) {
        (Action::Spawn(command), Action::System(system)) => {
            commands.get(system).is_some_and(|configured| {
                !configured.trim().is_empty() && configured.trim() == command.trim()
            })
        }
        _ => false,
    }
}

fn visible_binding(binding: &Binding, action: &Action) -> bool {
    // Hide the unwanted close-window alternative only after applying live
    // overrides. Never invent Super+Q if the user has disabled or reassigned it.
    !(action == &Action::Close
        && binding.modifiers == shortcuts::Modifiers::new().alt()
        && binding.key == Some(xkb::Keysym::F4))
}

fn modifiers(binding: &Binding) -> Vec<&'static str> {
    [
        (binding.modifiers.logo, "Super"),
        (binding.modifiers.ctrl, "Ctrl"),
        (binding.modifiers.alt, "Alt"),
        (binding.modifiers.shift, "Shift"),
    ]
    .into_iter()
    .filter_map(|(enabled, name)| enabled.then_some(name))
    .collect()
}

fn key_label(binding: &Binding) -> String {
    if let Some(key) = binding.key {
        let name = xkb::keysym_get_name(key);
        match name.as_str() {
            "Left" => "←".into(),
            "Right" => "→".into(),
            "Up" => "↑".into(),
            "Down" => "↓".into(),
            "space" => "Space".into(),
            "slash" => "/".into(),
            "Print" => "Print Screen".into(),
            "Return" => "Enter".into(),
            "Escape" => "Esc".into(),
            "BackSpace" => "Backspace".into(),
            "Page_Up" => "Page Up".into(),
            "Page_Down" => "Page Down".into(),
            _ if name.len() == 1 => name.to_uppercase(),
            _ => name,
        }
    } else if let Some(code) = binding.keycode {
        format!("Keycode {code}")
    } else {
        String::new()
    }
}

fn format_bindings<'a>(bindings: impl Iterator<Item = &'a Binding>) -> Vec<String> {
    let mut grouped: BTreeMap<Vec<&str>, Vec<String>> = BTreeMap::new();
    for binding in bindings {
        grouped
            .entry(modifiers(binding))
            .or_default()
            .push(key_label(binding));
    }
    let mut groups: Vec<_> = grouped.into_iter().collect();
    // Prefer the usual Super shortcuts, then fewer modifiers. Stable ordering
    // avoids changing the UI on each HashMap iteration/config notification.
    groups.sort_by(|(a, _), (b, _)| {
        (!a.contains(&"Super"), a.len(), a).cmp(&(!b.contains(&"Super"), b.len(), b))
    });
    groups
        .into_iter()
        .map(|(modifiers, mut keys)| {
            keys.sort_by(|a, b| {
                let rank = |key: &str| match key {
                    "←" => 0,
                    "→" => 1,
                    "↑" => 2,
                    "↓" => 3,
                    _ => 4,
                };
                (rank(a), a).cmp(&(rank(b), b))
            });
            keys.dedup();
            let prefix = modifiers.join(" + ");
            // Modifier-only bindings must remain separate from modified key chords.
            let modifier_only = keys.iter().any(String::is_empty);
            keys.retain(|key| !key.is_empty());
            let chord = if keys.is_empty() {
                prefix.clone()
            } else if prefix.is_empty() {
                keys.join(" / ")
            } else {
                format!("{prefix} + {}", keys.join(" / "))
            };
            if modifier_only && !keys.is_empty() {
                format!("{prefix}  or  {chord}")
            } else {
                chord
            }
        })
        .collect()
}

fn workspace_rows(bindings: &Shortcuts) -> Vec<ShortcutRow> {
    let numbered: Vec<Vec<&Binding>> = (1..=9)
        .map(|number| {
            bindings
                .iter()
                .filter_map(|(binding, action)| {
                    (*action == Action::Workspace(number)).then_some(binding)
                })
                .collect()
        })
        .collect();
    let uniform = numbered.iter().enumerate().all(|(index, entries)| {
        entries.len() == 1
            && entries[0].keycode.is_none()
            && key_label(entries[0]) == (index + 1).to_string()
            && numbered[0].len() == 1
            && modifiers(entries[0]) == modifiers(numbered[0][0])
    });
    if uniform {
        let prefix = modifiers(numbered[0][0]).join(" + ");
        return vec![ShortcutRow {
            label: "Workspace 1–9".into(),
            chords: vec![if prefix.is_empty() {
                "1–9".into()
            } else {
                format!("{prefix} + 1–9")
            }],
        }];
    }
    numbered
        .into_iter()
        .enumerate()
        .map(|(index, entries)| ShortcutRow {
            label: format!("Workspace {}", index + 1),
            chords: format_bindings(entries.into_iter()),
        })
        .collect()
}

pub fn subscription() -> Subscription<Result<Cheatsheet, String>> {
    Subscription::run(watch)
}

fn watch() -> impl cosmic::iced::futures::Stream<Item = Result<Cheatsheet, String>> {
    stream::channel(
        2,
        |mut output: mpsc::Sender<Result<Cheatsheet, String>>| async move {
            loop {
                let context = match shortcuts::context() {
                    Ok(context) => context,
                    Err(error) => {
                        _ = output.send(Err(error.to_string())).await;
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        continue;
                    }
                };
                match watch_context(&context, &mut output).await {
                    Ok(()) => return,
                    Err(error) => {
                        if output
                            .send(Err(format!("Live shortcut updates unavailable: {error}")))
                            .await
                            .is_err()
                        {
                            return;
                        }
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                }
            }
        },
    )
}

async fn watch_context(
    context: &Config,
    output: &mut mpsc::Sender<Result<Cheatsheet, String>>,
) -> Result<(), String> {
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let _watcher = context
        .watch(move |_, keys| {
            if keys
                .iter()
                .any(|key| matches!(key.as_str(), "custom" | "defaults" | "system_actions"))
            {
                // Coalesce notifications; the next read loads every key.
                _ = sender.try_send(());
            }
        })
        .map_err(|error| error.to_string())?;

    // Watch before reading to avoid missing a change during startup. Reload
    // complete snapshots so deleting an override also restores the defaults.
    loop {
        if output.send(Cheatsheet::load_from(context)).await.is_err() {
            return Ok(());
        }
        if receiver.recv().await.is_none() {
            return Err("Configuration watcher disconnected".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &str) -> Shortcuts {
        ron::from_str(value).unwrap()
    }

    fn find<'a>(sheet: &'a Cheatsheet, label: &str) -> &'a ShortcutRow {
        sheet
            .groups
            .iter()
            .flat_map(|group| &group.rows)
            .find(|row| row.label == label)
            .unwrap()
    }

    #[test]
    fn custom_reassignments_and_disable_replace_defaults_by_binding() {
        let defaults = parse(
            r#"{
            (modifiers: [Super], key: "t"): System(Terminal),
            (modifiers: [Super], key: "q"): Close,
            (modifiers: [Alt], key: "F4"): Close,
        }"#,
        );
        let custom = parse(
            r#"{
            (modifiers: [Super], key: "t", description: Some("My binding")): Maximize,
            (modifiers: [Super], key: "q"): Disable,
            (modifiers: [Super, Shift], key: "Return"): System(Terminal),
        }"#,
        );
        let sheet = Cheatsheet::from_bindings(defaults, custom, &BTreeMap::new());
        assert_eq!(find(&sheet, "Terminal").chords, ["Super + Shift + Enter"]);
        assert!(find(&sheet, "Close window").chords.is_empty());
        assert_eq!(find(&sheet, "Maximize window").chords, ["Super + T"]);
        assert!(find(&sheet, "Browser").chords.is_empty());
    }

    #[test]
    fn compact_workspaces_expand_when_any_workspace_is_customized_or_disabled() {
        let defaults = || {
            parse(&format!(
                "{{ {} }}",
                (1..=9)
                    .map(|n| { format!(r#"(modifiers: [Super], key: "{n}"): Workspace({n}),"#) })
                    .collect::<String>()
            ))
        };
        let sheet = Cheatsheet::from_bindings(defaults(), Shortcuts::default(), &BTreeMap::new());
        assert_eq!(find(&sheet, "Workspace 1–9").chords, ["Super + 1–9"]);
        let custom = parse(
            r#"{
            (modifiers: [Super], key: "3"): Disable,
            (modifiers: [Alt], key: "F5"): Workspace(5),
        }"#,
        );
        let sheet = Cheatsheet::from_bindings(defaults(), custom, &BTreeMap::new());
        assert!(find(&sheet, "Workspace 3").chords.is_empty());
        assert_eq!(
            find(&sheet, "Workspace 5").chords,
            ["Super + 5", "Alt + F5"]
        );
    }

    #[test]
    fn command_shortcuts_match_only_the_configured_app_command() {
        let bindings = parse(
            r#"{
            (modifiers: [Super], key: "Return"): Spawn("ghostty --gtk-single-instance=true"),
            (modifiers: [Super], key: "x"): Spawn("echo ghostty"),
        }"#,
        );
        let commands = BTreeMap::from([(
            System::Terminal,
            "ghostty --gtk-single-instance=true".into(),
        )]);
        let sheet = Cheatsheet::from_bindings(bindings, Shortcuts::default(), &commands);
        assert_eq!(find(&sheet, "Terminal").chords, ["Super + Enter"]);
    }

    #[test]
    fn close_alternative_is_hidden_without_losing_live_reassignments() {
        let defaults = || {
            parse(
                r#"{
            (modifiers: [Super], key: "q"): Close,
            (modifiers: [Alt], key: "F4"): Close,
        }"#,
            )
        };
        let sheet = Cheatsheet::from_bindings(defaults(), Shortcuts::default(), &BTreeMap::new());
        assert_eq!(find(&sheet, "Close window").chords, ["Super + Q"]);

        let custom = parse(
            r#"{
            (modifiers: [Super], key: "q"): System(WebBrowser),
            (modifiers: [Ctrl], key: "w"): Close,
            (modifiers: [Alt], key: "F4"): System(Screenshot),
        }"#,
        );
        let sheet = Cheatsheet::from_bindings(defaults(), custom, &BTreeMap::new());
        assert_eq!(find(&sheet, "Close window").chords, ["Ctrl + W"]);
        assert_eq!(find(&sheet, "Browser").chords, ["Super + Q"]);
        assert_eq!(find(&sheet, "Screenshot").chords, ["Alt + F4"]);
    }

    #[test]
    fn launcher_keeps_modifier_only_and_keyed_alternatives_distinct() {
        let defaults = parse(
            r#"{
            (modifiers: [Super]): System(Launcher),
            (modifiers: [Super], key: "slash"): System(Launcher),
            (modifiers: [], key: "Print"): System(Screenshot),
            (modifiers: [Super], key: "w"): System(WorkspaceOverview),
            (modifiers: [Alt], key: "Tab"): System(WindowSwitcher),
        }"#,
        );
        let sheet = Cheatsheet::from_bindings(defaults, Shortcuts::default(), &BTreeMap::new());
        assert_eq!(find(&sheet, "Launcher").chords, ["Super  or  Super + /"]);
        assert_eq!(find(&sheet, "Screenshot").chords, ["Print Screen"]);
        assert_eq!(find(&sheet, "Workspace overview").chords, ["Super + W"]);
        assert_eq!(find(&sheet, "Window switcher").chords, ["Alt + Tab"]);
    }

    #[test]
    fn alternatives_are_stable_and_compact_without_losing_modifiers() {
        let bindings = parse(
            r#"{
            (modifiers: [Super, Ctrl], key: "j"): NextWorkspace,
            (modifiers: [Super, Ctrl], key: "Down"): NextWorkspace,
            (modifiers: [Super, Ctrl], key: "Right"): NextWorkspace,
            (modifiers: [Alt], key: "n"): NextWorkspace,
        }"#,
        );
        let sheet = Cheatsheet::from_bindings(bindings, Shortcuts::default(), &BTreeMap::new());
        assert_eq!(
            find(&sheet, "Next workspace").chords,
            ["Super + Ctrl + → / ↓ / J", "Alt + N"]
        );
    }

    #[test]
    fn unknown_future_actions_do_not_break_known_shortcuts() {
        let bindings = parse(
            r#"{
            (modifiers: [Super], key: "t"): System(Terminal),
            (modifiers: [Super], key: "z"): FutureAction(Left),
        }"#,
        );
        let sheet = Cheatsheet::from_bindings(bindings, Shortcuts::default(), &BTreeMap::new());
        assert_eq!(find(&sheet, "Terminal").chords, ["Super + T"]);
    }

    #[test]
    fn config_load_handles_absent_custom_file_and_reports_malformed_overrides() {
        let directory = tempfile::tempdir().unwrap();
        let context = Config::with_custom_path(shortcuts::ID, 1, directory.path().into()).unwrap();
        let path = directory
            .path()
            .join("cosmic")
            .join(shortcuts::ID)
            .join("v1");
        std::fs::write(
            path.join("defaults"),
            r#"{ (modifiers: [Super], key: "t"): System(Terminal) }"#,
        )
        .unwrap();
        assert_eq!(
            find(&Cheatsheet::load_from(&context).unwrap(), "Terminal").chords,
            ["Super + T"]
        );
        std::fs::write(path.join("custom"), "{ invalid").unwrap();
        assert!(
            Cheatsheet::load_from(&context)
                .unwrap_err()
                .contains("custom shortcuts")
        );
        std::fs::remove_file(path.join("custom")).unwrap();
        assert!(Cheatsheet::load_from(&context).is_ok());
    }

    #[test]
    fn live_updates_handle_atomic_replacement_and_deletion() {
        use cosmic::iced::futures::StreamExt;

        let directory = tempfile::tempdir().unwrap();
        let context = Config::with_custom_path(shortcuts::ID, 1, directory.path().into()).unwrap();
        let path = directory
            .path()
            .join("cosmic")
            .join(shortcuts::ID)
            .join("v1");
        std::fs::write(
            path.join("defaults"),
            r#"{ (modifiers: [Super], key: "t"): System(Terminal) }"#,
        )
        .unwrap();

        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(async {
                let updates = stream::channel(2, move |mut output| async move {
                    watch_context(&context, &mut output).await.unwrap();
                });
                let mut updates = Box::pin(updates);
                let first = updates.next().await.unwrap().unwrap();
                assert_eq!(find(&first, "Terminal").chords, ["Super + T"]);

                std::fs::write(
                    path.join("custom.new"),
                    r#"{
                (modifiers: [Super], key: "t"): Disable,
                (modifiers: [Super], key: "Return"): System(Terminal),
            }"#,
                )
                .unwrap();
                std::fs::rename(path.join("custom.new"), path.join("custom")).unwrap();
                tokio::time::timeout(Duration::from_secs(3), async {
                    loop {
                        let sheet = updates.next().await.unwrap().unwrap();
                        if find(&sheet, "Terminal").chords == ["Super + Enter"] {
                            break;
                        }
                    }
                })
                .await
                .expect("atomic replacement must refresh the sheet");

                std::fs::remove_file(path.join("custom")).unwrap();
                tokio::time::timeout(Duration::from_secs(3), async {
                    loop {
                        let sheet = updates.next().await.unwrap().unwrap();
                        if find(&sheet, "Terminal").chords == ["Super + T"] {
                            break;
                        }
                    }
                })
                .await
                .expect("deleting custom must restore the defaults");
            });
    }
}

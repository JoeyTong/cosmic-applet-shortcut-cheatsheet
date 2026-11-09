use cosmic::{
    Element, app,
    iced::{
        self, Alignment, Border, Color, Length, Subscription, advanced::layout::Limits, window,
    },
    surface::{self, action},
    widget::{self, container, icon, scrollable, text},
};
use cosmic_applet_shortcut_cheatsheet::{
    APP_ID,
    popup::{Effect, PopupState},
    shortcuts::{self, Cheatsheet, Group, ShortcutRow},
};
use std::sync::LazyLock;

// Scale popup content together; changing only the surface size leaves text small.
const POPUP_SCALE: f32 = 1.25;
pub const WIDTH: f32 = 460.0 * POPUP_SCALE;
// Fit all selected default shortcuts, including alternative bindings and footer.
pub const MAX_HEIGHT: f32 = 720.0 * POPUP_SCALE;
static PANEL_ICON: LazyLock<icon::Handle> = LazyLock::new(|| {
    icon::from_svg_bytes(include_bytes!("../data/shortcut-cheatsheet-symbolic.svg").as_slice())
        .symbolic(true)
});

#[derive(Clone, Debug)]
pub enum Message {
    Click,
    Dismiss(window::Id),
    Closed(window::Id),
    Shortcuts(Result<Cheatsheet, String>),
}

pub struct Applet {
    core: app::Core,
    preview: bool,
    popup: PopupState,
    sheet: Cheatsheet,
    loaded: bool,
    error: Option<String>,
}

impl Applet {
    fn apply(&mut self, effect: Effect) -> app::Task<Message> {
        match effect {
            Effect::None => app::Task::none(),
            Effect::Open(id) => {
                let Some(parent) = self.core.main_window_id() else {
                    self.popup.closed(id);
                    return app::Task::none();
                };
                surface::surface_task(action::app_popup(
                    |_| Default::default(),
                    move |applet: &mut Self| {
                        let mut settings = applet
                            .core
                            .applet
                            .get_popup_settings(parent, id, None, None, None);
                        // Keep the cheatsheet open while the user works in other windows.
                        settings.grab = false;
                        settings.positioner.size_limits = popup_limits();
                        settings
                    },
                    None,
                ))
            }
            Effect::Close(id) => surface::surface_task(action::destroy_popup(id)),
        }
    }

    fn cheatsheet(&self) -> Element<'_, Message> {
        let theme = cosmic::theme::active();
        let spacing = theme.cosmic().spacing;

        if !self.loaded {
            return container(
                widget::column::with_children(vec![
                    section_header("Open apps", "view-app-grid-symbolic"),
                    popup_body("Shortcuts unavailable").into(),
                    popup_caption(self.error.as_deref().unwrap_or("Reading COSMIC settings…"))
                        .into(),
                ])
                .spacing(px(spacing.space_s.into())),
            )
            .padding(px(spacing.space_s.into()))
            .width(Length::Fill)
            .into();
        }

        let mut body = widget::column::with_children(vec![
            section(&self.sheet.groups[1], "focus-windows-symbolic"),
            section(&self.sheet.groups[2], "view-paged-symbolic"),
        ])
        .spacing(px(spacing.space_xs.into()));
        if self.error.is_some() {
            body = body.push(popup_caption(
                "Could not refresh shortcuts. Showing the last successfully read bindings.",
            ));
        }

        let hint = if self.preview {
            "Live COSMIC shortcuts"
        } else {
            "Click the panel icon to close"
        };
        let footer = popup_caption(hint).class(cosmic::theme::Text::Custom(muted_text));

        container(
            widget::column::with_children(vec![
                section(&self.sheet.groups[0], "view-app-grid-symbolic"),
                // Reserve space for Open apps and the footer before
                // allocating the remaining height to the scrolling groups.
                scrollable(body).height(Length::Fill).into(),
                container(footer)
                    .padding([0.0, px(spacing.space_xxs.into())])
                    .into(),
            ])
            .spacing(px(spacing.space_xs.into())),
        )
        .padding(px(spacing.space_s.into()))
        .width(Length::Fill)
        .max_height(MAX_HEIGHT)
        .into()
    }
}

impl cosmic::Application for Applet {
    type Executor = cosmic::executor::Default;
    type Flags = bool;
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut app::Core {
        &mut self.core
    }

    fn init(mut core: app::Core, preview: bool) -> (Self, app::Task<Message>) {
        core.window.show_headerbar = false;
        let (sheet, error, loaded) = match Cheatsheet::load() {
            Ok(sheet) => (sheet, None, true),
            Err(error) => {
                tracing::warn!(%error, "Could not load shortcuts");
                (Cheatsheet::default(), Some(error), false)
            }
        };
        (
            Self {
                core,
                preview,
                sheet,
                error,
                loaded,
                popup: PopupState::default(),
            },
            app::Task::none(),
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::event::listen_with(input_event),
            shortcuts::subscription().map(Message::Shortcuts),
        ])
    }

    fn update(&mut self, message: Message) -> app::Task<Message> {
        let effect = match message {
            Message::Click => self.popup.toggle(),
            Message::Dismiss(id) if self.preview && self.core.main_window_id() == Some(id) => {
                return iced::exit();
            }
            Message::Dismiss(id) => self.popup.dismiss(id),
            Message::Closed(id) if self.core.main_window_id() == Some(id) => return iced::exit(),
            Message::Closed(id) => {
                self.popup.closed(id);
                Effect::None
            }
            Message::Shortcuts(Ok(sheet)) => {
                self.sheet = sheet;
                self.loaded = true;
                self.error = None;
                Effect::None
            }
            Message::Shortcuts(Err(error)) => {
                tracing::warn!(%error, "Could not refresh shortcuts");
                self.error = Some(error);
                Effect::None
            }
        };
        self.apply(effect)
    }

    fn view(&self) -> Element<'_, Message> {
        if self.preview {
            return self.cheatsheet();
        }
        let button = self
            .core
            .applet
            .icon_button_from_handle(PANEL_ICON.clone())
            .selected(self.popup.id().is_some())
            .on_press_down(Message::Click);
        self.core.applet.autosize_window(button).into()
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if self.popup.id() != Some(id) {
            return widget::space().into();
        }
        self.core
            .applet
            .popup_container(self.cheatsheet())
            .limits(popup_limits())
            .into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::Closed(id))
    }

    fn style(&self) -> Option<iced::theme::Style> {
        (!self.preview).then(cosmic::applet::style)
    }
}

fn popup_limits() -> Limits {
    Limits::NONE
        .min_width(WIDTH)
        .max_width(WIDTH)
        .min_height(1.0)
        .max_height(MAX_HEIGHT)
}

fn px(value: f32) -> f32 {
    value * POPUP_SCALE
}

fn scaled_u16(value: u16) -> u16 {
    px(f32::from(value)).round() as u16
}

fn popup_body(label: &str) -> widget::Text<'_, cosmic::Theme, cosmic::Renderer> {
    text::body(label)
        .size(px(14.0))
        .line_height(iced::widget::text::LineHeight::Absolute(px(21.0).into()))
}

fn popup_caption(label: &str) -> widget::Text<'_, cosmic::Theme, cosmic::Renderer> {
    text::caption(label)
        .size(px(12.0))
        .line_height(iced::widget::text::LineHeight::Absolute(px(17.0).into()))
}

fn shortcut_row(row: &ShortcutRow) -> Element<'_, Message> {
    let mut chords = widget::column::with_capacity(row.chords.len().max(1))
        .align_x(Alignment::End)
        .spacing(px(6.0))
        .width(Length::Fill);
    if row.chords.is_empty() {
        chords =
            chords.push(popup_caption("Not set").class(cosmic::theme::Text::Custom(muted_text)));
    } else {
        for chord in &row.chords {
            chords = chords.push(shortcut_keys(chord));
        }
    }
    container(
        widget::row::with_children(vec![
            popup_body(row.label.as_str()).width(px(135.0)).into(),
            chords.into(),
        ])
        .spacing(px(12.0))
        .align_y(Alignment::Center),
    )
    .padding([px(1.0), 0.0])
    .into()
}

fn section<'a>(group: &'a Group, icon_name: &'static str) -> Element<'a, Message> {
    let mut rows = widget::column::with_capacity(group.rows.len()).spacing(px(4.0));
    for row in &group.rows {
        rows = rows.push(shortcut_row(row));
    }
    container(
        widget::column::with_children(vec![section_header(group.title, icon_name), rows.into()])
            .spacing(px(8.0)),
    )
    .padding(px(12.0))
    .width(Length::Fill)
    .style(section_style)
    .into()
}

fn section_header(title: &'static str, icon_name: &'static str) -> Element<'static, Message> {
    let badge = container(
        icon::from_name(icon_name)
            .symbolic(true)
            .icon()
            .size(scaled_u16(16))
            .class(cosmic::theme::Svg::custom(|theme| widget::svg::Style {
                color: Some(theme.cosmic().accent_text_color().into()),
            })),
    )
    .padding(px(4.0))
    .style(|theme| {
        let cosmic = theme.cosmic();
        container::Style {
            background: Some(Color::from(cosmic.accent_color()).scale_alpha(0.12).into()),
            border: Border {
                radius: cosmic.corner_radii.radius_s.map(px).into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });
    widget::row::with_children(vec![
        badge.into(),
        text::caption_heading(title)
            .size(px(13.0))
            .line_height(iced::widget::text::LineHeight::Absolute(px(17.0).into()))
            .class(cosmic::theme::Text::Accent)
            .width(Length::Fill)
            .into(),
    ])
    .spacing(px(10.0))
    .align_y(Alignment::Center)
    .into()
}

fn shortcut_keys(chord: &str) -> Element<'_, Message> {
    // Preserve the explicit alternative for modifier-only bindings. Otherwise,
    // separate the modifier/key components while keeping alternative keys together.
    let tokens: Vec<_> = if chord.contains("  or  ") {
        vec![chord]
    } else {
        chord.split(" + ").collect()
    };
    let mut keys = Vec::with_capacity(tokens.len() * 2);
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            keys.push(
                popup_caption("+")
                    .class(cosmic::theme::Text::Custom(muted_text))
                    .into(),
            );
        }
        let primary = index == tokens.len() - 1;
        keys.push(
            container(popup_caption(token).font(cosmic::font::semibold()))
                .padding([px(3.0), px(8.0)])
                .style(move |theme| keycap_style(theme, primary))
                .into(),
        );
    }
    widget::flex_row(keys)
        .width(Length::Fill)
        .column_spacing(scaled_u16(4))
        .row_spacing(scaled_u16(4))
        .align_items(Alignment::Center)
        .justify_content(widget::JustifyContent::End)
        .into()
}

fn section_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    let background = cosmic.background(theme.transparent);
    container::Style {
        background: Some(
            Color::from(background.component.base)
                .scale_alpha(0.40)
                .into(),
        ),
        border: Border {
            color: Color::from(background.divider).scale_alpha(0.55),
            width: px(1.0),
            radius: cosmic.corner_radii.radius_m.map(px).into(),
        },
        ..Default::default()
    }
}

fn keycap_style(theme: &cosmic::Theme, primary: bool) -> container::Style {
    let cosmic = theme.cosmic();
    let background = cosmic.background(theme.transparent);
    container::Style {
        text_color: Some(if primary {
            cosmic.accent_text_color().into()
        } else {
            background.on.into()
        }),
        background: Some(if primary {
            Color::from(cosmic.accent_color()).scale_alpha(0.10).into()
        } else {
            Color::from(background.component.base)
                .scale_alpha(0.65)
                .into()
        }),
        border: Border {
            color: if primary {
                Color::from(cosmic.accent_color()).scale_alpha(0.24)
            } else {
                Color::from(background.divider).scale_alpha(0.65)
            },
            width: px(1.0),
            radius: cosmic.corner_radii.radius_s.map(px).into(),
        },
        ..Default::default()
    }
}

fn muted_color(theme: &cosmic::Theme) -> Color {
    Color::from(theme.cosmic().background(theme.transparent).on).scale_alpha(0.65)
}

fn muted_text(theme: &cosmic::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(muted_color(theme)),
        ..Default::default()
    }
}

fn input_event(event: iced::Event, _: iced::event::Status, id: window::Id) -> Option<Message> {
    match event {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            ..
        }) => Some(Message::Dismiss(id)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_motion_and_crossings_do_not_trigger_popup_actions() {
        for status in [iced::event::Status::Ignored, iced::event::Status::Captured] {
            for event in [
                iced::mouse::Event::CursorEntered,
                iced::mouse::Event::CursorMoved {
                    position: iced::Point::new(10.0, 10.0),
                },
                iced::mouse::Event::CursorLeft,
            ] {
                assert!(
                    input_event(iced::Event::Mouse(event), status, window::Id::unique()).is_none()
                );
            }
        }
    }
}

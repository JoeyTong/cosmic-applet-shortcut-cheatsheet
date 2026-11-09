//! Click-toggle lifecycle with surface identities to ignore stale close events.

use cosmic::iced::window;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    None,
    Open(window::Id),
    Close(window::Id),
}

#[derive(Debug, Default)]
pub struct PopupState {
    id: Option<window::Id>,
}

impl PopupState {
    pub fn id(&self) -> Option<window::Id> {
        self.id
    }

    pub fn toggle(&mut self) -> Effect {
        if let Some(id) = self.id.take() {
            Effect::Close(id)
        } else {
            let id = window::Id::unique();
            self.id = Some(id);
            Effect::Open(id)
        }
    }

    pub fn dismiss(&mut self, id: window::Id) -> Effect {
        if self.id == Some(id) {
            self.id = None;
            Effect::Close(id)
        } else {
            Effect::None
        }
    }

    pub fn closed(&mut self, id: window::Id) {
        if self.id == Some(id) {
            self.id = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(state: &mut PopupState) -> window::Id {
        match state.toggle() {
            Effect::Open(id) => id,
            other => panic!("expected a new popup, got {other:?}"),
        }
    }

    #[test]
    fn click_toggles_the_popup_and_reopening_uses_a_new_surface() {
        let mut state = PopupState::default();
        assert_eq!(state.id(), None);
        let first = open(&mut state);
        assert_eq!(state.id(), Some(first));
        assert_eq!(state.toggle(), Effect::Close(first));
        assert_eq!(state.id(), None);
        let second = open(&mut state);
        assert_ne!(first, second);
        assert_eq!(state.id(), Some(second));
    }

    #[test]
    fn stale_surface_events_cannot_close_a_reopened_popup() {
        let mut state = PopupState::default();
        let first = open(&mut state);
        assert_eq!(state.toggle(), Effect::Close(first));
        let second = open(&mut state);
        state.closed(first);
        assert_eq!(state.dismiss(first), Effect::None);
        assert_eq!(state.id(), Some(second));
    }

    #[test]
    fn external_close_allows_the_next_click_to_reopen() {
        let mut state = PopupState::default();
        let first = open(&mut state);
        state.closed(first);
        assert_eq!(state.id(), None);
        assert_ne!(open(&mut state), first);
    }

    #[test]
    fn keyboard_dismissal_closes_the_current_surface_only_once() {
        let mut state = PopupState::default();
        let id = open(&mut state);
        assert_eq!(state.dismiss(id), Effect::Close(id));
        assert_eq!(state.dismiss(id), Effect::None);
        state.closed(id);
        assert_eq!(state.id(), None);
    }
}

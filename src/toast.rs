use crate::{Launcher, ToastItem};
use slint::{ComponentHandle, VecModel, Weak};
use std::{cell::RefCell, collections::VecDeque, rc::Rc, time::Instant};

const VISIBLE_LIMIT: usize = 3;
const QUEUE_LIMIT: usize = 8;
const HIDDEN_LIMIT: usize = 5;
const DEDUPE_TIME: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Level {
    Info,
    Success,
    Warning,
    Error,
}

fn duration_for_level(level: Level) -> std::time::Duration {
    match level {
        Level::Error => std::time::Duration::from_millis(4500),
        Level::Warning => std::time::Duration::from_millis(3500),
        Level::Info | Level::Success => std::time::Duration::from_millis(2800),
    }
}

#[derive(Clone, Debug)]
struct Message {
    id: i32,
    level: Level,
    text: String,
    created: Instant,
    pulse: i32,
}

struct State {
    visible: Vec<Message>,
    queued: VecDeque<Message>,
    held: VecDeque<Message>,
    next_id: i32,
    hidden: bool,
    hover_count: usize,
    hovered: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State { hidden: true, ..State::default() });
    static TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
    static UNHOVER_TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
}

impl Default for State {
    fn default() -> Self {
        Self {
            visible: Vec::new(),
            queued: VecDeque::new(),
            held: VecDeque::new(),
            next_id: 0,
            hidden: true,
            hover_count: 0,
            hovered: false,
        }
    }
}

impl State {
    fn add(&mut self, level: Level, text: String, now: Instant) {
        if self.hidden {
            if matches!(level, Level::Warning | Level::Error) {
                let message = self.message(level, text, now);
                self.held.push_back(message);
                while self.held.len() > HIDDEN_LIMIT {
                    self.held.pop_front();
                }
            }
            return;
        }

        if let Some(message) = self
            .visible
            .iter_mut()
            .find(|message| message.level == level && message.text == text)
            && now.duration_since(message.created) <= DEDUPE_TIME
        {
            message.pulse += 1;
            message.created = now;
            return;
        }
        if self
            .queued
            .iter()
            .any(|message| message.level == level && message.text == text)
        {
            return;
        }

        let message = self.message(level, text, now);
        if self.visible.len() < VISIBLE_LIMIT {
            self.visible.push(message);
        } else if matches!(level, Level::Warning | Level::Error)
            && let Some(index) = self
                .visible
                .iter()
                .position(|message| matches!(message.level, Level::Info | Level::Success))
        {
            self.visible.remove(index);
            self.visible.push(message);
        } else {
            self.queue(message);
        }
    }

    fn message(&mut self, level: Level, text: String, created: Instant) -> Message {
        self.next_id = self.next_id.wrapping_add(1).max(1);
        Message {
            id: self.next_id,
            level,
            text,
            created,
            pulse: 0,
        }
    }

    fn queue(&mut self, message: Message) {
        if self.queued.len() == QUEUE_LIMIT {
            let discard = self
                .queued
                .iter()
                .position(|item| matches!(item.level, Level::Info | Level::Success))
                .unwrap_or(0);
            self.queued.remove(discard);
        }
        self.queued.push_back(message);
    }

    fn dismiss(&mut self, id: i32) {
        if let Some(index) = self.visible.iter().position(|message| message.id == id) {
            self.visible.remove(index);
        }
        self.hover_count = 0;
        if !self.hidden
            && let Some(message) = self.queued.pop_front()
        {
            self.visible.push(message);
        }
        if let Some(front) = self.visible.last_mut() {
            front.created = Instant::now();
        }
    }

    fn set_hovered(&mut self, hovered: bool) {
        if self.hovered == hovered {
            return;
        }
        self.hovered = hovered;
        if !hovered && let Some(front) = self.visible.last_mut() {
            front.created = Instant::now();
        }
    }

    fn set_hidden(&mut self, hidden: bool) {
        if self.hidden == hidden {
            return;
        }
        self.hidden = hidden;
        if hidden {
            self.visible.clear();
            self.queued.clear();
            self.hover_count = 0;
            self.hovered = false;
        } else {
            let now = Instant::now();
            while let Some(message) = self.held.pop_front() {
                self.add(message.level, message.text, now);
            }
        }
    }
}

fn level_value(level: Level) -> i32 {
    match level {
        Level::Info => 0,
        Level::Success => 1,
        Level::Warning => 2,
        Level::Error => 3,
    }
}

fn publish(ui: &Launcher) {
    let messages = STATE.with(|state| state.borrow().visible.clone());
    let model = messages
        .into_iter()
        .map(|message| ToastItem {
            id: message.id,
            level: level_value(message.level),
            message: message.text.into(),
            pulse: message.pulse,
        })
        .collect::<Vec<_>>();
    ui.set_toast_items(Rc::new(VecModel::from(model)).into());
}

fn sync_timer(weak: &Weak<Launcher>) {
    let front = STATE.with(|state| {
        let state = state.borrow();
        if state.hidden || state.hovered {
            None
        } else {
            state
                .visible
                .last()
                .map(|m| (m.id, duration_for_level(m.level)))
        }
    });

    TIMER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some((id, duration)) = front {
            let weak = weak.clone();
            let timer = slot.get_or_insert_with(slint::Timer::default);
            timer.start(slint::TimerMode::SingleShot, duration, move || {
                STATE.with(|state| state.borrow_mut().dismiss(id));
                if let Some(ui) = weak.upgrade() {
                    publish(&ui);
                    sync_timer(&ui.as_weak());
                }
            });
        } else if let Some(timer) = slot.as_ref() {
            timer.stop();
        }
    });
}

fn request(weak: &Weak<Launcher>, level: Level, text: impl Into<String>) {
    let text = text.into();
    eprintln!("orbit notification: {text}");
    let _ = weak.upgrade_in_event_loop(move |ui| {
        STATE.with(|state| state.borrow_mut().add(level, text, Instant::now()));
        publish(&ui);
        sync_timer(&ui.as_weak());
    });
}

pub fn info(weak: &Weak<Launcher>, text: impl Into<String>) {
    request(weak, Level::Info, text);
}

pub fn success(weak: &Weak<Launcher>, text: impl Into<String>) {
    request(weak, Level::Success, text);
}

pub fn warning(weak: &Weak<Launcher>, text: impl Into<String>) {
    request(weak, Level::Warning, text);
}

pub fn error(weak: &Weak<Launcher>, text: impl Into<String>) {
    request(weak, Level::Error, text);
}

pub fn init(ui: &Launcher) {
    let weak = ui.as_weak();
    ui.on_dismiss_toast(move |id| {
        STATE.with(|state| state.borrow_mut().dismiss(id));
        if let Some(ui) = weak.upgrade() {
            publish(&ui);
            sync_timer(&ui.as_weak());
        }
    });
    let weak = ui.as_weak();
    ui.on_toast_card_hovered(move |hovered| {
        let (became_hovered, became_unhovered) = STATE.with(|state| {
            let mut s = state.borrow_mut();
            if hovered {
                s.hover_count += 1;
                let changed = !s.hovered;
                s.hovered = true;
                (changed, false)
            } else {
                s.hover_count = s.hover_count.saturating_sub(1);
                (false, s.hover_count == 0 && s.hovered)
            }
        });

        if became_hovered {
            UNHOVER_TIMER.with(|cell| {
                if let Some(timer) = cell.borrow_mut().as_ref() {
                    timer.stop();
                }
            });
            if let Some(ui) = weak.upgrade() {
                ui.set_toasts_hovered(true);
                sync_timer(&ui.as_weak());
            }
        } else if became_unhovered {
            let weak = weak.clone();
            UNHOVER_TIMER.with(|cell| {
                let mut slot = cell.borrow_mut();
                let timer = slot.get_or_insert_with(slint::Timer::default);
                timer.start(
                    slint::TimerMode::SingleShot,
                    std::time::Duration::from_millis(150),
                    move || {
                        let should_unhover = STATE.with(|state| {
                            let mut s = state.borrow_mut();
                            if s.hover_count == 0 {
                                s.set_hovered(false);
                                true
                            } else {
                                false
                            }
                        });
                        if should_unhover && let Some(ui) = weak.upgrade() {
                            ui.set_toasts_hovered(false);
                            sync_timer(&ui.as_weak());
                        }
                    },
                );
            });
        }
    });
}

pub fn on_window_hidden(ui: &Launcher) {
    UNHOVER_TIMER.with(|cell| {
        if let Some(timer) = cell.borrow_mut().as_ref() {
            timer.stop();
        }
    });
    ui.set_toasts_hovered(false);
    STATE.with(|state| state.borrow_mut().set_hidden(true));
    publish(ui);
    sync_timer(&ui.as_weak());
}

pub fn on_window_shown(ui: &Launcher) {
    STATE.with(|state| state.borrow_mut().set_hidden(false));
    publish(ui);
    sync_timer(&ui.as_weak());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        State {
            hidden: false,
            ..State::default()
        }
    }

    #[test]
    fn shows_three_then_promotes_fifo() {
        let now = Instant::now();
        let mut state = state();
        for text in ["one", "two", "three", "four"] {
            state.add(Level::Info, text.into(), now);
        }
        assert_eq!(state.visible.len(), 3);
        assert_eq!(state.queued.front().unwrap().text, "four");
        let first = state.visible[0].id;
        state.dismiss(first);
        assert_eq!(state.visible.last().unwrap().text, "four");
    }

    #[test]
    fn urgent_message_replaces_an_old_success_and_duplicates_are_coalesced() {
        let now = Instant::now();
        let mut state = state();
        for (level, text) in [
            (Level::Info, "loading"),
            (Level::Warning, "careful"),
            (Level::Error, "failed"),
        ] {
            state.add(level, text.into(), now);
        }
        state.add(Level::Error, "urgent".into(), now);
        state.add(Level::Error, "failed".into(), now);
        assert_eq!(state.visible.len(), 3);
        assert_eq!(state.visible[0].level, Level::Warning);
        assert_eq!(state.visible[1].level, Level::Error);
        assert_eq!(state.visible[1].text, "failed");
        assert_eq!(state.visible[1].pulse, 1);
        assert_eq!(state.visible[2].text, "urgent");
        assert!(state.queued.is_empty());
    }

    #[test]
    fn hidden_window_holds_only_urgent_feedback() {
        let mut state = State::default();
        let now = Instant::now();
        state.add(Level::Success, "saved".into(), now);
        state.add(Level::Warning, "check this".into(), now);
        state.set_hidden(false);
        assert_eq!(state.visible.len(), 1);
        assert_eq!(state.visible[0].text, "check this");
    }

    #[test]
    fn auto_dismiss_promotes_stacked_cards_sequentially() {
        let now = Instant::now();
        let mut state = state();
        state.add(Level::Info, "first".into(), now);
        state.add(Level::Error, "second".into(), now);
        assert_eq!(state.visible.len(), 2);
        assert_eq!(state.visible.last().unwrap().text, "second");

        // Simulating the front card expiring:
        let front_id = state.visible.last().unwrap().id;
        state.dismiss(front_id);
        assert_eq!(state.visible.len(), 1);
        assert_eq!(state.visible.last().unwrap().text, "first");

        let next_id = state.visible.last().unwrap().id;
        state.dismiss(next_id);
        assert!(state.visible.is_empty());
    }

    #[test]
    fn hover_pauses_auto_dismiss_timer_and_resumes_on_unhover() {
        let now = Instant::now();
        let mut state = state();
        state.add(Level::Info, "test message".into(), now);
        assert_eq!(state.visible.len(), 1);

        state.set_hovered(true);
        assert!(state.hovered);

        state.set_hovered(false);
        assert!(!state.hovered);
        assert!(state.visible[0].created >= now);
    }
}

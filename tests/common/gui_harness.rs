//! Headless GUI Test Harness for Mossambi E2E Test Suite.
//!
//! Provides a headless egui Context runner, simulated input event dispatchers,
//! frame runners, and screen clamping geometry evaluators.

#![allow(clippy::field_reassign_with_default)]

use eframe::egui::{
    self, Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2,
};

/// Headless test driver wrapping an `egui::Context`.
#[derive(Clone)]
pub struct HeadlessTestApp {
    pub ctx: Context,
    pub raw_input: RawInput,
}

impl Default for HeadlessTestApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadlessTestApp {
    /// Creates a new headless test harness with default settings and a standard 1280x800 viewport.
    pub fn new() -> Self {
        let ctx = Context::default();
        let mut raw_input = RawInput::default();
        raw_input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
        Self { ctx, raw_input }
    }

    /// Returns a reference to the underlying egui Context.
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// Dispatches a simulated resize event by updating `screen_rect`.
    pub fn simulate_resize(&mut self, size: Vec2) {
        self.raw_input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, size));
    }

    /// Dispatches simulated pointer move and click events at `pos`.
    pub fn simulate_click(&mut self, pos: Pos2) {
        self.raw_input.events.push(Event::PointerMoved(pos));
        self.raw_input.events.push(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::default(),
        });
        self.raw_input.events.push(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::default(),
        });
    }

    /// Dispatches simulated secondary (right-click) pointer events at `pos`.
    pub fn simulate_secondary_click(&mut self, pos: Pos2) {
        self.raw_input.events.push(Event::PointerMoved(pos));
        self.raw_input.events.push(Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed: true,
            modifiers: Modifiers::default(),
        });
        self.raw_input.events.push(Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed: false,
            modifiers: Modifiers::default(),
        });
    }

    /// Dispatches a simulated keypress event with modifiers.
    pub fn simulate_key(&mut self, key: Key, modifiers: Modifiers) {
        self.raw_input.events.push(Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        });
        self.raw_input.events.push(Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        });
    }

    /// Executes an immediate-mode UI frame callback headlessly and returns the full output.
    pub fn run_frame<F: FnMut(&Context)>(&mut self, mut f: F) -> egui::FullOutput {
        let input = std::mem::take(&mut self.raw_input);
        // Preserve screen_rect for next frame
        self.raw_input.screen_rect = input.screen_rect;
        self.ctx.run(input, |ctx| {
            f(ctx);
        })
    }
}

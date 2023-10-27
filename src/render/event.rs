use std::{cell::RefCell, rc::Rc, collections::{VecDeque, HashMap}};

use wasm_bindgen::{JsCast, prelude::Closure, JsValue};
use web_sys::{HtmlCanvasElement, EventTarget, KeyboardEvent, MouseEvent};

use crate::{console_log, util::get_expected_size};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum KeyboardKey {
    Character(char),
    Alt, 
    AltGr,
    CapsLock,
    Control,
    Fn,
    FnLock,
    Hyper,
    Meta,
    NumLock,
    ScrollLock,
    Shift,
    Super,
    Symbol,
    SymbolLock,
    Dead,

    Unidentified
}

impl KeyboardKey {
    pub fn extract(key: &str) -> Self {
        match key {
            "Alt" => KeyboardKey::Alt,
            "AltGraph" => KeyboardKey::AltGr,
            "CapsLock" => KeyboardKey::CapsLock,
            "Control" => KeyboardKey::Control,
            "Fn" => KeyboardKey::Fn,
            "FnLock" => KeyboardKey::FnLock,
            "Hyper" => KeyboardKey::Hyper,
            "Meta" => KeyboardKey::Meta,
            "NumLock" => KeyboardKey::Meta,
            "ScrollLock" => KeyboardKey::ScrollLock,
            "Shift" => KeyboardKey::Shift,
            "Super" => KeyboardKey::Super,
            "Symbol" => KeyboardKey::Symbol,
            "SymbolLock" => KeyboardKey::Symbol,
            "Dead" => KeyboardKey::Dead,

            s if s.len() == 1 => KeyboardKey::Character(s.chars().next().unwrap()),

            _ => KeyboardKey::Unidentified
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyboardEventData {
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,

    pub key: KeyboardKey
}

impl KeyboardEventData {
    pub fn extract(event: &KeyboardEvent) -> Self {
        KeyboardEventData { 
            alt_key: event.alt_key(), 
            ctrl_key: event.ctrl_key(), 
            shift_key: event.shift_key(), 
            meta_key: event.meta_key(), 
            key: KeyboardKey::extract(&event.key())
        }
    }
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    Left, // Main button
    Middle, // Auxiliary button
    Right, // Secondary button

    OtherButton(u8)
}

impl MouseButton {
    pub fn extract(button: u8) -> Self {
        match button {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            _ => MouseButton::OtherButton(button)
        }
    }
}

#[derive(Debug, Clone)]
pub struct MouseEventData {
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub shift_key: bool,
    pub meta_key: bool,

    pub button: MouseButton,

    pub movement_x: i32,
    pub movement_y: i32,

    pub x: i32,
    pub y: i32
}

impl MouseEventData {
    pub fn extract(event: &MouseEvent) -> Self {
        MouseEventData {
            alt_key: event.alt_key(),
            ctrl_key: event.ctrl_key(),
            shift_key: event.shift_key(),
            meta_key: event.meta_key(),

            button: MouseButton::extract(event.button() as u8),

            movement_x: event.movement_x(),
            movement_y: event.movement_y(),

            x: event.x(),
            y: event.y()
        }
    }
}

#[derive(Debug)]
pub struct CanvasResizeData {
    pub old_width: u32,
    pub old_height: u32,

    pub new_width: u32,
    pub new_height: u32
}

#[derive(Debug)]
pub enum Event {
    KeyDown(KeyboardEventData),
    KeyUp(KeyboardEventData),

    MouseDown(MouseEventData),
    MouseUp(MouseEventData),
    MouseMove(MouseEventData),

    CanvasResize(CanvasResizeData)
}

pub struct EventQueue {
    pub events: VecDeque<Event>,
    canvas: HtmlCanvasElement
}

impl EventQueue {
    pub fn for_canvas(canvas: HtmlCanvasElement) -> Result<Rc<RefCell<EventQueue>>, JsValue> {
        let event_target: EventTarget = canvas.clone().into();
        let document: EventTarget = canvas.owner_document().unwrap().into();

        let queue = Rc::new(RefCell::new(EventQueue {
            events: VecDeque::new(),
            canvas
        }));

        let queue_clone = queue.clone();
        let keydown_handler = move |event: web_sys::Event| {
            let key_data = KeyboardEventData::extract(&event.unchecked_into());
            queue_clone.borrow_mut().enqueue(Event::KeyDown(key_data));
        };

        let queue_clone = queue.clone();
        let keyup_handler = move |event: web_sys::Event| {
            let key_data = KeyboardEventData::extract(&event.unchecked_into());
            queue_clone.borrow_mut().enqueue(Event::KeyUp(key_data));
        };

        let queue_clone = queue.clone();
        let mousedown_handler = move |event: web_sys::Event| {
            let mouse_data = MouseEventData::extract(&event.unchecked_into());
            queue_clone.borrow_mut().enqueue(Event::MouseDown(mouse_data));
        };

        let queue_clone = queue.clone();
        let mouseup_handler = move |event: web_sys::Event| {
            let mouse_data = MouseEventData::extract(&event.unchecked_into());
            queue_clone.borrow_mut().enqueue(Event::MouseUp(mouse_data));
        };

        let queue_clone = queue.clone();
        let mousemove_handler = move |event: web_sys::Event| {
            let mouse_data = MouseEventData::extract(&event.unchecked_into());
            queue_clone.borrow_mut().enqueue(Event::MouseMove(mouse_data));
        };

        
        let keydown_handler: Closure<dyn FnMut(_)> = Closure::new(keydown_handler);
        let keyup_handler: Closure<dyn FnMut(_)> = Closure::new(keyup_handler);
        let mousedown_handler: Closure<dyn FnMut(_)> = Closure::new(mousedown_handler);
        let mouseup_handler: Closure<dyn FnMut(_)> = Closure::new(mouseup_handler);
        let mousemove_handler: Closure<dyn FnMut(_)> = Closure::new(mousemove_handler);

        document.add_event_listener_with_callback("keydown", &keydown_handler.as_ref().unchecked_ref())?;
        document.add_event_listener_with_callback("keyup", &keyup_handler.as_ref().unchecked_ref())?;
        event_target.add_event_listener_with_callback("mousedown", &mousedown_handler.as_ref().unchecked_ref())?;
        event_target.add_event_listener_with_callback("mouseup", &mouseup_handler.as_ref().unchecked_ref())?;
        event_target.add_event_listener_with_callback("mousemove", &mousemove_handler.as_ref().unchecked_ref())?;

        Box::leak(Box::new(keydown_handler));
        Box::leak(Box::new(keyup_handler));
        Box::leak(Box::new(mousedown_handler));
        Box::leak(Box::new(mouseup_handler));
        Box::leak(Box::new(mousemove_handler));

        Ok(queue)
    }

    pub fn detect_resize(&mut self) {
        let (new_width, new_height) = get_expected_size(&self.canvas);

        if new_width != self.canvas.width() || new_height!= self.canvas.height()  {
            self.canvas.set_width(new_width);
            self.canvas.set_height(new_height);

            self.enqueue_inner(Event::CanvasResize(CanvasResizeData { 
                old_width: self.canvas.width(),
                old_height: self.canvas.height(),

                new_width,
                new_height
            }));
        }
    }

    pub fn enqueue(&mut self, event: Event) {
        self.detect_resize();
        self.enqueue_inner(event);
    }

    fn enqueue_inner(&mut self, event: Event) {
        self.events.push_back(event);
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.events.pop_front()
    }

    pub fn empty(&mut self) -> bool {
        self.events.is_empty()
    }
}

pub struct KeyTracker {
    keys: HashMap<KeyboardKey, bool>
}

impl KeyTracker {
    pub fn new() -> Self {
        KeyTracker {
            keys: HashMap::new()
        }
    }

    pub fn set_key_down(&mut self, key: KeyboardKey) {
        self.keys.insert(key, true);
    }

    pub fn set_key_up(&mut self, key: KeyboardKey) {
        console_log!("Key up {:?}", key);
        self.keys.insert(key, false);
    }

    pub fn is_key_down(&self, key: KeyboardKey) -> bool {
        *self.keys.get(&key).unwrap_or(&false)
    }
}
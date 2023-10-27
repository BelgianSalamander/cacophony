use std::{time::Duration, rc::Rc, cell::RefCell};

use wasm_bindgen::prelude::{Closure, wasm_bindgen};
use web_sys::HtmlCanvasElement;
use winit::dpi::PhysicalSize;

use crate::{console_log, util::Interval};

use super::{wgpu_context::WgpuContext, event::{EventQueue, Event, CanvasResizeData, MouseEventData, KeyTracker, KeyboardEventData, KeyboardKey}, camera::Camera};

#[wasm_bindgen]
extern "C" {
    fn requestAnimationFrame(callback: &Closure<dyn FnMut(f64)>) -> u32;
}

pub struct Runtime {
    context: WgpuContext,
    canvas: HtmlCanvasElement,
    event_queue: Rc<RefCell<EventQueue>>,

    self_ref: Option<Rc<RefCell<Runtime>>>,
    render_closure: Option<Closure<dyn FnMut(f64)>>,

    frames: u128,
    last_frame: f64,

    camera: Camera,
    keyboard: KeyTracker,
}

impl Runtime {
    pub fn new(context: WgpuContext, canvas: HtmlCanvasElement, camera: Camera) -> Rc<RefCell<Self>> {
        let (width, height) = (canvas.width(), canvas.height());

        let base = Rc::new(RefCell::new(Runtime {
            context,
            canvas: canvas.clone(),
            event_queue: EventQueue::for_canvas(canvas).unwrap(),

            self_ref: None,
            render_closure: None,

            frames: 0,
            last_frame: 0.0,

            camera,
            keyboard: KeyTracker::new()
        }));
        let base_clone = base.clone();

        base.borrow_mut().self_ref = Some(base.clone());
        base.borrow_mut().render_closure = Some(Closure::wrap(Box::new(move |time| {
            base_clone.borrow_mut().render(time);
        })));

        base
    }

    pub fn request_animation_frame(&mut self) {
        if let Some(closure) = &mut self.render_closure {
            requestAnimationFrame(closure);
        }
    }

    pub fn render(&mut self, time: f64) {
        let dt = (time - self.last_frame) / 1000.0;
        self.last_frame = time;

        self.event_queue.borrow_mut().detect_resize();
        while let Some(event) = { let x = self.event_queue.borrow_mut().pop(); x } {
            self.handle_event(event);
        }

        let mut forward = 0.0;
        let mut right = 0.0;
        let mut up = 0.0;

        if self.keyboard.is_key_down(KeyboardKey::Character('w')) {
            forward += 1.0;
        } 
        if self.keyboard.is_key_down(KeyboardKey::Character('s')) {
            forward -= 1.0;
        } 
        if self.keyboard.is_key_down(KeyboardKey::Character('a')) {
            right -= 1.0;
        } 
        if self.keyboard.is_key_down(KeyboardKey::Character('d')) {
            right += 1.0;
        }
        if self.keyboard.is_key_down(KeyboardKey::Shift) {
            up -= 1.0;
        }
        if self.keyboard.is_key_down(KeyboardKey::Character(' ')) {
            up += 1.0;
        }

        const SPEED: f32 = 0.5;

        self.camera.do_move(SPEED * forward * dt as f32, SPEED * right * dt as f32, SPEED * up * dt as f32);

        self.context.render(dt, &self.camera).unwrap();

        self.frames += 1;

        self.request_animation_frame();
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::CanvasResize(CanvasResizeData {new_width, new_height, ..}) => {
                self.context.resize(PhysicalSize::new(new_width, new_height));
                self.camera.aspect = new_width as f32 / new_height as f32;
            },

            Event::MouseMove(MouseEventData {movement_x, movement_y,..}) => {
                self.camera.yaw += movement_x as f32 * 0.002;
                self.camera.pitch -= movement_y as f32 * 0.002;

                if self.camera.pitch > 3.14 / 2.0 {
                    self.camera.pitch = 3.14 / 2.0;
                } else if self.camera.pitch < -3.14 / 2.0 {
                    self.camera.pitch = -3.14 / 2.0;
                }
                
                //console_log!("Camera move: {},{}", self.camera.yaw, self.camera.pitch);
            },

            Event::KeyDown(KeyboardEventData {key,..}) => self.keyboard.set_key_down(key),
            Event::KeyUp(KeyboardEventData {key,..}) => self.keyboard.set_key_up(key),

            _ => {}
        }
    }

    pub fn cleanup(&mut self) {
        self.self_ref = None;
        self.render_closure = None;
    }
}
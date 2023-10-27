use wasm_bindgen::prelude::{wasm_bindgen, Closure};
use web_sys::HtmlCanvasElement;

#[wasm_bindgen]
extern "C" {
    fn setInterval(closure: &Closure<dyn FnMut()>, millis: u32) -> f64;
    fn clearInterval(token: f64);

    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => ($crate::util::log(&format_args!($($t)*).to_string()))
}

pub struct Interval {
    closure: Closure<dyn FnMut()>,
    token: f64,
}

impl Interval {
    pub fn new<F: 'static>(f: F, millis: u32) -> Interval
    where
        F: FnMut()
    {
        // Construct a new closure.
        let closure = Closure::new(f);

        // Pass the closure to JS, to run every n milliseconds.
        let token = setInterval(&closure, millis);

        Interval { closure, token }
    }

    pub fn leak(self) {
        Box::leak(Box::new(self));
    }
}

// When the Interval is destroyed, clear its `setInterval` timer.
impl Drop for Interval {
    fn drop(&mut self) {
        clearInterval(self.token);
    }
}

pub fn get_expected_size(canvas: &HtmlCanvasElement) -> (u32, u32) {
    let width = canvas.client_width();
    let height = canvas.client_height();

    let width = width.max(150);
    let height = height.max(150);

    (width as u32, height as u32)
}
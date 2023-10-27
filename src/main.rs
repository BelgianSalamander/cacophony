use std::{future::Future, task::{Context, Poll}, pin::Pin};

use noise::source::TestSource;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};

use crate::{render::{wgpu_context::WgpuContext, runtime::Runtime, event::EventQueue, camera::Camera}, noise::source::NoiseSource};

pub mod util;
pub mod render;
pub mod noise;

async fn run_main() -> Result<JsValue, JsValue> {
    let dom_window = web_sys::window().expect("no global `window` exists");
    let document = dom_window.document().expect("should have a document on a window");
    let body = document.body().expect("document should have a body");

    let canvas: HtmlCanvasElement = document.get_element_by_id("wgpu-canvas").expect("Cannot find canvas!").unchecked_into();
    let (width, height) = (canvas.width(), canvas.height());
    console_log!("Got canvas!");

    let camera = Camera::new(
        cgmath::Point3 { x: 0.0, y: 1.0, z: 0.0 },
        cgmath::Vector3 { x: 0.0, y: 1.0, z: 0.0 },
        0.0,
        0.0,
        width as f32 / height as f32,
        45.0
    );

    let context = WgpuContext::new(&canvas, &camera).await;
    console_log!("Created GPU context!");

    let runtime = Runtime::new(context, canvas, camera);
    console_log!("Created runtime!");
    
    runtime.borrow_mut().request_animation_frame();

    Ok(JsValue::NULL)
}

async fn noise_test() -> Result<JsValue, JsValue> {
    let dom_window = web_sys::window().expect("no global `window` exists");
    let document = dom_window.document().expect("should have a document on a window");
    let body = document.body().expect("document should have a body");

    let canvas: HtmlCanvasElement = document.get_element_by_id("wgpu-canvas").expect("Cannot find canvas!").unchecked_into();
    console_log!("Got canvas!");

    let source = TestSource;

    let resoultion: f32 = 1.0 / 20.0;
    let size = 1000;

    canvas.set_width(size as u32);
    canvas.set_height(size as u32);
    canvas.style().set_property("width", &format!("{}px", size)).unwrap();
    canvas.style().set_property("height", &format!("{}px", size)).unwrap();

    let context: CanvasRenderingContext2d = canvas.get_context("2d").unwrap().unwrap().unchecked_into();

    for xi in 0..size {
        for yi in 0..size {
            let x = xi as f32 * resoultion;
            let y = yi as f32 * resoultion;

            let sample = source.sample(x, y, 0);
            let sample = sample * 0.5 + 0.5;

            context.set_fill_style(&JsValue::from_str(&format!("rgba({}, {}, {}, 1.0)", sample * 255.0, sample * 255.0, sample * 255.0)));
            context.fill_rect(xi as _, yi as _, 1.0, 1.0);
        }

        if (xi + 1) % 10 == 0 {
            console_log!("{:?} / {}", xi + 1, size);
        }
    }

    Ok(JsValue::NULL)
}

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Warn).expect("Couldn't intialize logger");

    wasm_bindgen_futures::future_to_promise(run_main());
    //wasm_bindgen_futures::future_to_promise(noise_test());
}
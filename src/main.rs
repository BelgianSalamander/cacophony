use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use winit::dpi::PhysicalSize;

pub mod util;

pub struct WgpuContext {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>
}

impl WgpuContext {
    pub async fn new(canvas: &HtmlCanvasElement)-> Self {
        let (width, height) = (canvas.width(), canvas.height());
        console_log!("Surface size: {} {}", width, height);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface_from_canvas(canvas.clone()).expect("Could not create surface :(");

        let adpater = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        console_log!("Adapter: {:?}", adpater.get_info());

        let (device, queue) = adpater
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    label: None
                },
                None,
            )
            .await
            .unwrap();

        let tex_format = surface.get_capabilities(&adpater).formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: tex_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
            view_formats: vec![tex_format]
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,
            size: PhysicalSize::new(width, height)
        }
    }
}

async fn run_main() {
    let dom_window = web_sys::window().expect("no global `window` exists");
    let document = dom_window.document().expect("should have a document on a window");
    let body = document.body().expect("document should have a body");

    let canvas: HtmlCanvasElement = document.get_element_by_id("wgpu-canvas").expect("Cannot find canvas!").unchecked_into();
    console_log!("Got canvas!");

    let context = WgpuContext::new(&canvas).await;
    console_log!("Created GPU context!");

    
}

fn main() {
    console_error_panic_hook::set_once();

    pollster::block_on(run_main());
}
use crate::renderer::Renderer;
use crate::{gui::Gui, App};
use wgpu::{
    Adapter, Dx12Compiler, Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, Surface,
};
use winit::{
    event::{self, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
};

#[rustfmt::skip]
#[allow(unused)]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::{mem::size_of_val, slice::from_raw_parts};

    unsafe { from_raw_parts(data.as_ptr() as *const u8, size_of_val(data)) }
}

struct Setup {
    window: winit::window::Window,
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter, // what is the difference btw Adapter and Device ?
    device: wgpu::Device,
    queue: wgpu::Queue,
}

async fn setup(title: &str) -> Setup {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    };

    let event_loop = EventLoopBuilder::with_user_event().build();
    let window = create_window(title, &event_loop);
    let instance = create_instance();

    log::info!("Initializing the surface...");
    // create the main rendering surface (on screen or window)
    let (size, surface) = unsafe {
        let size = window.inner_size();
        let surface = instance.create_surface(&window).unwrap();
        (size, surface)
    };

    let adapter = create_adapter(&instance, &surface).await;
    // check features
    let optional_features = wgpu::Features::empty();
    let required_features = wgpu::Features::empty();
    let adapter_features = adapter.features();
    assert!(
        adapter_features.contains(required_features),
        "Adapter does not support required features for this example: {:?}",
        required_features - adapter_features
    );
    println!("Features: {:?}", adapter_features);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let adapter_info = adapter.get_info();
        println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    }

    #[cfg(not(target_arch = "wasm32"))]
    let needed_limits = wgpu::Limits::default().using_resolution(adapter.limits());

    #[cfg(target_arch = "wasm32")]
    let needed_limits =
        wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());

    let trace_dir = std::env::var("WGPU_TRACE");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Framework: device descriptor"),
                features: (optional_features & adapter_features) | required_features,
                limits: needed_limits,
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        )
        .await
        .expect("Cannot request GPU device");

    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }
}

async fn create_adapter(instance: &Instance, surface: &Surface) -> Adapter {
    wgpu::util::initialize_adapter_from_env_or_default(instance, Some(surface))
        .await
        .expect("No suitable GPU adapters found on the system!")
}

fn create_window(title: &str, event_loop: &EventLoop<()>) -> winit::window::Window {
    let mut builder = winit::window::WindowBuilder::new();
    builder = builder.with_title(title);
    #[cfg(windows_OFF)] // TODO
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        let mut width = 0;
        let mut height = 0;
        use winit::platform::web::WindowExtWebSys;
        console_log::init().expect("could not initialize logger");
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // On wasm, append the canvas to the document body
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                width = body.client_width() as u32;
                height = body.client_height() as u32;
                window.set_inner_size(winit::dpi::PhysicalSize::new(width, height));
                window
                    .canvas()
                    .set_oncontextmenu(Some(&js_sys::Function::new_no_args("return false;")));
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");
    }
    window
}

fn create_instance() -> Instance {
    // rendering backend (OpenGL, Vulkan, DirectX, ...)
    let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    // wgpu instance creates adapters and surfaces
    let instance_desc = InstanceDescriptor {
        backends: backend,
        dx12_shader_compiler: Dx12Compiler::default(),
        flags: InstanceFlags::default(),
        gles_minor_version: Gles3MinorVersion::Automatic,
    };
    wgpu::Instance::new(instance_desc)
}

fn start(
    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }: Setup,
) {
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![format],
    };
    println!("Surface config: {surface_config:?}");
    surface.configure(&device, &surface_config);

    log::info!("Initializing the example...");
    let mut gui = Gui::new(&window, &event_loop);
    let renderer = Renderer::init(&surface_config, device, queue);
    let mut app = App::new(renderer);
    let context = egui::Context::default();
    context.set_pixels_per_point(window.scale_factor() as f32);

    log::info!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter); // force ownership by the closure
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        app.update();

        match event {
            event::Event::RedrawEventsCleared => {
                window.request_redraw();
            }
            event::Event::WindowEvent {
                event:
                    WindowEvent::Resized(size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut size,
                        ..
                    },
                ..
            } => {
                log::info!("Resizing to {:?}", size);
                surface_config.width = size.width.max(1);
                surface_config.height = size.height.max(1);
                surface.configure(&app.renderer.device, &surface_config);
                app.renderer.resize(&surface_config);
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {
                    // forward events to egui
                    let _ = gui.winit_state.on_window_event(&context, &event);
                    app.winit_update(&event);
                }
            },
            event::Event::RedrawRequested(_) => {
                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(&app.renderer.device, &surface_config);
                        surface
                            .get_current_texture()
                            .expect("Failed to acquire next swap chain texture!")
                    }
                };

                let output = gui.update(&context, &window, &mut app);

                app.renderer.render(
                    &frame,
                    output,
                    &mut app.compute,
                    &context,
                    window.scale_factor() as f32,
                );
                frame.present();
            }

            _ => {}
        }

        if gui.exit_app {
            *control_flow = ControlFlow::Exit;
        }
        let (instances_raw, num_instances) = app.psys.get_instances();
        app.renderer.sub_rpass_particles.update_instance_buffer(
            &app.renderer.device,
            &instances_raw,
            num_instances,
        );

        // gui.app.update();
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn wgpu_main() {
    let setup = pollster::block_on(setup("Particles"));
    start(setup);
}

#[cfg(target_arch = "wasm32")]
pub fn wgpu_main() {
    wasm_bindgen_futures::spawn_local(async move {
        let setup = setup("Particles").await;
        start(setup);
    });
}

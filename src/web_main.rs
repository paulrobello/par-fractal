//! Web/WASM entry point for Par Fractal

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use winit::event::*;
use winit::event_loop::EventLoop;
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};

use crate::app::App;

/// Hide the loading indicator and show the canvas
fn hide_loading() {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(loading) = document.get_element_by_id("loading") {
                let _ = loading.set_attribute("style", "display: none");
            }
        }
    }
}

/// Show an error message
fn show_error(message: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(loading) = document.get_element_by_id("loading") {
                let _ = loading.set_attribute("style", "display: none");
            }
            if let Some(error) = document.get_element_by_id("error") {
                let _ = error.set_attribute("style", "display: block");
                if let Some(p) = error.query_selector("p").ok().flatten() {
                    p.set_text_content(Some(message));
                }
            }
        }
    }
    log::error!("{}", message);
}

/// Main entry point for the web build
#[wasm_bindgen(start)]
pub async fn main_web() {
    // Set up better panic messages in browser console
    console_error_panic_hook::set_once();

    // Initialize logging to browser console
    console_log::init_with_level(log::Level::Info).ok();

    log::info!("Par Fractal WASM starting...");

    // Check for WebGPU support
    let window = web_sys::window().expect("no global window");
    let navigator = window.navigator();

    // Note: navigator.gpu() returns Option, not a JS property check
    // The actual WebGPU check happens during wgpu initialization

    let document = window.document().expect("no document");

    // Get the canvas element
    let canvas = match document.get_element_by_id("canvas") {
        Some(el) => match el.dyn_into::<web_sys::HtmlCanvasElement>() {
            Ok(canvas) => canvas,
            Err(_) => {
                show_error("Canvas element is not an HtmlCanvasElement");
                return;
            }
        },
        None => {
            show_error("Canvas element not found");
            return;
        }
    };

    // Set canvas size to match window with proper device pixel ratio
    // Get the actual viewport dimensions
    let width = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0);
    let height = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(600.0);

    // Get device pixel ratio for high-DPI displays
    let device_pixel_ratio = window.device_pixel_ratio();

    // Set canvas backing buffer size (physical pixels)
    let physical_width = (width * device_pixel_ratio) as u32;
    let physical_height = (height * device_pixel_ratio) as u32;
    canvas.set_width(physical_width);
    canvas.set_height(physical_height);

    // Set CSS size (logical pixels) via style
    let canvas_style = canvas.style();
    let _ = canvas_style.set_property("width", &format!("{}px", width));
    let _ = canvas_style.set_property("height", &format!("{}px", height));

    log::info!(
        "Canvas size: {}x{} (logical), {}x{} (physical), DPR: {}",
        width,
        height,
        physical_width,
        physical_height,
        device_pixel_ratio
    );

    // Create winit event loop
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            show_error(&format!("Failed to create event loop: {}", e));
            return;
        }
    };

    // Create window attached to canvas with explicit size (physical pixels)
    let window_attrs = winit::window::Window::default_attributes()
        .with_title("Par Fractal")
        .with_inner_size(winit::dpi::PhysicalSize::new(
            physical_width,
            physical_height,
        ))
        .with_canvas(Some(canvas));

    let winit_window = match event_loop.create_window(window_attrs) {
        Ok(w) => w,
        Err(e) => {
            show_error(&format!("Failed to create window: {}", e));
            return;
        }
    };

    log::info!("Window created, initializing app...");

    // Initialize app (async for wgpu)
    let app = match App::new_async(winit_window, None, None, None).await {
        Ok(app) => {
            hide_loading();
            log::info!("App initialized successfully");
            app
        }
        Err(e) => {
            show_error(&format!("Failed to initialize: {}", e));
            return;
        }
    };

    // Run event loop (web-compatible non-blocking version)
    let app = std::rc::Rc::new(std::cell::RefCell::new(app));

    // Set up resize handler for mobile orientation changes and window resizing
    {
        let window_clone = window.clone();
        let app_clone = app.clone();

        let closure = Closure::wrap(Box::new(move || {
            if let Some(canvas_element) = window_clone
                .document()
                .and_then(|d| d.get_element_by_id("canvas"))
            {
                if let Ok(canvas) = canvas_element.dyn_into::<web_sys::HtmlCanvasElement>() {
                    // Get updated dimensions
                    let width = window_clone
                        .inner_width()
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(800.0);
                    let height = window_clone
                        .inner_height()
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(600.0);

                    let device_pixel_ratio = window_clone.device_pixel_ratio();
                    let physical_width = (width * device_pixel_ratio) as u32;
                    let physical_height = (height * device_pixel_ratio) as u32;

                    // Update canvas size
                    canvas.set_width(physical_width);
                    canvas.set_height(physical_height);

                    let canvas_style = canvas.style();
                    let _ = canvas_style.set_property("width", &format!("{}px", width));
                    let _ = canvas_style.set_property("height", &format!("{}px", height));

                    // Notify app of resize
                    let mut app = app_clone.borrow_mut();
                    app.resize(winit::dpi::PhysicalSize::new(
                        physical_width,
                        physical_height,
                    ));

                    log::info!(
                        "Window resized: {}x{} (logical), {}x{} (physical)",
                        width,
                        height,
                        physical_width,
                        physical_height
                    );
                }
            }
        }) as Box<dyn FnMut()>);

        // Listen for both resize and orientationchange events
        let _ = window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
        let _ = window.add_event_listener_with_callback(
            "orientationchange",
            closure.as_ref().unchecked_ref(),
        );

        // Keep closure alive for the lifetime of the application
        closure.forget();
    }

    event_loop.spawn(move |event, target| {
        let mut app = app.borrow_mut();

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app.window().id() => {
                if !app.input(event) {
                    match event {
                        WindowEvent::CloseRequested => target.exit(),
                        WindowEvent::Resized(physical_size) => {
                            app.resize(*physical_size);
                        }
                        WindowEvent::RedrawRequested => {
                            app.update();
                            match app.render() {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => {
                                    let size = app.size();
                                    app.resize(size);
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                                Err(e) => log::error!("Render error: {:?}", e),
                            }
                        }
                        _ => {}
                    }
                }
            }
            Event::AboutToWait => {
                app.window().request_redraw();
            }
            _ => {}
        }
    });
}

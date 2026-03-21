//! DOM-embedded GPU surface manager for draw-heavy widgets on web.

#![cfg(target_arch = "wasm32")]

use sparsh_core::{Color, Rect};
use sparsh_render::{DrawList, Renderer};
use sparsh_text::TextSystem;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement};

pub struct SurfaceFrame {
    pub css_bounds: Rect,
    pub scale_factor: f32,
    pub elapsed_time: f32,
    pub draw_list: DrawList,
}

pub struct HybridSurfaceManager {
    root: HtmlElement,
    bootstrap_canvas: HtmlCanvasElement,
    canvases: Vec<HtmlCanvasElement>,
    canvas_states: Vec<CanvasState>,
    gpu_state: Rc<RefCell<Option<HybridGpuState>>>,
    init_started: bool,
}

#[derive(Default)]
struct CanvasState {
    display: String,
    left: String,
    top: String,
    width: String,
    height: String,
    pixel_width: u32,
    pixel_height: u32,
}

struct HybridGpuState {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    slots: Vec<Option<SurfaceSlot>>,
}

struct SurfaceSlot {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    text_system: TextSystem,
}

impl HybridSurfaceManager {
    pub fn new(root: &HtmlElement) -> Result<Self, wasm_bindgen::JsValue> {
        let document = root
            .owner_document()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("missing owner document"))?;
        let bootstrap_canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        bootstrap_canvas.style().set_property("display", "none")?;
        root.append_child(&bootstrap_canvas)?;

        Ok(Self {
            root: root.clone(),
            bootstrap_canvas,
            canvases: Vec::new(),
            canvas_states: Vec::new(),
            gpu_state: Rc::new(RefCell::new(None)),
            init_started: false,
        })
    }

    pub fn start_init(&mut self) {
        if self.init_started || self.gpu_state.borrow().is_some() {
            return;
        }
        self.init_started = true;

        let gpu_state = Rc::clone(&self.gpu_state);
        let bootstrap_canvas = self.bootstrap_canvas.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match HybridGpuState::new(bootstrap_canvas).await {
                Ok(state) => {
                    *gpu_state.borrow_mut() = Some(state);
                }
                Err(err) => {
                    log::error!("failed to initialize hybrid surface manager: {err}");
                }
            }
        });
    }

    pub fn render(
        &mut self,
        frames: &[SurfaceFrame],
        clear_color: Color,
    ) -> Result<(), wasm_bindgen::JsValue> {
        for (index, frame) in frames.iter().enumerate() {
            let canvas = self.ensure_canvas(index)?;
            self.update_canvas_node(index, frame)?;

            let physical_width =
                ((frame.css_bounds.width * frame.scale_factor).round() as u32).max(1);
            let physical_height =
                ((frame.css_bounds.height * frame.scale_factor).round() as u32).max(1);
            let state = &mut self.canvas_states[index];
            if state.pixel_width != physical_width {
                canvas.set_width(physical_width);
                state.pixel_width = physical_width;
            }
            if state.pixel_height != physical_height {
                canvas.set_height(physical_height);
                state.pixel_height = physical_height;
            }

            if let Some(gpu) = self.gpu_state.borrow_mut().as_mut() {
                gpu.ensure_slot(index, canvas.clone())
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err))?;
                gpu.render_slot(index, frame, clear_color)
                    .map_err(|err| wasm_bindgen::JsValue::from_str(&err))?;
            }
        }

        for index in frames.len()..self.canvases.len() {
            self.set_canvas_style(index, "display", "none")?;
        }

        Ok(())
    }

    fn ensure_canvas(&mut self, index: usize) -> Result<HtmlCanvasElement, wasm_bindgen::JsValue> {
        if let Some(canvas) = self.canvases.get(index) {
            return Ok(canvas.clone());
        }

        let document = self
            .root
            .owner_document()
            .ok_or_else(|| wasm_bindgen::JsValue::from_str("missing owner document"))?;
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        set_style(&canvas, "position", "absolute")?;
        set_style(&canvas, "pointer-events", "none")?;
        set_style(&canvas, "display", "none")?;
        if let Some(first) = self.root.first_child() {
            self.root.insert_before(&canvas, Some(&first))?;
        } else {
            self.root.append_child(&canvas)?;
        }
        self.canvases.push(canvas.clone());
        self.canvas_states.push(CanvasState::default());
        Ok(canvas)
    }

    fn update_canvas_node(
        &mut self,
        index: usize,
        frame: &SurfaceFrame,
    ) -> Result<(), wasm_bindgen::JsValue> {
        self.set_canvas_style(index, "display", "block")?;
        self.set_canvas_style(index, "left", px(frame.css_bounds.x))?;
        self.set_canvas_style(index, "top", px(frame.css_bounds.y))?;
        self.set_canvas_style(index, "width", px(frame.css_bounds.width))?;
        self.set_canvas_style(index, "height", px(frame.css_bounds.height))?;
        Ok(())
    }

    fn set_canvas_style(
        &mut self,
        index: usize,
        key: &'static str,
        value: impl Into<String>,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let value = value.into();
        let state = &mut self.canvas_states[index];
        let slot = match key {
            "display" => &mut state.display,
            "left" => &mut state.left,
            "top" => &mut state.top,
            "width" => &mut state.width,
            "height" => &mut state.height,
            _ => {
                return Err(wasm_bindgen::JsValue::from_str(
                    "unexpected canvas style key",
                ))
            }
        };
        if *slot == value {
            return Ok(());
        }
        set_style(&self.canvases[index], key, &value)?;
        *slot = value;
        Ok(())
    }
}

impl HybridGpuState {
    async fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL;
        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|err| format!("failed to create bootstrap surface: {err}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .map_err(|_| "no compatible GPU adapter available".to_owned())?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("hybrid-surface-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|err| format!("failed to request GPU device: {err}"))?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            slots: Vec::new(),
        })
    }

    fn ensure_slot(&mut self, index: usize, canvas: HtmlCanvasElement) -> Result<(), String> {
        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, || None);
        }
        if self.slots[index].is_some() {
            return Ok(());
        }

        let surface: wgpu::Surface<'static> = self
            .instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|err| format!("failed to create surface: {err}"))?;
        let caps = surface.get_capabilities(&self.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| "surface did not report any supported formats".to_owned())?;
        let present_mode = caps
            .present_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::PresentMode::AutoVsync || *mode == wgpu::PresentMode::Fifo)
            .or_else(|| caps.present_modes.first().copied())
            .ok_or_else(|| "surface did not report any supported present modes".to_owned())?;
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 1,
            height: 1,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&self.device, &config);

        self.slots[index] = Some(SurfaceSlot {
            surface,
            config,
            renderer: Renderer::new(&self.device, format),
            text_system: TextSystem::new(&self.device),
        });
        Ok(())
    }

    fn render_slot(
        &mut self,
        index: usize,
        frame: &SurfaceFrame,
        clear_color: Color,
    ) -> Result<(), String> {
        let slot = self.slots[index]
            .as_mut()
            .ok_or_else(|| "surface slot missing".to_owned())?;
        let width = ((frame.css_bounds.width * frame.scale_factor).round() as u32).max(1);
        let height = ((frame.css_bounds.height * frame.scale_factor).round() as u32).max(1);
        if slot.config.width != width || slot.config.height != height {
            slot.config.width = width;
            slot.config.height = height;
            slot.surface.configure(&self.device, &slot.config);
        }

        slot.renderer
            .set_viewport(width as f32, height as f32, frame.scale_factor);
        slot.renderer.set_time(frame.elapsed_time);
        slot.renderer.prepare(
            &self.device,
            &self.queue,
            &frame.draw_list,
            &mut slot.text_system,
        );

        let surface_texture = match slot.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                slot.surface.configure(&self.device, &slot.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                slot.surface.configure(&self.device, &slot.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                return Ok(());
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("hybrid_surface_encoder"),
            });
        slot.renderer.render(
            &mut encoder,
            &view,
            wgpu::Color {
                r: clear_color.r as f64,
                g: clear_color.g as f64,
                b: clear_color.b as f64,
                a: clear_color.a as f64,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
        Ok(())
    }
}

fn set_style(
    node: &HtmlCanvasElement,
    key: &str,
    value: &str,
) -> Result<(), wasm_bindgen::JsValue> {
    node.style().set_property(key, value)
}

fn px(value: f32) -> String {
    format!("{value}px")
}

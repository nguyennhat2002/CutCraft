//! egui-wgpu bridge for the renderer-owned Preview pipeline.
//!
//! Upload happens in the UI update tick; this callback only records draw
//! commands into the render pass that egui-wgpu already owns.

use std::sync::{Arc, Mutex};

use capcut_renderer::PreviewRenderer;
use eframe::egui_wgpu::{Callback, CallbackResources, CallbackTrait};
use eframe::wgpu::RenderPass;
use egui::epaint::PaintCallbackInfo;
use egui::{Rect, Shape, Ui};

struct GpuPreviewCallback {
    renderer: Arc<Mutex<PreviewRenderer>>,
}

impl CallbackTrait for GpuPreviewCallback {
    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut RenderPass<'static>,
        _callback_resources: &CallbackResources,
    ) {
        if let Ok(mut renderer) = self.renderer.lock() {
            let _ = renderer.paint(render_pass);
        }
    }
}

pub fn add_gpu_preview(ui: &Ui, canvas: Rect, renderer: &Arc<Mutex<PreviewRenderer>>) {
    let callback = Callback::new_paint_callback(
        canvas,
        GpuPreviewCallback {
            renderer: Arc::clone(renderer),
        },
    );
    ui.painter().add(Shape::Callback(callback));
}

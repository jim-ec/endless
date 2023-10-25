#![allow(dead_code)]

mod camera;
mod field;
mod renderer;
mod symmetry;
mod util;
mod world;

use cgmath::{vec3, InnerSpace, Vector3, Zero};
use egui::mutex::Mutex;
use pollster::FutureExt;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::{mpsc, Arc};
use std::thread;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use world::{Chunk, N};

pub const FRAME_TIME: f32 = 1.0 / 60.0;

fn main() {
    run().block_on()
}

async fn run() {
    env_logger::init();
    let mut last_render_time = Instant::now();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("")
        .with_visible(false)
        .with_maximized(true)
        .build(&event_loop)
        .unwrap();

    let mut renderer = renderer::Renderer::new(&window).await;

    let mut camera = camera::Camera::default();
    let player_cell = Arc::new(Mutex::new(Vector3::zero()));
    let mut w_down = false;
    let mut s_down = false;
    let mut a_down = false;
    let mut d_down = false;
    let mut shift_down = false;
    let mut alt_down = false;

    let mut events = vec![];

    let (chunk_sender, chunk_receiver) = mpsc::channel::<(Vector3<isize>, Chunk)>();
    let mut world = world::World::default();
    let tasks: Arc<Mutex<HashMap<Vector3<isize>, usize>>> = Default::default();

    {
        let device = renderer.device.clone();
        let tasks = tasks.clone();
        let player_cell = player_cell.clone();
        thread::spawn(move || loop {
            let (key, lod) = {
                let tasks = tasks.lock();
                let player_cell = *player_cell.lock();

                // Get next task, order by distance to camera
                let Some((&key, &lod)) = tasks.iter().min_by_key(|(&key, &lod)| {
                    let distance = (key - player_cell).magnitude2();
                    (distance, lod)
                }) else {
                    // No task available, sleep for a bit
                    thread::yield_now();
                    continue;
                };

                (key, lod)
            };

            // Generate the chunk. This can take a long time.
            let chunk = world::Chunk::new(key, lod, &device);

            {
                // Check if the task is still valid
                let mut tasks = tasks.lock();
                if let Some(&new_lod) = tasks.get(&key) {
                    if lod == new_lod {
                        // Worker generated the chunk we wanted
                        tasks.remove(&key);
                        chunk_sender.send((key, chunk)).unwrap();
                    }
                };
            }
        });
    }

    event_loop.run(move |event, _, control_flow| match event {
        Event::NewEvents(winit::event::StartCause::Init) => {
            window.set_visible(true);
        }

        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(code),
                        state,
                        ..
                    },
                ..
            } => {
                *match code {
                    VirtualKeyCode::W => &mut w_down,
                    VirtualKeyCode::S => &mut s_down,
                    VirtualKeyCode::A => &mut a_down,
                    VirtualKeyCode::D => &mut d_down,
                    VirtualKeyCode::LShift => &mut shift_down,
                    VirtualKeyCode::RShift => &mut shift_down,
                    VirtualKeyCode::LAlt => &mut alt_down,
                    VirtualKeyCode::RAlt => &mut alt_down,
                    _ => return,
                } = state == ElementState::Pressed;
            }

            WindowEvent::Resized(size)
            | WindowEvent::ScaleFactorChanged {
                new_inner_size: &mut size,
                ..
            } => {
                renderer.resize(size.width, size.height);
                window.request_redraw();
            }

            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::PixelDelta(delta),
                ..
            } => {
                camera.yaw += camera.fovy * camera.up().z.signum() * 0.00008 * -delta.x as f32;
                camera.pitch += camera.fovy * 0.00008 * -delta.y as f32;
                camera.pitch = camera.pitch.clamp(-0.25 * TAU, 0.25 * TAU);
            }

            WindowEvent::CursorMoved { position, .. } => {
                events.push(egui::Event::PointerMoved(egui::pos2(
                    position.x as f32 / window.scale_factor() as f32,
                    position.y as f32 / window.scale_factor() as f32,
                )));
            }

            WindowEvent::MouseInput { button, state, .. } => {
                let Some(pos) = events
                    .iter()
                    .rev()
                    .filter_map(|e| match e {
                        &egui::Event::PointerMoved(pos) => Some(pos),
                        _ => None,
                    })
                    .next()
                else {
                    return;
                };
                events.push(egui::Event::PointerButton {
                    pos,
                    button: match button {
                        MouseButton::Left => egui::PointerButton::Primary,
                        MouseButton::Right => egui::PointerButton::Secondary,
                        MouseButton::Middle => egui::PointerButton::Middle,
                        MouseButton::Other(_) => return,
                    },
                    pressed: state == ElementState::Pressed,
                    modifiers: Default::default(),
                })
            }

            WindowEvent::TouchpadMagnify { delta, .. } => {
                camera.fovy *= 1.0 + 0.5 * -delta as f32;
                camera.fovy = camera.fovy.min(180.0);
            }

            WindowEvent::SmartMagnify { .. } => {
                camera.fovy = camera::Camera::initial().fovy;
            }

            _ => {}
        },

        Event::RedrawRequested(..) => {
            last_render_time = Instant::now();

            let mut translation = Vector3::new(0.0, 0.0, 0.0);
            if w_down && !alt_down {
                translation += camera.forward();
            }
            if w_down && alt_down {
                translation.z += camera.up().z.signum();
            }
            if s_down && !alt_down {
                translation -= camera.forward();
            }
            if s_down && alt_down {
                translation.z -= camera.up().z.signum();
            }
            if a_down {
                translation += camera.left();
            }
            if d_down {
                translation -= camera.left();
            }
            if translation.magnitude2() > 0.0 {
                let speed = if shift_down { 500.0 } else { 100.0 };
                camera.translation += FRAME_TIME * speed * translation.normalize_to(1.0);
            }

            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(renderer.config.width as f32, renderer.config.height as f32),
                )),
                pixels_per_point: Some(window.scale_factor() as f32),
                events: std::mem::take(&mut events),
                ..Default::default()
            };

            let ui_output = renderer.ctx().run(input, |ctx| {
                egui::Window::new("")
                    .title_bar(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        #[cfg(debug_assertions)]
                        ui.label(egui::RichText::new("Debug Build").strong());
                        #[cfg(not(debug_assertions))]
                        ui.label(egui::RichText::new("Release Build").strong());

                        ui.add(egui::Slider::new(&mut camera.fovy, 1.0..=180.0).text("FoV"));
                    });
            });

            window.set_cursor_icon(match ui_output.platform_output.cursor_icon {
                egui::CursorIcon::Default => winit::window::CursorIcon::Default,
                egui::CursorIcon::None => winit::window::CursorIcon::Default,
                egui::CursorIcon::ContextMenu => winit::window::CursorIcon::ContextMenu,
                egui::CursorIcon::Help => winit::window::CursorIcon::Help,
                egui::CursorIcon::PointingHand => winit::window::CursorIcon::Hand,
                egui::CursorIcon::Progress => winit::window::CursorIcon::Progress,
                egui::CursorIcon::Wait => winit::window::CursorIcon::Wait,
                egui::CursorIcon::Cell => winit::window::CursorIcon::Cell,
                egui::CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
                egui::CursorIcon::Text => winit::window::CursorIcon::Text,
                egui::CursorIcon::VerticalText => winit::window::CursorIcon::VerticalText,
                egui::CursorIcon::Alias => winit::window::CursorIcon::Alias,
                egui::CursorIcon::Copy => winit::window::CursorIcon::Copy,
                egui::CursorIcon::Move => winit::window::CursorIcon::Move,
                egui::CursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
                egui::CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
                egui::CursorIcon::Grab => winit::window::CursorIcon::Grab,
                egui::CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
                egui::CursorIcon::AllScroll => winit::window::CursorIcon::AllScroll,
                egui::CursorIcon::ResizeHorizontal => winit::window::CursorIcon::EwResize,
                egui::CursorIcon::ResizeNeSw => winit::window::CursorIcon::NeswResize,
                egui::CursorIcon::ResizeNwSe => winit::window::CursorIcon::NwseResize,
                egui::CursorIcon::ResizeVertical => winit::window::CursorIcon::NsResize,
                egui::CursorIcon::ResizeEast => winit::window::CursorIcon::EResize,
                egui::CursorIcon::ResizeSouthEast => winit::window::CursorIcon::SeResize,
                egui::CursorIcon::ResizeSouth => winit::window::CursorIcon::SResize,
                egui::CursorIcon::ResizeSouthWest => winit::window::CursorIcon::SwResize,
                egui::CursorIcon::ResizeWest => winit::window::CursorIcon::WResize,
                egui::CursorIcon::ResizeNorthWest => winit::window::CursorIcon::NwResize,
                egui::CursorIcon::ResizeNorth => winit::window::CursorIcon::NResize,
                egui::CursorIcon::ResizeNorthEast => winit::window::CursorIcon::NeResize,
                egui::CursorIcon::ResizeColumn => winit::window::CursorIcon::ColResize,
                egui::CursorIcon::ResizeRow => winit::window::CursorIcon::RowResize,
                egui::CursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
                egui::CursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
            });

            let camera_index: Vector3<isize> = (camera.translation / world::N as f32)
                .map(f32::floor)
                .cast()
                .unwrap();

            while let Ok((key, chunk)) = chunk_receiver.try_recv() {
                world.chunks.insert(
                    key,
                    world::Chunk {
                        lod: chunk.lod,
                        mask: chunk.mask,
                        color: chunk.color,
                        voxel_mesh: chunk.voxel_mesh,
                    },
                );
            }

            let lod_shift = 2;
            let radius = 4;

            let mut required_chunks = HashSet::new();

            *player_cell.lock() = camera_index;

            // Delete chunks that are outside the generation radius
            world.chunks.retain(|&key, _| {
                let d = (key - camera_index).map(isize::abs);
                d.x <= radius && d.y <= radius
            });

            // Gather all required chunks and their LoDs based on the camera position
            for x in -radius..=radius {
                for y in -radius..=radius {
                    for z in 0..=1 {
                        let c = vec3(camera_index.x + x, camera_index.y + y, z);
                        let lod = x.unsigned_abs().min(y.unsigned_abs());
                        let lod = lod >> lod_shift;
                        if (N >> lod) > 0 {
                            required_chunks.insert((c, lod));
                        }
                    }
                }
            }

            // Record new chunk generation tasks
            {
                let mut tasks = tasks.lock();
                for (key, lod) in required_chunks {
                    // Check if the task is already in progress
                    if let Some(&old_lod) = tasks.get(&key) {
                        if lod == old_lod {
                            continue;
                        }
                    }

                    // Check if the task is already done
                    if let Some(chunk) = world.chunks.get(&key) {
                        if chunk.lod == lod {
                            continue;
                        }
                    }

                    tasks.insert(key, lod);
                }
            }

            match renderer.render(
                camera,
                ui_output,
                &world.chunks,
                window.scale_factor() as f32,
            ) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    renderer.resize(renderer.config.width, renderer.config.height);
                }
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(wgpu::SurfaceError::Timeout) | Err(wgpu::SurfaceError::Outdated) => (),
            }
        }

        Event::MainEventsCleared => {
            let target_frametime = Duration::from_secs_f32(FRAME_TIME);
            let time_since_last_frame = last_render_time.elapsed();
            if time_since_last_frame >= target_frametime {
                window.request_redraw();
            } else {
                *control_flow = ControlFlow::WaitUntil(
                    Instant::now() + target_frametime - time_since_last_frame,
                );
            }
        }

        _ => {}
    });
}

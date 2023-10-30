#![allow(dead_code)]

mod camera;
mod field;
mod renderer;
mod symmetry;
mod util;
mod world;

use cgmath::{vec3, InnerSpace, Vector3, Zero};
use egui::mutex::Mutex;
use itertools::Itertools;
use pollster::FutureExt;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use world::{Chunk, N};

use crate::world::K;

pub const FRAME_TIME: f32 = 1.0 / 60.0;

fn main() {
    run().block_on()
}

async fn run() {
    env_logger::init();

    // {
    //     let n = 256;
    //     let mut img = bmp::Image::new(n, n);
    //     for i in 0..n {
    //         for j in 0..n {
    //             let x = j as f32;
    //             let y = i as f32;

    //             let g = util::random(x, y);
    //             let g = (g * 255.0) as u8;
    //             img.set_pixel(j, i, bmp::Pixel::new(g, g, g));
    //         }
    //     }
    //     img.save("noise.bmp").unwrap();
    //     return;
    // }

    puffin::set_scopes_on(true);

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
    let mut max_lod = K >> 1;
    let mut lod_shift = 2;
    let mut enable_gizmos = false;
    let mut invert_x_axis = false;
    let mut invert_y_axis = false;

    let mut stats = renderer::RenderStats::default();
    let mut frame_time_counter = util::Counter::default();
    let mut show_profiler = false;

    let chunk_generation_time = Arc::new(Mutex::new(0.0));

    #[derive(Default)]
    struct Tasks {
        task_list: HashMap<Vector3<isize>, usize>,
        in_progress: HashMap<Vector3<isize>, usize>,
    }
    let tasks: Arc<Mutex<Tasks>> = Default::default();

    // Spawn chunk worker threads
    for i in 0..8 {
        let device = renderer.device.clone();
        let tasks = tasks.clone();
        let player_cell = player_cell.clone();
        let chunk_sender = chunk_sender.clone();
        let chunk_generation_time = chunk_generation_time.clone();
        thread::Builder::new()
            .name(format!("Worker #{i}"))
            .spawn(move || loop {
                let (key, lod) = {
                    let mut tasks = tasks.lock();
                    let player_cell = *player_cell.lock();

                    // Get next task, order by distance to camera
                    let Some((&key, &lod)) = tasks
                        .task_list
                        .iter()
                        .filter(|(key, &lod)| {
                            if let Some(&in_progress_lod) = tasks.in_progress.get(key) {
                                lod != in_progress_lod
                            } else {
                                true
                            }
                        })
                        .min_by_key(|(&key, &lod)| {
                            let distance = (key - player_cell).magnitude2();
                            (distance, lod)
                        })
                    else {
                        // No task available, sleep for a bit
                        thread::yield_now();
                        continue;
                    };

                    tasks.in_progress.insert(key, lod);
                    (key, lod)
                };

                // Generate the chunk. This can take a long time.
                let start = Instant::now();
                let chunk = world::Chunk::new(key, lod, &device);
                let elapsed = start.elapsed().as_millis();
                if lod == 0 {
                    let mut chunk_generation_time = chunk_generation_time.lock();
                    *chunk_generation_time = 0.9 * *chunk_generation_time + 0.1 * elapsed as f32;
                }

                {
                    // Check if the task is still valid
                    let mut tasks = tasks.lock();
                    if let Some(&new_lod) = tasks.in_progress.get(&key) {
                        if lod == new_lod {
                            // Worker generated the chunk we wanted
                            tasks.in_progress.remove(&key);
                        }
                    };
                    if let Some(&new_lod) = tasks.task_list.get(&key) {
                        if lod == new_lod {
                            // Worker generated the chunk we wanted
                            tasks.task_list.remove(&key);
                            chunk_sender.send((key, chunk)).unwrap();
                        }
                    };
                }
            })
            .unwrap();
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
                camera.yaw +=
                    camera.fovy * 0.00008 * if invert_x_axis { delta.x } else { -delta.x } as f32;
                camera.pitch +=
                    camera.fovy * 0.00008 * if invert_y_axis { delta.y } else { -delta.y } as f32;
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
            puffin::GlobalProfiler::lock().new_frame();

            frame_time_counter.push(last_render_time.elapsed().as_secs_f32());

            last_render_time = Instant::now();

            let mut translation = Vector3::new(0.0, 0.0, 0.0);
            if w_down && !alt_down {
                translation += camera.forward();
            }
            if w_down && alt_down {
                translation.z += 1.0;
            }
            if s_down && !alt_down {
                translation -= camera.forward();
            }
            if s_down && alt_down {
                translation.z -= 1.0;
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
                puffin::profile_scope!("UI");
                egui::Window::new("Inspector").show(ctx, |ui| {
                    #[cfg(debug_assertions)]
                    ui.label(egui::RichText::new("Debug Build").strong());
                    #[cfg(not(debug_assertions))]
                    ui.label(egui::RichText::new("Release Build").strong());
                    if show_profiler {
                        show_profiler = !ui.button("Close Profiler").clicked();
                    } else {
                        show_profiler = ui.button("Open Profiler").clicked();
                    }

                    egui::CollapsingHeader::new("Renderer")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!("Processor: {}", renderer.adapter.get_info().name));
                            ui.label(format!(
                                "Backend: {:?}",
                                renderer.adapter.get_info().backend
                            ));
                            ui.label(format!("FPS: {:.0}", 1.0 / frame_time_counter.smoothed));
                            ui.label(format!(
                                "Frame Time: {:.2}ms",
                                1000.0 * frame_time_counter.smoothed
                            ));

                            {
                                let desired_size = egui::vec2(ui.available_width(), 30.0);

                                let (rect, _) = ui.allocate_exact_size(
                                    desired_size,
                                    egui::Sense::focusable_noninteractive(),
                                );

                                if ui.is_rect_visible(rect) {
                                    ui.painter().rect(
                                        rect,
                                        2.0,
                                        egui::Color32::from_additive_luminance(40),
                                        egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_additive_luminance(80),
                                        ),
                                    );

                                    for (x, (a, b)) in frame_time_counter
                                        .measures
                                        .iter()
                                        .copied()
                                        .map(f32::recip)
                                        .tuple_windows()
                                        .enumerate()
                                    {
                                        let xa = egui::lerp(
                                            rect.left()..=rect.right(),
                                            x as f32 / util::MAX_COUNTER_HISTORY as f32,
                                        );
                                        let xb = egui::lerp(
                                            rect.left()..=rect.right(),
                                            (x + 1) as f32 / util::MAX_COUNTER_HISTORY as f32,
                                        );
                                        let ya = rect.bottom() - desired_size.y / 60.0 * a;
                                        let yb = rect.bottom() - desired_size.y / 60.0 * b;
                                        ui.painter().line_segment(
                                            [egui::pos2(xa, ya), egui::pos2(xb, ya)],
                                            egui::Stroke::new(
                                                1.5,
                                                egui::Color32::from_additive_luminance(150),
                                            ),
                                        );
                                        ui.painter().line_segment(
                                            [egui::pos2(xb, ya), egui::pos2(xb, yb)],
                                            egui::Stroke::new(
                                                1.5,
                                                egui::Color32::from_additive_luminance(150),
                                            ),
                                        );
                                    }
                                }
                            }
                        });

                    egui::CollapsingHeader::new("Chunks")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.label(format!("Side Extent: {N}"));
                            ui.label(format!("Total: {}", world.chunks.len()));
                            ui.label(format!("Rendered: {}", stats.chunk_count));
                            ui.label(format!("Generation Radius: {}", max_lod << lod_shift));
                            ui.label(format!(
                                "Generation Time: {:.0}ms",
                                *chunk_generation_time.lock()
                            ));
                            ui.add(egui::Slider::new(&mut max_lod, 0..=K).text("Max LoD"));
                            ui.add(egui::Slider::new(&mut lod_shift, 0..=6).text("LoD Exp Scale"));
                        });

                    egui::CollapsingHeader::new("Misc")
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.add(egui::Slider::new(&mut camera.fovy, 1.0..=180.0).text("FoV"));
                            ui.checkbox(&mut enable_gizmos, "Gizmos");
                            ui.horizontal(|ui| {
                                ui.label("Camera:");
                                ui.checkbox(&mut invert_x_axis, "Invert X");
                                ui.checkbox(&mut invert_y_axis, "Invert Y");
                            });
                        });
                });

                if show_profiler {
                    show_profiler = puffin_egui::profiler_window(ctx);
                }
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
                world.chunks.insert(key, chunk);
            }

            let mut required_chunks = HashMap::new();

            *player_cell.lock() = camera_index;

            // Gather all required chunks and their LoDs based on the camera position
            let generation_radius = (K << lod_shift) as isize;
            for x in -generation_radius..=generation_radius {
                for y in -generation_radius..=generation_radius {
                    for z in 0..=1 {
                        let c = vec3(camera_index.x + x, camera_index.y + y, z);
                        let lod = ((x.pow(2) + y.pow(2)) as f32).sqrt() as usize;
                        let lod = lod >> lod_shift;
                        if lod <= max_lod {
                            required_chunks.insert(c, lod);
                        }
                    }
                }
            }

            // Delete chunks that are outside the generation radius
            world
                .chunks
                .retain(|key, _| required_chunks.contains_key(key));

            // Record new chunk generation tasks
            {
                puffin::profile_scope!("Record Tasks");

                let mut tasks = tasks.lock();

                // Cancel outdated tasks which are not yet in progress
                tasks
                    .task_list
                    .retain(|key, lod| required_chunks.get(key).copied() == Some(*lod));

                for (key, lod) in required_chunks {
                    // Check if the task is already in progress
                    if let Some(&old_lod) = tasks.task_list.get(&key) {
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

                    tasks.task_list.insert(key, lod);
                }

                for key in tasks.in_progress.keys() {
                    renderer.gizmos.aabb(
                        N as f32 * key.cast().unwrap(),
                        N as f32 * (key + vec3(1, 1, 1)).cast().unwrap(),
                        util::rgb(0, 255, 0),
                    );
                }
            }

            match renderer.render(
                camera,
                ui_output,
                &world.chunks,
                window.scale_factor() as f32,
                enable_gizmos,
            ) {
                Ok(new_stats) => stats = new_stats,
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

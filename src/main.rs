// Release builds are a GUI app on Windows: no console window behind the game.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod corpus;
mod demo;
mod kana;
mod rng;
mod scene;

use bevy::{
    prelude::*,
    render::{RenderPlugin, pipelined_rendering::PipelinedRenderingPlugin},
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use std::num::NonZero;

/// Reverse-DNS id shared by the Wayland app_id, X11 WM_CLASS, the .desktop file, the hicolor icon and the macOS bundle.
const APP_ID: &str = "io.github.brennschlus.kanablade";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let no_vsync = args.iter().any(|a| a == "--no-vsync");
    let text_file = args.iter().find(|a| !a.starts_with("--")).cloned();

    let lines = match &text_file {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(src) => corpus::parse(&src),
            Err(e) => {
                eprintln!("не удалось прочитать {path}: {e}");
                std::process::exit(1);
            }
        },
        None => corpus::parse(corpus::BUILTIN),
    };
    if lines.is_empty() {
        eprintln!("в тексте нет ни одной фразы");
        std::process::exit(1);
    }

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "kanablade · 禅".into(),
                    name: Some(APP_ID.into()),
                    resolution: WindowResolution::new(1280, 760),
                    present_mode: if no_vsync { PresentMode::AutoNoVsync } else { PresentMode::AutoVsync },
                    // Never queue frames ahead: the glyph you just typed is on the next scanout.
                    desired_maximum_frame_latency: NonZero::new(1),
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin { synchronous_pipeline_compilation: true, ..default() })
            // Pipelined rendering adds a frame of input latency.
            .disable::<PipelinedRenderingPlugin>(),
    )
    .insert_resource(WinitSettings::game())
    .insert_resource(ClearColor(Color::srgb(0.035, 0.038, 0.05)))
    .add_plugins((audio::AudioPlugin, scene::ScenePlugin { lines }));
    #[cfg(any(windows, target_os = "linux"))]
    app.add_systems(Update, set_window_icon);
    if let Ok(dir) = std::env::var("KANABLADE_DEMO") {
        app.add_plugins(demo::DemoPlugin(dir));
    }
    app.run();
}

/// Title bar and taskbar icon. macOS takes it from the bundle and Wayland from the .desktop file,
/// so this matters on Windows (the .ico resource, all sizes) and X11 (a 256px PNG).
#[cfg(any(windows, target_os = "linux"))]
fn set_window_icon(
    window: Single<Entity, With<bevy::window::PrimaryWindow>>,
    mut done: Local<bool>,
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    if *done {
        return;
    }
    // The winit window appears a frame or so after the entity, so keep trying until it is there.
    bevy::winit::WINIT_WINDOWS.with_borrow(|winit_windows| {
        let Some(window) = winit_windows.get_window(*window) else { return };
        *done = true;

        #[cfg(windows)]
        {
            use winit::{
                dpi::PhysicalSize,
                platform::windows::{IconExtWindows, WindowExtWindows},
                window::Icon,
            };
            // Windows picks the best match from the .ico: 16pt for the title bar, system size for the taskbar.
            let small = (16.0 * window.scale_factor()).round() as u32;
            window.set_window_icon(Icon::from_resource(1, Some(PhysicalSize::new(small, small))).ok());
            window.set_taskbar_icon(Icon::from_resource(1, None).ok());
        }
        #[cfg(target_os = "linux")]
        {
            let png = include_bytes!("../packaging/icons/hicolor/256x256/apps/io.github.brennschlus.kanablade.png");
            let icon = image::load_from_memory(png).ok().and_then(|img| {
                let img = img.into_rgba8();
                let (w, h) = img.dimensions();
                winit::window::Icon::from_rgba(img.into_raw(), w, h).ok()
            });
            window.set_window_icon(icon);
        }
    });
}

mod audio;
mod corpus;
mod demo;
mod kana;
mod scene;

use bevy::{
    prelude::*,
    render::{RenderPlugin, pipelined_rendering::PipelinedRenderingPlugin},
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use std::num::NonZero;

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
    app
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "kanablade · 禅".into(),
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
    if let Ok(dir) = std::env::var("KANABLADE_DEMO") {
        app.add_plugins(demo::DemoPlugin(dir));
    }
    app.run();
}

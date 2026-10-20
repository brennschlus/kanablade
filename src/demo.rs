//! `KANABLADE_DEMO=<dir>`: types on its own, saves screenshots, exits. Used to check visuals.

use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    },
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::PrimaryWindow,
};

const SCRIPT: &str = "furuikeyakawazutobikomumizunooto";
const SHOTS: [f32; 3] = [2.2, 4.4, 5.6];

pub struct DemoPlugin(pub String);

#[derive(Resource)]
struct Demo {
    dir: String,
    typed: usize,
    shots: usize,
    script: Vec<char>,
}

impl Plugin for DemoPlugin {
    fn build(&self, app: &mut App) {
        let script = std::env::var("KANABLADE_SCRIPT").unwrap_or_else(|_| SCRIPT.into());
        app.insert_resource(Demo { dir: self.0.clone(), typed: 0, shots: 0, script: script.chars().collect() })
            .add_systems(PreUpdate, drive);
    }
}

fn code(c: char) -> KeyCode {
    use KeyCode::*;
    #[rustfmt::skip]
    const KEYS: [KeyCode; 26] = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ,
        KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    match c {
        'a'..='z' => KEYS[(c as u8 - b'a') as usize],
        '-' => Minus,
        '\'' => Quote,
        '#' => Tab,
        _ => Space,
    }
}

fn drive(
    mut commands: Commands,
    mut demo: ResMut<Demo>,
    mut keys: MessageWriter<KeyboardInput>,
    mut exit: MessageWriter<AppExit>,
    window: Single<Entity, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let t = time.elapsed_secs() - 1.0;
    let want = ((t * 9.0).max(0.0) as usize).min(demo.script.len());
    while demo.typed < want {
        let c = demo.script[demo.typed];
        demo.typed += 1;
        keys.write(KeyboardInput {
            key_code: code(c),
            logical_key: Key::Character(c.to_string().into()),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: *window,
        });
    }
    if demo.shots < SHOTS.len() && t + 1.0 >= SHOTS[demo.shots] {
        let path = format!("{}/shot{}.png", demo.dir, demo.shots);
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
        demo.shots += 1;
    }
    if t + 1.0 > SHOTS[SHOTS.len() - 1] + 0.6 {
        exit.write(AppExit::Success);
    }
}

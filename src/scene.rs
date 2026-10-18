//! The zen karaoke scene: lines drift in, light up glyph by glyph, and float away.

use crate::{
    audio::{self, Audio, Cue},
    corpus::Line,
    kana::{self, Matcher},
};
use bevy::{
    asset::RenderAssetUsages,
    camera::Hdr,
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    image::Image,
    input::{ButtonState, keyboard::KeyboardInput},
    post_process::bloom::Bloom,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{MonitorSelection, PrimaryWindow, WindowMode},
};
use std::ops::Range;

// Design resolution; everything scales with the window.
const DESIGN: Vec2 = Vec2::new(1280.0, 760.0);
const GLYPH_PX: f32 = 66.0;
const LINE_Y: f32 = 40.0;
const NEXT_Y: f32 = -215.0;
const NEXT_SCALE: f32 = 0.5;
const ROMAJI_Y: f32 = -48.0;
const TRANSLATION_Y: f32 = -100.0;

// Palette (linear, HDR where > 1).
const DIM: Vec3 = Vec3::new(0.14, 0.15, 0.19);
const GOLD: Vec3 = Vec3::new(1.0, 0.70, 0.36);
const GOLD_HOT: Vec3 = Vec3::new(3.2, 2.1, 0.95);
const CURRENT: Vec3 = Vec3::new(0.95, 0.94, 0.92);
const VERMILION: Vec3 = Vec3::new(2.4, 0.32, 0.18);

pub struct ScenePlugin {
    pub lines: Vec<Line>,
}

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        let mut rng = Rng::seeded();
        let mut order: Vec<usize> = (0..self.lines.len()).collect();
        let shuffle = std::env::var_os("KANABLADE_DEMO").is_none();
        for i in (1..order.len()).rev().filter(|_| shuffle) {
            order.swap(i, rng.below(i + 1));
        }
        let matcher = Matcher::new(self.lines[order[0]].units.clone());
        app.insert_resource(Session {
            lines: self.lines.clone(),
            order,
            at: 0,
            matcher,
            hint: HintMode::Full,
            show_translation: true,
            keys_ok: 0,
            keys_miss: 0,
            units_done: 0,
            lines_done: 0,
            active_secs: 0.0,
            last_key: -100.0,
            miss_at: -100.0,
            line_started: 0.0,
            swallow_n_until: 0.0,
            toast: None,
        })
        .insert_resource(rng)
        .insert_resource(View { scale: 1.0, size: DESIGN })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_view,
                handle_input,
                rescale_text,
                animate_lines,
                animate_glyphs,
                (update_cursor, update_romaji).chain(),
                update_overlay,
                (update_motes, update_sparks, update_ripples, update_background),
            )
                .chain(),
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HintMode {
    Full,
    Syllable,
    Hidden,
}

#[derive(Resource)]
struct Session {
    lines: Vec<Line>,
    order: Vec<usize>,
    at: usize,
    matcher: Matcher,
    hint: HintMode,
    show_translation: bool,
    keys_ok: u32,
    keys_miss: u32,
    units_done: u32,
    lines_done: u32,
    active_secs: f32,
    last_key: f32,
    miss_at: f32,
    line_started: f32,
    swallow_n_until: f32,
    toast: Option<(String, f32)>,
}

impl Session {
    fn line_at(&self, k: usize) -> &Line {
        &self.lines[self.order[k % self.order.len()]]
    }
    fn current(&self) -> &Line {
        self.line_at(self.at)
    }
}

#[derive(Resource)]
struct View {
    scale: f32,
    size: Vec2,
}

#[derive(Resource)]
struct Assets2 {
    mincho: Handle<Font>,
    maru: Handle<Font>,
    glow: Handle<Image>,
    ring: Handle<Image>,
}

#[derive(Resource)]
struct Rng(u32);

impl Rng {
    fn seeded() -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.subsec_nanos() ^ d.as_secs() as u32);
        Rng(t | 1)
    }
    fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    fn unit(&mut self) -> f32 {
        self.next() as f32 / u32::MAX as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }
    fn below(&mut self, n: usize) -> usize {
        self.next() as usize % n
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Next,
    Current,
    Leaving,
}

#[derive(Component)]
struct LineRoot {
    role: Role,
    alpha: f32,
    /// Seconds since this root changed role.
    age: f32,
    y: f32,
    scale: f32,
}

/// Font size in design pixels; actual size follows the window.
#[derive(Component)]
struct Scaled(f32);

/// Position inside a line root, in design pixels.
#[derive(Component)]
struct Slot(Vec2);

#[derive(Component)]
struct Lit {
    units: Range<usize>,
    lit: f32,
    pop: f32,
    full: bool,
    ruby: bool,
}

#[derive(Component)]
struct Cursor {
    pos: Vec2,
    alpha: f32,
}

#[derive(Component)]
struct RomajiLine;

#[derive(Component)]
struct RomajiSpan(usize);

#[derive(Component)]
struct Translation {
    for_at: usize,
    alpha: f32,
}

#[derive(Component)]
struct Stats(f32);

#[derive(Component)]
struct Footer(f32);

#[derive(Component)]
struct Toast;

#[derive(Component)]
struct Background;

#[derive(Component)]
struct Mote {
    pos: Vec2,
    speed: f32,
    size: f32,
    phase: f32,
    alpha: f32,
}

#[derive(Component)]
struct Spark {
    vel: Vec2,
    life: f32,
    max: f32,
    size: f32,
}

#[derive(Component)]
struct Ripple {
    life: f32,
}

fn lin(c: Vec3, a: f32) -> Color {
    Color::LinearRgba(LinearRgba::new(c.x, c.y, c.z, a.clamp(0.0, 1.0)))
}

fn font(handle: &Handle<Font>, px: f32) -> TextFont {
    TextFont::from(handle.clone()).with_font_size(FontSize::Px(px))
}

// ---------------------------------------------------------------------------
// Setup

fn soft_dot(size: u32, ring: bool) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let c = (size as f32 - 1.0) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let d = Vec2::new(x as f32 - c, y as f32 - c).length() / c;
            let a = if ring {
                let k = (d - 0.9) / 0.035;
                (-k * k).exp() * (d < 1.0) as u8 as f32
            } else {
                let core = (1.0 - d).clamp(0.0, 1.0);
                core * core * (3.0 - 2.0 * core)
            };
            data.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn background_image() -> Image {
    let (w, h) = (64u32, 256u32);
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let v = y as f32 / (h - 1) as f32; // 0 top .. 1 bottom
            let u = x as f32 / (w - 1) as f32 * 2.0 - 1.0;
            let top = Vec3::new(0.085, 0.09, 0.135);
            let bottom = Vec3::new(0.035, 0.032, 0.04);
            let mut c = top.lerp(bottom, v.powf(0.8));
            // Faint warm horizon glow behind the text.
            let glow = (-((v - 0.48) * 4.0).powi(2)).exp() * (1.0 - u * u * 0.6);
            c += Vec3::new(0.045, 0.028, 0.02) * glow;
            let vignette = 1.0 - 0.35 * (u * u);
            c *= vignette;
            // Values are sRGB; a little noise hides banding in the dark gradient.
            let n = ((x * 7919 + y * 104_729) % 97) as f32 / 97.0 - 0.5;
            let s = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + n).round().clamp(0.0, 255.0) as u8;
            data.extend_from_slice(&[s(c.x), s(c.y), s(c.z), 255]);
        }
    }
    Image::new(
        Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn setup(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut images: ResMut<Assets<Image>>,
    session: Res<Session>,
    mut rng: ResMut<Rng>,
) {
    let a = Assets2 {
        mincho: fonts.add(Font::from_bytes(include_bytes!("../assets/fonts/ZenOldMincho-Regular.ttf").to_vec())),
        maru: fonts.add(Font::from_bytes(include_bytes!("../assets/fonts/ZenMaruGothic-Medium.ttf").to_vec())),
        glow: images.add(soft_dot(64, false)),
        ring: images.add(soft_dot(256, true)),
    };

    commands.spawn((
        Camera2d,
        Hdr,
        Tonemapping::None,
        DebandDither::Disabled,
        Bloom { intensity: 0.22, ..Bloom::NATURAL },
    ));

    commands.spawn((Background, Sprite::from_image(images.add(background_image())), Transform::from_xyz(0.0, 0.0, -10.0)));

    for _ in 0..90 {
        let size = rng.range(2.0, 7.0);
        commands.spawn((
            Mote {
                pos: Vec2::new(rng.unit(), rng.unit()),
                speed: rng.range(0.006, 0.02),
                size,
                phase: rng.range(0.0, 100.0),
                alpha: rng.range(0.04, 0.22) * (7.0 / size).sqrt().min(1.6),
            },
            Sprite { image: a.glow.clone(), color: Color::NONE, custom_size: Some(Vec2::splat(size)), ..default() },
            Transform::from_xyz(0.0, 0.0, -5.0),
        ));
    }

    spawn_line(&mut commands, &a, session.line_at(0), Role::Current);
    spawn_line(&mut commands, &a, session.line_at(1), Role::Next);

    commands.spawn((
        Cursor { pos: Vec2::ZERO, alpha: 0.0 },
        Sprite { image: a.glow.clone(), color: Color::NONE, custom_size: Some(Vec2::splat(16.0)), ..default() },
        Transform::from_xyz(0.0, 0.0, 0.5),
    ));

    commands
        .spawn((RomajiLine, Text2d::new(""), font(&a.maru, 26.0), Scaled(26.0), Transform::from_xyz(0.0, 0.0, 0.2)))
        .with_children(|p| {
            for i in 0..4 {
                p.spawn((TextSpan::new(""), font(&a.maru, 26.0), Scaled(26.0), TextColor(Color::NONE), RomajiSpan(i)));
            }
        });

    commands.spawn((
        Translation { for_at: usize::MAX, alpha: 0.0 },
        Text2d::new(""),
        font(&a.mincho, 21.0),
        Scaled(21.0),
        TextColor(Color::NONE),
    ));
    commands.spawn((Stats(0.0), Text2d::new(""), font(&a.maru, 16.0), Scaled(16.0), TextColor(Color::NONE)));
    commands.spawn((
        Footer(0.0),
        Text2d::new("Tab — подсказки     F2 — перевод     F3 — звук     Enter — следующая фраза     F11 — полный экран     Esc — выход"),
        font(&a.maru, 15.0),
        Scaled(15.0),
        TextColor(Color::NONE),
    ));
    commands.spawn((Toast, Text2d::new(""), font(&a.maru, 18.0), Scaled(18.0), TextColor(Color::NONE)));

    commands.insert_resource(a);
}

fn glyph_px(line: &Line) -> f32 {
    GLYPH_PX.min(1120.0 / (line.glyphs.len() as f32 * 1.04))
}

fn spawn_line(commands: &mut Commands, a: &Assets2, line: &Line, role: Role) -> Entity {
    let px = glyph_px(line);
    let adv = px * 1.04;
    let n = line.glyphs.len() as f32;
    let x_of = |i: usize| (i as f32 + 0.5 - n / 2.0) * adv;
    let (y, scale) = match role {
        Role::Current => (LINE_Y, 1.0),
        _ => (NEXT_Y - 40.0, NEXT_SCALE),
    };
    commands
        .spawn((
            LineRoot { role, alpha: 0.0, age: 0.0, y, scale },
            Transform::from_xyz(0.0, y, 0.0).with_scale(Vec3::splat(scale)),
            Visibility::default(),
        ))
        .with_children(|p| {
            for (i, g) in line.glyphs.iter().enumerate() {
                p.spawn((
                    Text2d::new(g.ch.to_string()),
                    font(&a.mincho, px),
                    Scaled(px),
                    TextColor(Color::NONE),
                    Slot(Vec2::new(x_of(i), 0.0)),
                    Lit { units: g.units.clone(), lit: 0.0, pop: 0.0, full: false, ruby: false },
                    Transform::default(),
                ));
            }
            let rpx = (px * 0.36).max(13.0);
            for r in &line.rubies {
                let cx = (x_of(r.glyphs.start) + x_of(r.glyphs.end - 1)) / 2.0;
                let rn = r.chars.len() as f32;
                for (j, c) in r.chars.iter().enumerate() {
                    let x = cx + (j as f32 + 0.5 - rn / 2.0) * rpx * 1.02;
                    p.spawn((
                        Text2d::new(c.ch.to_string()),
                        font(&a.mincho, rpx),
                        Scaled(rpx),
                        TextColor(Color::NONE),
                        Slot(Vec2::new(x, px * 0.80)),
                        Lit { units: c.units.clone(), lit: 0.0, pop: 0.0, full: false, ruby: true },
                        Transform::default(),
                    ));
                }
            }
        })
        .id()
}

// ---------------------------------------------------------------------------
// Input

fn letter(code: KeyCode) -> Option<char> {
    use KeyCode::*;
    let c = match code {
        KeyA => 'a', KeyB => 'b', KeyC => 'c', KeyD => 'd', KeyE => 'e', KeyF => 'f', KeyG => 'g',
        KeyH => 'h', KeyI => 'i', KeyJ => 'j', KeyK => 'k', KeyL => 'l', KeyM => 'm', KeyN => 'n',
        KeyO => 'o', KeyP => 'p', KeyQ => 'q', KeyR => 'r', KeyS => 's', KeyT => 't', KeyU => 'u',
        KeyV => 'v', KeyW => 'w', KeyX => 'x', KeyY => 'y', KeyZ => 'z',
        Minus | NumpadSubtract => '-',
        Quote => '\'',
        _ => return None,
    };
    Some(c)
}

#[allow(clippy::too_many_arguments)]
fn handle_input(
    mut commands: Commands,
    mut keys: MessageReader<KeyboardInput>,
    mut session: ResMut<Session>,
    mut audio: ResMut<Audio>,
    mut roots: Query<(Entity, &mut LineRoot)>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
    a: Option<Res<Assets2>>,
    view: Res<View>,
    time: Res<Time>,
) {
    let Some(a) = a else { return };
    let now = time.elapsed_secs();
    let s = &mut *session;

    if now - s.last_key < 3.0 {
        s.active_secs += time.delta_secs();
    }

    for ev in keys.read() {
        if ev.state != ButtonState::Pressed {
            continue;
        }
        match ev.key_code {
            KeyCode::Escape => {
                exit.write(AppExit::Success);
            }
            KeyCode::F11 => {
                window.mode = match window.mode {
                    WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    _ => WindowMode::Windowed,
                };
            }
            KeyCode::Tab => {
                s.hint = match s.hint {
                    HintMode::Full => HintMode::Syllable,
                    HintMode::Syllable => HintMode::Hidden,
                    HintMode::Hidden => HintMode::Full,
                };
                let label = match s.hint {
                    HintMode::Full => "ромадзи: вся фраза",
                    HintMode::Syllable => "ромадзи: только текущий слог",
                    HintMode::Hidden => "ромадзи: скрыто (появится, если задумаешься)",
                };
                s.toast = Some((label.into(), now));
            }
            KeyCode::F2 => {
                s.show_translation = !s.show_translation;
                let label = if s.show_translation { "перевод: показан" } else { "перевод: скрыт" };
                s.toast = Some((label.into(), now));
            }
            KeyCode::F3 => {
                audio.toggle_mute();
                let label = if audio.muted { "звук: выключен" } else { "звук: включён" };
                s.toast = Some((label.into(), now));
            }
            KeyCode::Enter | KeyCode::NumpadEnter if !ev.repeat => {
                advance(&mut commands, s, &mut roots, &a, &view, now, false);
            }
            code => {
                let Some(c) = letter(code) else { continue };
                if ev.repeat {
                    continue;
                }
                s.last_key = now;
                if c == 'n' && now < s.swallow_n_until {
                    s.swallow_n_until = 0.0;
                    audio.play(Cue::Key);
                    continue;
                }
                let out = s.matcher.input(c);
                if !out.accepted {
                    s.keys_miss += 1;
                    s.miss_at = now;
                    audio.play(Cue::Miss);
                    continue;
                }
                s.keys_ok += 1;
                if out.completed.is_empty() {
                    audio.play(Cue::Key);
                }
                for r in &out.completed {
                    s.units_done += r.len() as u32;
                    let unit = &s.matcher.units()[r.end - 1];
                    audio.play(Cue::Note(audio::degree_for(&kana::preferred(unit), r.start)));
                }
                if out.lone_n_at_end {
                    s.swallow_n_until = now + 0.8;
                }
                if s.matcher.is_done() {
                    audio.play(Cue::LineDone);
                    advance(&mut commands, s, &mut roots, &a, &view, now, true);
                }
            }
        }
    }
}

fn advance(
    commands: &mut Commands,
    s: &mut Session,
    roots: &mut Query<(Entity, &mut LineRoot)>,
    a: &Assets2,
    view: &View,
    now: f32,
    finished: bool,
) {
    if finished {
        s.lines_done += 1;
        // The ripple and the bowl mark the end of a phrase.
        commands.spawn((
            Ripple { life: 0.0 },
            Sprite { image: a.ring.clone(), color: Color::NONE, custom_size: Some(Vec2::ZERO), ..default() },
            Transform::from_xyz(0.0, LINE_Y * view.scale, -4.0),
        ));
    }
    for (e, mut root) in roots.iter_mut() {
        match root.role {
            Role::Current => {
                root.role = Role::Leaving;
                root.age = 0.0;
            }
            Role::Next => {
                root.role = Role::Current;
                root.age = 0.0;
            }
            Role::Leaving => commands.entity(e).despawn(),
        }
    }
    s.at += 1;
    s.matcher = Matcher::new(s.current().units.clone());
    s.line_started = now;
    s.swallow_n_until = if finished { s.swallow_n_until } else { 0.0 };
    spawn_line(commands, a, s.line_at(s.at + 1), Role::Next);
}

// ---------------------------------------------------------------------------
// Animation

fn smooth(rate: f32, dt: f32) -> f32 {
    1.0 - (-rate * dt).exp()
}

fn update_view(window: Single<&Window, With<PrimaryWindow>>, mut view: ResMut<View>) {
    let size = Vec2::new(window.width(), window.height());
    let scale = (size.x / DESIGN.x).min(size.y / DESIGN.y).max(0.4);
    // Quantize so that font atlases are not regenerated on every resize step.
    let scale = (scale * 20.0).round() / 20.0;
    if view.size != size {
        view.size = size;
    }
    if view.scale != scale {
        view.scale = scale;
    }
}

fn rescale_text(view: Res<View>, mut q: Query<(Ref<Scaled>, &mut TextFont)>) {
    let changed = view.is_changed();
    for (scaled, mut font) in &mut q {
        if changed || scaled.is_added() {
            font.font_size = FontSize::Px((scaled.0 * view.scale).round().max(6.0));
        }
    }
}

fn animate_lines(
    mut commands: Commands,
    mut q: Query<(Entity, &mut LineRoot, &mut Transform)>,
    view: Res<View>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (e, mut root, mut tf) in &mut q {
        root.age += dt;
        let (ty, ts, ta, rate) = match root.role {
            Role::Current => (LINE_Y, 1.0, 1.0, 7.0),
            Role::Next => (NEXT_Y, NEXT_SCALE, 0.22, 4.0),
            Role::Leaving => (LINE_Y + 230.0, 0.92, 0.0, 2.6),
        };
        let k = smooth(rate, dt);
        root.y += (ty - root.y) * k;
        root.scale += (ts - root.scale) * k;
        root.alpha += (ta - root.alpha) * smooth(rate * 0.8, dt);
        tf.translation = Vec3::new(0.0, root.y * view.scale, 0.0);
        tf.scale = Vec3::splat(root.scale);
        if root.role == Role::Leaving && root.age > 2.5 {
            commands.entity(e).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn animate_glyphs(
    mut commands: Commands,
    session: Res<Session>,
    roots: Query<&LineRoot>,
    mut q: Query<(&ChildOf, &Slot, &mut Lit, &mut Transform, &mut TextColor)>,
    a: Option<Res<Assets2>>,
    mut rng: ResMut<Rng>,
    view: Res<View>,
    time: Res<Time>,
) {
    let Some(a) = a else { return };
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let s = view.scale;
    let pos = session.matcher.pos();
    let miss = (-(now - session.miss_at) * 9.0).exp();
    let breath = 0.5 + 0.5 * (now * 2.6).sin();

    for (parent, slot, mut lit, mut tf, mut color) in &mut q {
        let Ok(root) = roots.get(parent.parent()) else { continue };
        let done = match root.role {
            Role::Current => pos,
            Role::Next => 0,
            Role::Leaving => usize::MAX,
        };
        let u = &lit.units;
        let progress = if u.is_empty() {
            (done >= u.start) as u8 as f32
        } else {
            ((done.saturating_sub(u.start)) as f32 / u.len() as f32).min(1.0)
        };
        let is_current = root.role == Role::Current && !u.is_empty() && u.start <= pos && pos < u.end;

        lit.lit += (progress - lit.lit) * smooth(22.0, dt);
        if progress >= 1.0 && !lit.full {
            lit.full = true;
            if root.role == Role::Current {
                lit.pop = 1.0;
                let origin = Vec2::new(slot.0.x * s, (root.y + slot.0.y) * s);
                let n = if lit.ruby { 3 } else { 7 };
                for _ in 0..n {
                    let ang = rng.range(0.15, std::f32::consts::PI - 0.15);
                    let speed = rng.range(40.0, 140.0) * s;
                    let max = rng.range(0.5, 1.1);
                    commands.spawn((
                        Spark { vel: Vec2::from_angle(ang) * speed, life: 0.0, max, size: rng.range(3.0, 7.0) * s },
                        Sprite { image: a.glow.clone(), color: Color::NONE, custom_size: Some(Vec2::ZERO), ..default() },
                        Transform::from_xyz(origin.x + rng.range(-12.0, 12.0) * s, origin.y, 1.0),
                    ));
                }
            }
        }
        lit.pop = (lit.pop - dt * 2.2).max(0.0);

        let mut c = DIM.lerp(GOLD, lit.lit);
        if is_current {
            c = c.lerp(CURRENT, 0.55 + 0.25 * breath);
            c = c.lerp(VERMILION, miss * 0.85);
        }
        c = c.lerp(GOLD_HOT, lit.pop * lit.pop);
        let alpha = root.alpha * if lit.ruby { 0.9 } else { 1.0 };
        color.0 = lin(c, alpha);

        let shake = if is_current { (now * 70.0).sin() * miss * 7.0 } else { 0.0 };
        let pop = lit.pop * lit.pop;
        tf.translation = Vec3::new((slot.0.x + shake) * s, (slot.0.y + pop * 7.0) * s, 0.0);
        tf.scale = Vec3::splat(1.0 + 0.16 * pop);
    }
}

/// Where the cursor should be: under the glyph(s) the next key belongs to.
fn cursor_target(session: &Session, s: f32) -> Option<Vec2> {
    let line = session.current();
    let pos = session.matcher.pos();
    let px = glyph_px(line);
    let adv = px * 1.04;
    let n = line.glyphs.len() as f32;
    let xs: Vec<f32> = line
        .glyphs
        .iter()
        .enumerate()
        .filter(|(_, g)| g.units.start <= pos && pos < g.units.end)
        .map(|(i, _)| (i as f32 + 0.5 - n / 2.0) * adv)
        .collect();
    if xs.is_empty() {
        return None;
    }
    let x = (xs[0] + xs[xs.len() - 1]) / 2.0;
    Some(Vec2::new(x, LINE_Y - px * 0.72) * s)
}

fn update_cursor(
    session: Res<Session>,
    mut q: Single<(&mut Cursor, &mut Transform, &mut Sprite)>,
    view: Res<View>,
    time: Res<Time>,
) {
    let (cursor, tf, sprite) = &mut *q;
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let target = cursor_target(&session, view.scale);
    if let Some(t) = target {
        // Snap on a fresh line, glide within one.
        if cursor.alpha < 0.05 {
            cursor.pos = t;
        }
        cursor.pos = cursor.pos.lerp(t, smooth(30.0, dt));
    }
    let ta = if target.is_some() && now - session.line_started > 0.12 { 1.0 } else { 0.0 };
    cursor.alpha += (ta - cursor.alpha) * smooth(10.0, dt);
    let miss = (-(now - session.miss_at) * 7.0).exp();
    let breath = 0.75 + 0.25 * (now * 3.2).sin();
    let c = Vec3::new(2.2, 1.2, 0.5).lerp(VERMILION, miss);
    sprite.color = lin(c, cursor.alpha * breath);
    sprite.custom_size = Some(Vec2::splat((11.0 + 3.0 * breath + miss * 6.0) * view.scale));
    tf.translation = cursor.pos.extend(0.5);
}

fn update_romaji(
    session: Res<Session>,
    root: Single<(&mut Transform, &Children), With<RomajiLine>>,
    mut spans: Query<(&RomajiSpan, &mut TextSpan, &mut TextColor)>,
    cursor: Single<&Cursor>,
    view: Res<View>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    let m = &session.matcher;
    let hint = m.current_hint().map(|c| c.romaji).unwrap_or_default();
    let buf = m.buffer().to_string();
    let rest_of_current = hint.get(buf.len()..).unwrap_or("").to_string();

    let hesitating = now - session.last_key.max(session.line_started) > 3.0;
    let missed = now - session.miss_at < 2.5;
    let (texts, alpha, follow_cursor): ([String; 4], f32, bool) = match session.hint {
        HintMode::Full => {
            let mut typed = m.typed().join(" ");
            if !typed.is_empty() {
                typed.push(' ');
            }
            let mut rest = m.preview_after_current().join(" ");
            if !rest.is_empty() {
                rest.insert(0, ' ');
            }
            ([typed, buf, rest_of_current, rest], 1.0, false)
        }
        HintMode::Syllable => ([String::new(), buf, rest_of_current, String::new()], 1.0, true),
        HintMode::Hidden => {
            let a = if hesitating || missed { 0.6 } else { 0.0 };
            ([String::new(), buf, rest_of_current, String::new()], a, true)
        }
    };

    let (mut tf, children) = root.into_inner();
    let x = if follow_cursor { cursor.pos.x } else { 0.0 };
    let target = Vec3::new(x, ROMAJI_Y * view.scale, 0.2);
    tf.translation = if follow_cursor { tf.translation.lerp(target, 0.5) } else { target };

    let colors = [
        lin(GOLD * 0.8, 0.55 * alpha),
        lin(GOLD_HOT * 0.7, alpha),
        lin(CURRENT, 0.95 * alpha),
        lin(DIM * 1.25, 0.8 * alpha),
    ];
    for &child in children {
        if let Ok((span, mut text, mut color)) = spans.get_mut(child) {
            if text.0 != texts[span.0] {
                text.0 = texts[span.0].clone();
            }
            color.set_if_neq(TextColor(colors[span.0]));
        }
    }
}

#[allow(clippy::type_complexity)]
fn update_overlay(
    mut session: ResMut<Session>,
    mut tr: Single<(&mut Translation, &mut Text2d, &mut TextColor, &mut Transform), (Without<Stats>, Without<Footer>, Without<Toast>)>,
    mut stats: Single<(&mut Stats, &mut Text2d, &mut TextColor, &mut Transform), (Without<Footer>, Without<Toast>)>,
    mut footer: Single<(&mut Footer, &mut TextColor, &mut Transform), Without<Toast>>,
    mut toast: Single<(&mut Text2d, &mut TextColor, &mut Transform), (With<Toast>, Without<Stats>, Without<Translation>)>,
    view: Res<View>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let s = view.scale;
    let idle = now - session.last_key > 2.5;

    // Translation fades out and back in when the phrase changes.
    let (t, text, color, tf) = &mut *tr;
    if t.for_at != session.at {
        t.alpha -= dt * 6.0;
        if t.alpha <= 0.0 {
            t.alpha = 0.0;
            t.for_at = session.at;
            text.0 = session.current().translation.clone();
        }
    } else {
        let target = if session.show_translation { 1.0 } else { 0.0 };
        t.alpha += (target - t.alpha) * smooth(3.0, dt);
    }
    color.0 = lin(Vec3::new(0.62, 0.58, 0.54), t.alpha * 0.85);
    tf.translation = Vec3::new(0.0, TRANSLATION_Y * s, 0.0);

    let (st, text, color, tf) = &mut *stats;
    let minutes = session.active_secs / 60.0;
    let total = session.keys_ok + session.keys_miss;
    let acc = if total == 0 { 100.0 } else { session.keys_ok as f32 * 100.0 / total as f32 };
    let kpm = if minutes > 0.05 { session.units_done as f32 / minutes } else { 0.0 };
    let line = format!("{kpm:.0} кана/мин   ·   точность {acc:.0}%   ·   фраз {}", session.lines_done);
    if text.0 != line {
        text.0 = line;
    }
    st.0 += ((if idle { 0.5 } else { 0.16 }) - st.0) * smooth(2.0, dt);
    color.0 = lin(Vec3::new(0.7, 0.68, 0.66), st.0);
    tf.translation = Vec3::new(0.0, view.size.y / 2.0 - 34.0 * s, 0.0);

    let (f, color, tf) = &mut *footer;
    f.0 += ((if idle { 0.42 } else { 0.0 }) - f.0) * smooth(if idle { 1.2 } else { 6.0 }, dt);
    color.0 = lin(Vec3::new(0.6, 0.6, 0.62), f.0);
    tf.translation = Vec3::new(0.0, -view.size.y / 2.0 + 28.0 * s, 0.0);

    let (text, color, tf) = &mut *toast;
    let mut a = 0.0;
    if let Some((msg, at)) = &session.toast {
        let age = now - at;
        if text.0 != *msg {
            text.0 = msg.clone();
        }
        a = (age * 8.0).min(1.0) * (1.0 - ((age - 1.6) * 2.0).clamp(0.0, 1.0));
        if age > 2.2 {
            session.toast = None;
        }
    }
    color.0 = lin(Vec3::new(0.95, 0.85, 0.7), a * 0.85);
    tf.translation = Vec3::new(0.0, -view.size.y / 2.0 + 70.0 * s, 0.0);
}

fn update_motes(mut q: Query<(&mut Mote, &mut Transform, &mut Sprite)>, view: Res<View>, time: Res<Time>) {
    let dt = time.delta_secs();
    let now = time.elapsed_secs();
    let half = view.size / 2.0;
    for (mut m, mut tf, mut sprite) in &mut q {
        m.pos.y += m.speed * dt;
        if m.pos.y > 1.05 {
            m.pos.y -= 1.1;
        }
        let sway = (now * 0.35 + m.phase).sin() * 0.012;
        let x = (m.pos.x + sway) * view.size.x - half.x;
        let y = m.pos.y * view.size.y - half.y;
        tf.translation = Vec3::new(x, y, -5.0);
        let twinkle = 0.7 + 0.3 * (now * 0.9 + m.phase * 3.0).sin();
        sprite.color = lin(Vec3::new(1.0, 0.72, 0.5), m.alpha * twinkle);
        sprite.custom_size = Some(Vec2::splat(m.size * view.scale));
    }
}

fn update_sparks(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Spark, &mut Transform, &mut Sprite)>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    for (e, mut sp, mut tf, mut sprite) in &mut q {
        sp.life += dt;
        if sp.life >= sp.max {
            commands.entity(e).despawn();
            continue;
        }
        sp.vel *= 1.0 - smooth(3.0, dt);
        sp.vel.y += 25.0 * dt;
        tf.translation += (sp.vel * dt).extend(0.0);
        let k = 1.0 - sp.life / sp.max;
        sprite.color = lin(GOLD_HOT * (0.4 + k), k * k);
        sprite.custom_size = Some(Vec2::splat(sp.size * (0.5 + 0.5 * k)));
    }
}

fn update_ripples(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Ripple, &mut Sprite)>,
    view: Res<View>,
    time: Res<Time>,
) {
    for (e, mut r, mut sprite) in &mut q {
        r.life += time.delta_secs();
        let t = r.life / 2.4;
        if t >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        let ease = 1.0 - (1.0 - t).powi(3);
        let size = (200.0 + 1300.0 * ease) * view.scale;
        sprite.custom_size = Some(Vec2::new(size, size * 0.3));
        sprite.color = lin(Vec3::new(1.4, 0.95, 0.55), 0.08 * (1.0 - t).powi(2));
    }
}

fn update_background(mut q: Single<&mut Sprite, With<Background>>, view: Res<View>) {
    q.custom_size = Some(view.size);
}

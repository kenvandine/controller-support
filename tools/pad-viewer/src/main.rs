//! pad-viewer: draws a white gamepad and lights up every control live, using
//! SDL3's gamepad API (the same path most games and Steam's runtime use).
//!
//!   pad-viewer [--no-hidapi] [--screenshot FILE.bmp]
//!
//! Keys: R rumble, T trigger rumble, Esc/Q quit.
//!
//! --no-hidapi sets SDL_JOYSTICK_HIDAPI=0 so SDL uses the kernel's evdev
//! device even when it could open hidraw. --screenshot renders one frame
//! with a few simulated inputs to a BMP and exits (works headless with
//! SDL_VIDEO_DRIVER=offscreen).

use sdl3::event::Event;
use sdl3::gamepad::{Axis, Button, Gamepad};
use sdl3::keyboard::Keycode;
use sdl3::pixels::{Color, FColor};
use sdl3::render::{FPoint, Vertex, WindowCanvas};
use sdl3::sys;
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::time::{Duration, Instant};

const W: u32 = 1000;
const H: u32 = 760;

const BG: Color = Color::RGB(0x1d, 0x21, 0x26);
const BODY: Color = Color::RGB(0xf3, 0xf4, 0xf1);
const BODY_EDGE: Color = Color::RGB(0xb4, 0xb8, 0xbc);
const BODY_SHADE: Color = Color::RGB(0xe2, 0xe4, 0xe0);
const RECESS: Color = Color::RGB(0x3a, 0x3f, 0x45);
const CAP: Color = Color::RGB(0xe9, 0xea, 0xe7);
const DARK_BTN: Color = Color::RGB(0x2b, 0x2f, 0x34);
const ACCENT: Color = Color::RGB(0x2f, 0x9b, 0xff);
const TEXT: Color = Color::RGB(0xdd, 0xe1, 0xe6);
const DIM: Color = Color::RGB(0x80, 0x88, 0x91);

const A_COL: Color = Color::RGB(0x4c, 0xc2, 0x4f);
const B_COL: Color = Color::RGB(0xe5, 0x3e, 0x3e);
const X_COL: Color = Color::RGB(0x2f, 0x7f, 0xe8);
const Y_COL: Color = Color::RGB(0xf0, 0xc0, 0x2c);

/// Everything the drawing code needs for one frame.
#[derive(Default, Clone)]
struct PadState {
    buttons: Vec<Button>,
    lx: f32,
    ly: f32,
    rx: f32,
    ry: f32,
    lt: f32,
    rt: f32,
    raw_lt: i16,
    raw_rt: i16,
    raw_buttons: Vec<bool>,
    raw_axes: Vec<i16>,
    raw_hats: Vec<u8>,
}

impl PadState {
    fn down(&self, b: Button) -> bool {
        self.buttons.contains(&b)
    }

    fn read(pad: &Gamepad) -> PadState {
        let all = [
            Button::South,
            Button::East,
            Button::West,
            Button::North,
            Button::Back,
            Button::Guide,
            Button::Start,
            Button::LeftStick,
            Button::RightStick,
            Button::LeftShoulder,
            Button::RightShoulder,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
            Button::Misc1,
            Button::RightPaddle1,
            Button::LeftPaddle1,
            Button::RightPaddle2,
            Button::LeftPaddle2,
            Button::Touchpad,
        ];
        let norm = |v: i16| (v as f32 / 32767.0).clamp(-1.0, 1.0);
        let mut s = PadState {
            buttons: all.into_iter().filter(|b| pad.button(*b)).collect(),
            lx: norm(pad.axis(Axis::LeftX)),
            ly: norm(pad.axis(Axis::LeftY)),
            rx: norm(pad.axis(Axis::RightX)),
            ry: norm(pad.axis(Axis::RightY)),
            raw_lt: pad.axis(Axis::TriggerLeft),
            raw_rt: pad.axis(Axis::TriggerRight),
            ..Default::default()
        };
        s.lt = norm(s.raw_lt).max(0.0);
        s.rt = norm(s.raw_rt).max(0.0);

        // The underlying joystick, to show SDL's raw button/axis numbering
        // (what a mapping string's b<N>/a<N> refer to).
        unsafe {
            let js = sys::gamepad::SDL_GetGamepadJoystick(pad.raw());
            if !js.is_null() {
                let nb = sys::joystick::SDL_GetNumJoystickButtons(js).max(0);
                s.raw_buttons = (0..nb)
                    .map(|i| sys::joystick::SDL_GetJoystickButton(js, i))
                    .collect();
                let na = sys::joystick::SDL_GetNumJoystickAxes(js).max(0);
                s.raw_axes = (0..na)
                    .map(|i| sys::joystick::SDL_GetJoystickAxis(js, i))
                    .collect();
                let nh = sys::joystick::SDL_GetNumJoystickHats(js).max(0);
                s.raw_hats = (0..nh)
                    .map(|i| sys::joystick::SDL_GetJoystickHat(js, i))
                    .collect();
            }
        }
        s
    }

    /// A fixed state for --screenshot.
    fn demo() -> PadState {
        PadState {
            buttons: vec![Button::South, Button::Back, Button::DPadRight, Button::LeftShoulder],
            lx: -0.6,
            ly: -0.5,
            rx: 0.3,
            ry: 0.7,
            lt: 0.35,
            rt: 1.0,
            raw_lt: 11468,
            raw_rt: 32767,
            raw_buttons: (0..17).map(|i| i == 0 || i == 10 || i == 15).collect(),
            raw_axes: vec![-19660, -16383, 9830, 22937, 11468, 32767],
            raw_hats: vec![2],
        }
    }
}

/// Static facts about the open gamepad, shown in the info panel.
#[derive(Default, Clone)]
struct PadInfo {
    name: String,
    ids: String,
    path: String,
    kind: String,
    mapping: String,
    power: String,
}

impl PadInfo {
    fn read(pad: &Gamepad) -> PadInfo {
        let power = pad.power_info();
        let path = pad.path().unwrap_or_default();
        let kind = if path.contains("hidraw") {
            "HIDAPI (hidraw)".to_string()
        } else if path.contains("/dev/input/event") {
            "evdev (kernel driver mapping)".to_string()
        } else if path.is_empty() {
            "unknown".to_string()
        } else {
            path.clone()
        };
        PadInfo {
            name: pad.name().unwrap_or_else(|| "(unnamed)".into()),
            ids: format!(
                "{:04x}:{:04x}",
                pad.vendor_id().unwrap_or(0),
                pad.product_id().unwrap_or(0)
            ),
            path,
            kind,
            mapping: pad.mapping().unwrap_or_default(),
            power: if power.percentage >= 0 {
                format!("{:?}, {}%", power.state, power.percentage)
            } else {
                format!("{:?}", power.state)
            },
        }
    }
}

/// Collects triangles so a whole frame of shapes is one SDL_RenderGeometry.
struct Painter {
    verts: Vec<Vertex>,
    idx: Vec<i32>,
}

impl Painter {
    fn new() -> Self {
        Painter { verts: Vec::new(), idx: Vec::new() }
    }

    /// Fill a convex polygon (fan from its centroid).
    fn poly(&mut self, pts: &[(f32, f32)], c: Color) {
        if pts.len() < 3 {
            return;
        }
        let col = FColor::from(c);
        let n = pts.len() as f32;
        let cx = pts.iter().map(|p| p.0).sum::<f32>() / n;
        let cy = pts.iter().map(|p| p.1).sum::<f32>() / n;
        let base = self.verts.len() as i32;
        let v = |x: f32, y: f32| Vertex {
            position: FPoint::new(x, y),
            color: col,
            tex_coord: FPoint::new(0.0, 0.0),
        };
        self.verts.push(v(cx, cy));
        for p in pts {
            self.verts.push(v(p.0, p.1));
        }
        let m = pts.len() as i32;
        for i in 0..m {
            self.idx.extend([base, base + 1 + i, base + 1 + (i + 1) % m]);
        }
    }

    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, c: Color) {
        self.poly(&ellipse_pts(cx, cy, rx, ry, rot), c);
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Color) {
        self.ellipse(cx, cy, r, r, 0.0, c);
    }

    fn rrect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
        self.poly(&rrect_pts(x, y, w, h, r), c);
    }

    fn flush(&mut self, canvas: &mut WindowCanvas) {
        if !self.idx.is_empty() {
            let _ = canvas.render_geometry(&self.verts, None, &self.idx[..]);
        }
        self.verts.clear();
        self.idx.clear();
    }
}

fn ellipse_pts(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32) -> Vec<(f32, f32)> {
    let n = 48;
    let (s, c) = rot.sin_cos();
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * 2.0 * PI;
            let (x, y) = (rx * t.cos(), ry * t.sin());
            (cx + x * c - y * s, cy + x * s + y * c)
        })
        .collect()
}

fn rrect_pts(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<(f32, f32)> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let corners = [
        (x + w - r, y + r, -PI / 2.0),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, PI / 2.0),
        (x + r, y + r, PI),
    ];
    let mut pts = Vec::new();
    for (cx, cy, a0) in corners {
        for i in 0..=8 {
            let a = a0 + i as f32 / 8.0 * PI / 2.0;
            pts.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    pts
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::RGB(l(a.r, b.r), l(a.g, b.g), l(a.b, b.b))
}

/// Text in SDL's built-in 8x8 debug font, at an integer scale.
fn text(canvas: &mut WindowCanvas, s: &str, x: f32, y: f32, scale: f32, c: Color) {
    canvas.set_draw_color(c);
    let _ = canvas.set_scale(scale, scale);
    let _ = canvas.draw_debug_text(s, FPoint::new(x / scale, y / scale));
    let _ = canvas.set_scale(1.0, 1.0);
}

fn text_centered(canvas: &mut WindowCanvas, s: &str, cx: f32, cy: f32, scale: f32, c: Color) {
    let w = s.len() as f32 * 8.0 * scale;
    text(canvas, s, cx - w / 2.0, cy - 4.0 * scale, scale, c);
}

// Layout of the controller (logical 1000x760 canvas).
const LS: (f32, f32) = (338.0, 262.0);
const RS: (f32, f32) = (590.0, 352.0);
const DPAD: (f32, f32) = (420.0, 352.0);
const FACE: (f32, f32) = (668.0, 262.0);
const GUIDE: (f32, f32) = (500.0, 196.0);

fn draw_pad(canvas: &mut WindowCanvas, s: &PadState, connected: bool) {
    let mut p = Painter::new();
    let on = |b: Button| s.down(b);

    // Triggers: gauges above the bumpers, filled by how far they're pulled.
    for (x, v, raw, label) in [(292.0, s.lt, s.raw_lt, "LT"), (598.0, s.rt, s.raw_rt, "RT")] {
        p.rrect(x - 2.0, 40.0, 114.0, 84.0, 14.0, BODY_EDGE);
        p.rrect(x, 42.0, 110.0, 80.0, 12.0, BODY_SHADE);
        if v > 0.0 {
            let h = 76.0 * v;
            p.rrect(x + 2.0, 44.0 + 76.0 - h, 106.0, h, 10.0_f32.min(h / 2.0), ACCENT);
        }
        let _ = (raw, label);
    }

    // Bumpers: behind the top edge of the body.
    for (x, b) in [(262.0, Button::LeftShoulder), (568.0, Button::RightShoulder)] {
        p.rrect(x - 2.0, 126.0, 174.0, 56.0, 24.0, BODY_EDGE);
        p.rrect(x, 128.0, 170.0, 52.0, 22.0, if on(b) { ACCENT } else { BODY_SHADE });
    }

    // Body: grips + centre, outline first, then the white fill on top.
    let shapes = |p: &mut Painter, grow: f32, c: Color| {
        p.ellipse(318.0, 410.0, 92.0 + grow, 158.0 + grow, 0.40, c);
        p.ellipse(682.0, 410.0, 92.0 + grow, 158.0 + grow, -0.40, c);
        p.rrect(268.0 - grow, 160.0 - grow, 464.0 + 2.0 * grow, 250.0 + 2.0 * grow, 90.0, c);
        p.ellipse(500.0, 405.0, 170.0 + grow, 50.0 + grow, 0.0, c);
    };
    shapes(&mut p, 3.0, BODY_EDGE);
    shapes(&mut p, 0.0, BODY);

    // Guide button.
    p.circle(GUIDE.0, GUIDE.1, 27.0, BODY_EDGE);
    p.circle(GUIDE.0, GUIDE.1, 24.0, if on(Button::Guide) { ACCENT } else { CAP });
    p.circle(GUIDE.0, GUIDE.1, 15.0, if on(Button::Guide) { mix(ACCENT, BODY, 0.4) } else { BODY_SHADE });

    // View, Menu, Share.
    for (x, y, b) in [(452.0, 262.0, Button::Back), (548.0, 262.0, Button::Start)] {
        p.circle(x, y, 14.0, BODY_EDGE);
        p.circle(x, y, 12.0, if on(b) { ACCENT } else { DARK_BTN });
    }
    p.rrect(487.0, 296.0, 26.0, 14.0, 6.0, BODY_EDGE);
    p.rrect(488.0, 297.0, 24.0, 12.0, 5.0, if on(Button::Misc1) { ACCENT } else { DARK_BTN });

    // Sticks.
    for ((cx, cy), x, y, b) in [(LS, s.lx, s.ly, Button::LeftStick), (RS, s.rx, s.ry, Button::RightStick)] {
        p.circle(cx, cy, 50.0, BODY_EDGE);
        p.circle(cx, cy, 47.0, RECESS);
        let (kx, ky) = (cx + x * 22.0, cy + y * 22.0);
        p.circle(kx, ky + 3.0, 34.0, mix(RECESS, BG, 0.5));
        p.circle(kx, ky, 34.0, if on(b) { ACCENT } else { CAP });
        p.circle(kx, ky, 26.0, if on(b) { mix(ACCENT, BODY, 0.3) } else { BODY_SHADE });
    }

    // D-pad.
    let (dx, dy) = DPAD;
    p.circle(dx, dy, 46.0, BODY_SHADE);
    p.rrect(dx - 15.0, dy - 41.0, 30.0, 82.0, 5.0, DARK_BTN);
    p.rrect(dx - 41.0, dy - 15.0, 82.0, 30.0, 5.0, DARK_BTN);
    for (b, x, y, w, h) in [
        (Button::DPadUp, dx - 13.0, dy - 39.0, 26.0, 26.0),
        (Button::DPadDown, dx - 13.0, dy + 13.0, 26.0, 26.0),
        (Button::DPadLeft, dx - 39.0, dy - 13.0, 26.0, 26.0),
        (Button::DPadRight, dx + 13.0, dy - 13.0, 26.0, 26.0),
    ] {
        if on(b) {
            p.rrect(x, y, w, h, 4.0, ACCENT);
        }
    }

    // A B X Y.
    let face = [
        (Button::South, 0.0, 38.0, A_COL),
        (Button::East, 38.0, 0.0, B_COL),
        (Button::West, -38.0, 0.0, X_COL),
        (Button::North, 0.0, -38.0, Y_COL),
    ];
    for (b, ox, oy, c) in face {
        let (x, y) = (FACE.0 + ox, FACE.1 + oy);
        p.circle(x, y + 2.0, 21.0, BODY_EDGE);
        p.circle(x, y, 20.0, if on(b) { c } else { DARK_BTN });
    }

    // Back paddles (Elite): small tabs under the grips, only lit when used.
    for (x, b) in [
        (318.0, Button::LeftPaddle1),
        (350.0, Button::LeftPaddle2),
        (650.0, Button::RightPaddle2),
        (682.0, Button::RightPaddle1),
    ] {
        p.rrect(x - 12.0, 572.0, 24.0, 30.0, 8.0, if on(b) { ACCENT } else { mix(BG, RECESS, 0.5) });
    }

    p.flush(canvas);

    // Labels drawn over the shapes.
    for ((x, y), name, c, b) in [
        ((FACE.0, FACE.1 + 38.0), "A", A_COL, Button::South),
        ((FACE.0 + 38.0, FACE.1), "B", B_COL, Button::East),
        ((FACE.0 - 38.0, FACE.1), "X", X_COL, Button::West),
        ((FACE.0, FACE.1 - 38.0), "Y", Y_COL, Button::North),
    ] {
        let fg = if on(b) { Color::RGB(0x10, 0x10, 0x10) } else { c };
        text_centered(canvas, name, x, y + 1.0, 2.0, fg);
    }
    for (x, label, v, raw) in [(292.0, "LT", s.lt, s.raw_lt), (598.0, "RT", s.rt, s.raw_rt)] {
        let fg = if v > 0.45 { Color::WHITE } else { RECESS };
        text_centered(canvas, label, x + 55.0, 72.0, 2.0, fg);
        text_centered(canvas, &raw.to_string(), x + 55.0, 98.0, 1.0, fg);
    }
    let lbl = |on: bool| if on { Color::WHITE } else { RECESS };
    text_centered(canvas, "LB", 347.0, 145.0, 2.0, lbl(on(Button::LeftShoulder)));
    text_centered(canvas, "RB", 653.0, 145.0, 2.0, lbl(on(Button::RightShoulder)));
    text_centered(canvas, "View", 452.0, 288.0, 1.0, if on(Button::Back) { ACCENT } else { DIM });
    text_centered(canvas, "Menu", 548.0, 288.0, 1.0, if on(Button::Start) { ACCENT } else { DIM });
    text_centered(canvas, "Share", 500.0, 320.0, 1.0, if on(Button::Misc1) { ACCENT } else { DIM });
    text_centered(canvas, "Guide", GUIDE.0, GUIDE.1 - 36.0, 1.0, if on(Button::Guide) { ACCENT } else { DIM });
    for ((cx, cy), x, y) in [(LS, s.lx, s.ly), (RS, s.rx, s.ry)] {
        text_centered(canvas, &format!("{:+.2} {:+.2}", x, y), cx, cy + 64.0, 1.0, RECESS);
    }
    text_centered(canvas, "Paddles", 335.0, 614.0, 1.0, DIM);
    text_centered(canvas, "Paddles", 666.0, 614.0, 1.0, DIM);

    if !connected {
        text_centered(canvas, "No gamepad: connect or pair one", 500.0, 420.0, 2.0, Color::RGB(0xc0, 0x40, 0x40));
    }
}

fn draw_panel(canvas: &mut WindowCanvas, s: &PadState, info: &PadInfo, log: &VecDeque<String>, status: &str) {
    let x0 = 24.0;
    let mut y = 636.0;
    text(canvas, &format!("{}  [{}]", info.name, info.ids), x0, y, 1.0, TEXT);
    y += 14.0;
    text(canvas, &format!("SDL path: {}  ->  {}", info.kind, info.path), x0, y, 1.0, DIM);
    y += 14.0;
    text(canvas, &format!("Battery: {}", info.power), x0, y, 1.0, DIM);
    y += 18.0;

    // Raw joystick buttons: the b<N> numbers a mapping string uses.
    text(canvas, "raw b:", x0, y + 4.0, 1.0, DIM);
    let mut p = Painter::new();
    for (i, d) in s.raw_buttons.iter().enumerate() {
        let x = x0 + 56.0 + i as f32 * 26.0;
        p.rrect(x, y, 22.0, 16.0, 3.0, if *d { ACCENT } else { RECESS });
    }
    p.flush(canvas);
    for i in 0..s.raw_buttons.len() {
        let x = x0 + 56.0 + i as f32 * 26.0;
        text_centered(canvas, &i.to_string(), x + 11.0, y + 8.0, 1.0, TEXT);
    }
    y += 22.0;
    let axes: Vec<String> = s.raw_axes.iter().enumerate().map(|(i, v)| format!("a{i}={v}")).collect();
    let hats: Vec<String> = s.raw_hats.iter().enumerate().map(|(i, v)| format!("h{i}={v}")).collect();
    text(canvas, &format!("raw {} {}", axes.join(" "), hats.join(" ")), x0, y, 1.0, DIM);
    y += 14.0;
    // Mapping string, wrapped.
    let m = if info.mapping.is_empty() { "(no mapping)".to_string() } else { info.mapping.clone() };
    for chunk in m.as_bytes().chunks(118).take(2) {
        text(canvas, &String::from_utf8_lossy(chunk), x0, y, 1.0, DIM);
        y += 11.0;
    }

    // Recent events, top right.
    let lx = 790.0;
    text(canvas, "Events", lx, 150.0, 1.0, TEXT);
    for (i, e) in log.iter().enumerate() {
        text(canvas, e, lx, 166.0 + i as f32 * 12.0, 1.0, DIM);
    }
    text(canvas, "R rumble  T trigger rumble  Q quit", 24.0, 16.0, 1.0, DIM);
    text(canvas, status, 24.0, H as f32 - 14.0, 1.0, ACCENT);
}

fn frame(canvas: &mut WindowCanvas, s: &PadState, info: &PadInfo, log: &VecDeque<String>, status: &str, connected: bool) {
    canvas.set_draw_color(BG);
    canvas.clear();
    draw_pad(canvas, s, connected);
    draw_panel(canvas, s, info, log, status);
}

fn push_log(log: &mut VecDeque<String>, msg: String) {
    log.push_front(msg);
    log.truncate(30);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut screenshot: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--no-hidapi" => {
                sdl3::hint::set("SDL_JOYSTICK_HIDAPI", "0");
            }
            "--screenshot" => screenshot = it.next().cloned(),
            "-h" | "--help" => {
                println!("usage: pad-viewer [--no-hidapi] [--screenshot FILE.bmp]");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }

    let sdl = sdl3::init()?;
    let video = sdl.video()?;
    let gamepads = sdl.gamepad()?;
    let window = video
        .window("pad-viewer", W, H)
        .resizable()
        .high_pixel_density()
        .build()?;
    let mut canvas = window.into_canvas();
    canvas.set_logical_size(W, H, sys::render::SDL_LOGICAL_PRESENTATION_LETTERBOX)?;

    if let Some(path) = screenshot {
        let info = PadInfo {
            name: "Demo controller (simulated input)".into(),
            ids: "045e:0b20".into(),
            path: "/dev/input/event20".into(),
            kind: "evdev (kernel driver mapping)".into(),
            mapping: "050000005e040000200b000023050000,Xbox Wireless Controller,a:b0,b:b1,back:b10,...".into(),
            power: "Unknown".into(),
        };
        let mut log = VecDeque::new();
        for e in ["Back up", "Back down", "South down", "connected: demo"] {
            log.push_back(e.to_string());
        }
        frame(&mut canvas, &PadState::demo(), &info, &log, "screenshot", true);
        let surf = canvas.read_pixels(None)?;
        surf.save_bmp(&path)?;
        println!("wrote {path}");
        return Ok(());
    }

    let mut events = sdl.event_pump()?;
    let mut pad: Option<Gamepad> = None;
    let mut info = PadInfo::default();
    let mut log: VecDeque<String> = VecDeque::new();
    let mut status = String::new();
    let mut status_at = Instant::now();

    'main: loop {
        for ev in events.poll_iter() {
            match ev {
                Event::Quit { .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                | Event::KeyDown { keycode: Some(Keycode::Q), .. } => break 'main,
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    if let Some(p) = pad.as_mut() {
                        status = match p.set_rumble(0xC000, 0x8000, 500) {
                            Ok(()) => "rumble: both motors, 0.5 s".into(),
                            Err(e) => format!("rumble failed: {e}"),
                        };
                        status_at = Instant::now();
                    }
                }
                Event::KeyDown { keycode: Some(Keycode::T), .. } => {
                    if let Some(p) = pad.as_mut() {
                        status = match p.set_rumble_triggers(0xC000, 0xC000, 500) {
                            Ok(()) => "trigger rumble: 0.5 s".into(),
                            Err(e) => format!("trigger rumble not available: {e}"),
                        };
                        status_at = Instant::now();
                    }
                }
                Event::GamepadAdded { which, .. } => {
                    if pad.is_none() {
                        match gamepads.open(which) {
                            Ok(p) => {
                                info = PadInfo::read(&p);
                                push_log(&mut log, format!("connected: {}", info.name));
                                println!("connected: {} [{}] via {} ({})", info.name, info.ids, info.kind, info.path);
                                println!("mapping: {}", info.mapping);
                                pad = Some(p);
                            }
                            Err(e) => push_log(&mut log, format!("open failed: {e}")),
                        }
                    }
                }
                Event::GamepadRemoved { which, .. } => {
                    if pad.as_ref().and_then(|p| p.id().ok()) == Some(which) {
                        push_log(&mut log, "disconnected".into());
                        pad = None;
                        info = PadInfo::default();
                    }
                }
                Event::GamepadButtonDown { button, .. } => push_log(&mut log, format!("{button:?} down")),
                Event::GamepadButtonUp { button, .. } => push_log(&mut log, format!("{button:?} up")),
                Event::JoyButtonDown { button_idx, .. } => push_log(&mut log, format!("  raw b{button_idx} down")),
                Event::JoyButtonUp { button_idx, .. } => push_log(&mut log, format!("  raw b{button_idx} up")),
                _ => {}
            }
        }

        if status_at.elapsed() > Duration::from_secs(4) {
            status.clear();
        }
        if let Some(p) = pad.as_ref() {
            // Battery can change; refresh it now and then.
            if status_at.elapsed().as_millis() % 2000 < 20 {
                info.power = PadInfo::read(p).power;
            }
        }
        let state = pad.as_ref().map(PadState::read).unwrap_or_default();
        frame(&mut canvas, &state, &info, &log, &status, pad.is_some());
        canvas.present();
        std::thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}

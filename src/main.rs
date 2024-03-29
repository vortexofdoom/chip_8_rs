use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::time::Duration;

use display::Display;
use rand::Rng;
use sdl2::EventPump;
use sdl2::audio::{AudioCallback, AudioSpecDesired, AudioDevice, AudioStatus};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{Texture, Canvas};
use sdl2::video::Window;
use sdl2::pixels::PixelFormatEnum;

pub mod args;
pub mod display;

pub const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[derive(Debug, Default)]
pub struct Chip8 {
    display: Display,
    input: Option<u8>,
    memory: Vec<u8>,
    pc: u16,
    i: u16,
    stack: Vec<u16>,
    v: [u8; 16],
    dt: u8,
    st: u8,
}

trait Nibbles {
    fn x(&self) -> usize;
    fn y(&self) -> usize;
    fn n(&self) -> u8;
    fn nn(&self) -> u8;
    fn nnn(&self) -> u16;
}

impl Nibbles for u16 {
    /// Returns the second-most significant 4 bits (0000_XXXX_0000_0000)
    /// This is always used as a memory index 
    fn x(&self) -> usize {
        (self >> 8) as usize & 0xF
    }
    
    /// Returns the second-least significant 4 bits (0000_0000_XXXX_0000)
    /// This is always used as a memory index 
    fn y(&self) -> usize {
        (self >> 4) as usize & 0xF
    }

    /// Returns the least significant 4 bits (0000_0000_0000_XXXX)
    fn n(&self) -> u8 {
        *self as u8 & 0xF
    }

    /// Returns the lower byte
    fn nn(&self) -> u8 {
        *self as u8
    }

    /// Returns the lowest 12 bits
    fn nnn(&self) -> u16 {
        self & 0xFFF
    }
}

impl Chip8 {
    pub fn new(path: &str) -> Self {
        let mut memory = vec![0; 512];
        for i in 0..80 {
            memory[0x50 + i] = FONT[i];
        }
        let file = File::open(Path::new(path)).expect("failed to open");
        let mut buf = vec![];
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut buf).expect("failed to read file");
        memory.append(&mut buf);
        memory.resize(4096, 0);
        Self { memory, pc: 0x200, ..Default::default() }
    }

    pub fn render(&mut self, texture: &mut Texture, canvas: &mut Canvas<Window>) {
        if self.display.changed() {
            self.display.render(texture, canvas);
        }
    }

    pub fn beep(&mut self, audio_device: &AudioDevice<SquareWave>) {
        match (self.st > 0, audio_device.status()) {
            (true, AudioStatus::Paused) => audio_device.resume(),
            (false, AudioStatus::Playing) => audio_device.pause(),
            _ => {/*Do nothing*/}
        }
    }

    pub fn get_input(&mut self, event_pump: &mut EventPump) {
        if let Some(event) = event_pump.poll_event() {
            use Keycode::*;
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Escape), .. } => std::process::exit(0),
                Event::KeyDown { keycode: Some(Num1), .. } => self.input = Some(0x1),
                Event::KeyDown { keycode: Some(Num2), .. } => self.input = Some(0x2),
                Event::KeyDown { keycode: Some(Num3), .. } => self.input = Some(0x3),
                Event::KeyDown { keycode: Some(Num4), .. } => self.input = Some(0xC),
                Event::KeyDown { keycode: Some(Q), .. } => self.input = Some(0x4),
                Event::KeyDown { keycode: Some(W), .. } => self.input = Some(0x5),
                Event::KeyDown { keycode: Some(E), .. } => self.input = Some(0x6), 
                Event::KeyDown { keycode: Some(R), .. } => self.input = Some(0xD),
                Event::KeyDown { keycode: Some(A), .. } => self.input = Some(0x7),
                Event::KeyDown { keycode: Some(S), .. } => self.input = Some(0x8),
                Event::KeyDown { keycode: Some(D), .. } => self.input = Some(0x9),
                Event::KeyDown { keycode: Some(F), .. } => self.input = Some(0xE),
                Event::KeyDown { keycode: Some(Z), .. } => self.input = Some(0xA),
                Event::KeyDown { keycode: Some(X), .. } => self.input = Some(0x0),
                Event::KeyDown { keycode: Some(C), .. } => self.input = Some(0xB),
                Event::KeyDown { keycode: Some(V), .. } => self.input = Some(0xF),
                _ => self.input = None,
            }
        } else {
            self.input = None;
        }
    }

    fn fetch(&mut self) -> u16 {
        let i = self.pc as usize;
        self.pc += 2;
        u16::from_be_bytes([self.memory[i], self.memory[i + 1]])
    }

    fn tick(&mut self) {
        let instruction = self.fetch();
        self.decode(instruction);
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        }
    }

    fn decode(&mut self, instruction: u16) {
        match instruction >> 12 {
            0x0 => match instruction.nnn() {
                    0x0E0 => self.display.clear(),
                    0x0EE => { self.pc = self.stack.pop().expect("stack is empty") },
                    // SuperChip instructions
                    0x0FF => { /*enable 128x64 graphics*/ }
                    0x0FE => { /*disable 128x64 graphics*/ }
                    _n @ 0x0C0..=0x0CF => self.display.scroll_down(instruction.n() as usize),
                    0x0FB => self.display.scroll_right(),
                    0x0FC => self.display.scroll_left(),
                    _ => { /*Ignore for modern interpreters*/ }
                }
            0x1 => self.pc = instruction.nnn(),
            0x2 => {
                self.stack.push(self.pc);
                self.pc = instruction.nnn();
            }
            0x3 => if self.v[instruction.x()] == instruction.nn() {
                    self.pc += 2;
                },
            0x4 => if self.v[instruction.x()] != instruction.nn() {
                    self.pc += 2;
                },
            0x5 => if self.v[instruction.x()] == self.v[instruction.y()] {
                    self.pc += 2;
                },
            0x6 => { self.v[instruction.x()] = instruction.nn() }
            0x7 => { self.v[instruction.x()] = self.v[instruction.x()].wrapping_add(instruction.nn()); }
            0x8 => match instruction.n() {
                    0x0 => self.v[instruction.x()] = self.v[instruction.y()],
                    0x1 => self.v[instruction.x()] |= self.v[instruction.y()],
                    0x2 => self.v[instruction.x()] &= self.v[instruction.y()],
                    0x3 => self.v[instruction.x()] ^= self.v[instruction.y()],
                    0x4 => {
                        let (res, carry) = self.v[instruction.x()].overflowing_add(self.v[instruction.y()]);
                        self.v[instruction.x()] = res;
                        self.v[0xF] = if carry {
                            1
                        } else {
                            0
                        };
                    }
                    0x5 => {
                        let (res, carry) = self.v[instruction.x()].overflowing_sub(self.v[instruction.y()]);
                        self.v[instruction.x()] = res;
                        self.v[0xF] = if !carry {
                            1
                        } else {
                            0
                        };
                    }
                    0x6 => {
                        // Optional self.v[instruction.x()] = self.v[instruction.y()];
                        self.v[0xF] = self.v[instruction.x()] & 1;
                        self.v[instruction.x()] >>= 1;
                    }
                    0x7 => {
                        let (res, carry) = self.v[instruction.y()].overflowing_sub(self.v[instruction.x()]);
                        self.v[instruction.x()] = res;
                        self.v[0xF] = if !carry {
                            1
                        } else {
                            0
                        };
                    }
                    0xE => {
                        // Optional self.v[instruction.x()] = self.v[instruction.y()];
                        self.v[0xF] = self.v[instruction.x()] >> 7 & 1;
                        self.v[instruction.x()] <<= 1;
                    }
                    _ => println!("Invalid instruction: {instruction:#06x}"),
                }
            0x9 => if self.v[instruction.x()] != self.v[instruction.y()] {
                    self.pc += 2;
                }
            0xA => { self.i = instruction.nnn(); }
            0xB => { 
                // Optional self.pc = instruction.nnn() + self.v[0] as u16;
                self.pc = instruction.nnn() + self.v[instruction.x()] as u16;
            }
            0xC => self.v[instruction.x()] = rand::thread_rng().gen::<u8>() & instruction.nn(),
            0xD => {
                self.v[0xF] = 0;
                let x = self.v[instruction.x()] & 63;
                let y = self.v[instruction.y()] as usize & 31;
                let mut sprite = vec![];
                for row in 0..instruction.n() as usize {
                    sprite.push(self.memory[self.i as usize + row]);
                }
                if self.display.draw(x, y, sprite) {
                    self.v[0xF] = 1;
                }
            }
            0xE => match instruction.nn() {
                    0x9E => if self.input == Some(self.v[instruction.x()]) {
                        self.pc += 2;
                    }
                    0xA1 => if self.input != Some(self.v[instruction.x()]) {
                        self.pc += 2;
                    }
                    _ => println!("Invalid instruction: {instruction:#06x}"),
                }
            0xF => match instruction.nn() {
                    // Set Vx to the value of the delay timer
                    0x07 => self.v[instruction.x()] = self.dt,
                    0x0A => {
                        match self.input {
                            Some(n) => self.v[instruction.x()] = n,
                            None => {
                                // decrements the pc by 2 before incrementing it in tick(), so we end up here until input
                                self.pc -= 2;
                                self.tick();
                            },
                        }

                    }
                    // Set the delay timer to Vx
                    0x15 => self.dt = self.v[instruction.x()],
                    // Set the sound timer to Vx
                    0x18 => self.st = self.v[instruction.x()],
                    0x1E => {
                        let res = self.i.wrapping_add(self.v[instruction.x()] as u16);
                        // If I + Vx overflows out of normal addressing range set VF to 1
                        // This was not universal, but when unused it shouldn't matter
                        if res > 0xFFF || res < self.i {
                            self.v[0xF] = 1;
                        }
                        self.i = res;
                    }
                    0x29 => self.i = 0x50 + 5 * instruction.x() as u16,
                    // SuperChip BigHex characters
                    0x30 => {}
                    0x33 => {
                        let vx = self.v[instruction.x()];
                        let i = self.i as usize;
                        self.memory[i] = vx / 100;
                        self.memory[i + 1] = (vx / 10) % 10;
                        self.memory[i + 2] = vx % 10;
                    }
                    0x55 => {
                        for n in 0..instruction.x() as usize {
                            self.memory[self.i as usize + n] = self.v[n];
                        }
                    }
                    0x65 => {
                        for n in 0..=instruction.x() as usize {
                            self.v[n] = self.memory[self.i as usize + n];
                        }
                    }
                    0x75 => {}
                    0x85 => {}
                    _ => println!("Invalid instruction: {instruction:#06x}"),
                }
            _ => { /*categorically impossible*/ }
        }
    }
}

pub struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [Self::Channel]) {
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("CHIP-8", 64 * 8, 32 * 8)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .build()
        .unwrap();
    
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(8.0, 8.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator.create_texture_target(PixelFormatEnum::RGB24, 64, 32).unwrap();

    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),
        samples: None,
    };

    let audio_device = audio_subsystem.open_playback(None, &desired_spec, |spec| {
        SquareWave {
            phase_inc: 440.0 / spec.freq as f32,
            phase: 0.0,
            volume: 0.25,
        }
    }).unwrap();

    let mut chip_8 = Chip8::new("chip8-test-rom-with-audio.ch8");
    let mut start = std::time::Instant::now();
    let mut cycles = 0;

    loop {
        cycles += 1;
        chip_8.tick();
        chip_8.render(&mut texture, &mut canvas);
        chip_8.beep(&audio_device);
        chip_8.get_input(&mut event_pump);
        if start.elapsed() >= Duration::new(1, 0) {
            start = std::time::Instant::now();
            println!("cycles last second: {cycles}");
            cycles = 0;
        }
    }    
}

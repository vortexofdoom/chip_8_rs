use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use rand::Rng;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{Texture, Canvas};
use sdl2::video::Window;
use sdl2::pixels::PixelFormatEnum;

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

pub const COLOR_ON: [u8; 3] = [255, 255, 255];
pub const COLOR_OFF: [u8; 3] = [0, 0, 0];

#[derive(Debug)]
pub struct Display {
    color_on: [u8; 3],
    color_off: [u8; 3],
    hi_mode: bool,
    lo_res: [u64; 32],
    hi_res: [u128; 64],
}

impl Default for Display {
    fn default() -> Self {
        Self {
            color_on: [255, 255, 255],
            color_off: [0, 0, 0], 
            hi_mode: false,
            lo_res: [0; 32],
            hi_res: [0; 64], 
        }
    }
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.hi_mode {
            true => for row in self.hi_res.iter() {
                writeln!(f, "{row:0128b}")?;
            }
            false => for row in self.lo_res.iter() {
                writeln!(f, "{row:064b}")?;
            }
        }
        Ok(())
    }
}

impl Display {
    pub fn draw(&mut self, x: u8, y: usize, sprite: Vec<u8>) -> bool {
        let mut res = false;
        if self.hi_mode {
            for row in 0..sprite.len() {
                if y + row >= 64 {
                    break;
                }
                let sprite = u128::from_be_bytes([sprite[row], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]) >> x;
                if !res && self.hi_res[y + row] & sprite != 0 {
                    res = true;
                }
                self.hi_res[y + row] ^= sprite;
            }
        } else {
            for row in 0..sprite.len() {
                if y + row >= 32 {
                    break;
                }
                let sprite = u64::from_be_bytes([sprite[row], 0, 0, 0, 0, 0, 0, 0]) >> x;
                if !res && self.lo_res[y + row] & sprite != 0 {
                    res = true;
                }
                self.lo_res[y + row] ^= sprite;
            }
        }
        res
    }

    pub fn render(&self, texture: &mut Texture, canvas: &mut Canvas<Window>) {
        let (cols, rows) = if self.hi_mode {
            (128, 64)
        } else {
            (64, 32)
        };
        let mut data = vec![];
        let pixel = |row, col| {
            (if self.hi_mode { self.hi_res[row] } else { self.lo_res[row] as u128 } >> col) & 1 == 1
        };
        for row in 0..rows {
            for col in (0..cols).rev() {
                if pixel(row, col) {
                    data.extend_from_slice(&self.color_on);
                } else {
                    data.extend_from_slice(&self.color_off);
                };
            }
        }
        texture.update(None, &data, cols * 3).unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
    }

    pub fn clear(&mut self) {
        if self.hi_mode {
            self.hi_res.fill(0);
        } else {
            self.lo_res.fill(0);
        }
    }

    fn scroll_down(&mut self, rows: usize) {
        if self.hi_mode {
            // move down all rows starting from the back
            for row in (rows..64).rev() {
                self.hi_res[row] = self.hi_res[row - rows];
            }
            // set the remainder to 0
            for row in 0..rows {
                self.hi_res[row] = 0;
            }
        } else {
            for row in (rows..32).rev() {
                self.hi_res[row] = self.hi_res[row - rows];
            }
            for row in 0..rows {
                self.hi_res[row] = 0;
            }
        }
    }

    fn scroll_right(&mut self) {
        if self.hi_mode {
            for row in self.hi_res.iter_mut() {
                *row >>= 4;
            }
        } else {
            for row in self.lo_res.iter_mut() {
                *row >>= 4;
            }
        }
    }

    fn scroll_left(&mut self) {
        if self.hi_mode {
            for row in self.hi_res.iter_mut() {
                *row <<= 4;
            }
        } else {
            for row in self.lo_res.iter_mut() {
                *row <<= 4;
            }
        }
    }
}

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

pub trait Nibbles {
    /// Returns the second-most significant 4 bits (0000_XXXX_0000_0000)
    fn x(&self) -> usize;
    
    /// Returns the second-least significant 4 bits (0000_0000_XXXX_0000)
    fn y(&self) -> usize;

    /// Returns the least significant 4 bits (0000_0000_0000_XXXX)
    fn n(&self) -> u8;

    /// Returns the lower byte
    fn nn(&self) -> u8;

    /// Returns the lowest 12 bits
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

    pub fn render(&self, texture: &mut Texture, canvas: &mut Canvas<Window>) {
        self.display.render(texture, canvas);
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
    let mut chip_8 = Chip8::new("test_opcode.ch8");
    loop {
        chip_8.tick();
        chip_8.render(&mut texture, &mut canvas);
        chip_8.get_input(&mut event_pump);
    }    
}

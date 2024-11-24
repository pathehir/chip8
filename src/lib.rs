#![cfg_attr(not(feature = "std"), no_std)]

//! # CHIP-8
//! Implementation of [CHIP-8](https://en.wikipedia.org/wiki/chip-8) in rust

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

#[cfg(feature = "std")]
use std::time::Instant;

const DISPLAY_WIDTH: u8 = 64;
const DISPLAY_HEIGHT: u8 = 32;
const DISPLAY_SIZE: usize = DISPLAY_WIDTH as usize * DISPLAY_HEIGHT as usize / 8;

const FONT_BYTES: [u8; 80] = [
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

/// Struct representing the state of a CHIP-8 program
pub struct Chip8 {
    memory: [u8; 4096],
    display: [u8; DISPLAY_SIZE],
    pc: usize,
    i: u16,
    stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    registers: [u8; 16],

    #[cfg(feature = "std")]
    last_cycle: Instant,
    #[cfg(feature = "std")]
    last_timer: Instant,
}

impl Chip8 {
    /// Create a new [`Self`].
    /// This takes in a slice of bytes as program input and optionally a custom font.
    pub fn new(program: &[u8], custom_font: Option<[u8; 80]>) -> Self {
        let mut memory = [0; 4096];
        memory[0x050..0x0A0].copy_from_slice(&custom_font.unwrap_or(FONT_BYTES));
        memory[0x200..0x200 + program.len()].copy_from_slice(program);

        Self {
            memory,
            display: [0; DISPLAY_SIZE],
            pc: 0x200,
            i: 0,
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            registers: [0; 16],

            #[cfg(feature = "std")]
            last_cycle: Instant::now(),
            #[cfg(feature = "std")]
            last_timer: Instant::now(),
        }
    }

    /// access memory (useful for debugging).
    pub fn memory(&self) -> [u8; 4096] {
        self.memory
    }

    /// Get display output.
    /// You probably want to put a callback in [`Self::update`] instead.
    pub fn display(&self) -> [u8; DISPLAY_SIZE] {
        self.display
    }

    /// Run this inside your loop.
    /// Only runs [`Self::cycle`] and [`Self::timers`] when they need to be run.
    #[cfg(feature = "std")]
    pub fn update(&mut self, draw: impl FnMut([u8; DISPLAY_SIZE]), beep: impl FnMut()) {
        const CLOCK_DUR: f64 = 1. / 700.;
        const TIMER_DUR: f64 = 1. / 60.;

        if self.last_cycle.elapsed().as_secs_f64() > CLOCK_DUR {
            self.cycle(draw);
            self.last_cycle = Instant::now();
        }

        if self.last_timer.elapsed().as_secs_f64() > TIMER_DUR {
            self.timers(beep);
            self.last_timer = Instant::now();
        }
    }

    pub fn cycle(&mut self, mut draw: impl FnMut([u8; DISPLAY_SIZE])) {
        let current = self.memory[self.pc];
        let current2 = self.memory[self.pc + 1];
        let (o, x, y, n) = (current >> 4, current & 0x0F, current2 >> 4, current2 & 0x0F);

        self.pc += 2;

        match o {
            0x0 if x == 0x0 && y == 0xE => match n {
                0x0 => self.display = [0; DISPLAY_SIZE],
                0xE => self.pc = self.stack.pop().unwrap() as usize,
                _ => todo!(),
            },
            0x1 => {
                let addr = (x as u16) << 8 | (y as u16) << 4 | n as u16;
                self.pc = addr as usize;
            }
            0x2 => {
                let addr = (x as u16) << 8 | (y as u16) << 4 | n as u16;
                self.stack.push(self.pc as u16);
                self.pc = addr as usize;
            }
            0x3 => {
                let value = self.registers[x as usize];
                let input = y << 4 | n;

                if value == input {
                    self.pc += 2;
                }
            }
            0x4 => {
                let value = self.registers[x as usize];
                let input = y << 4 | n;

                if value != input {
                    self.pc += 2;
                }
            }
            0x5 if n == 0 => {
                let vx = self.registers[x as usize];
                let vy = self.registers[y as usize];

                if vx == vy {
                    self.pc += 2;
                }
            }
            0x6 => {
                let value = y << 4 | n;
                self.registers[x as usize] = value;
            }
            0x7 => {
                let value = y << 4 | n;
                (self.registers[x as usize], _) = self.registers[x as usize].overflowing_add(value);
            }
            0x8 => match n {
                0x0 => self.registers[x as usize] = self.registers[y as usize],
                0x1 => self.registers[x as usize] |= self.registers[y as usize],
                0x2 => self.registers[x as usize] &= self.registers[y as usize],
                0x3 => self.registers[x as usize] ^= self.registers[y as usize],
                0x4 => {
                    let out = self.registers[x as usize] as u16 + self.registers[y as usize] as u16;
                    self.registers[x as usize] = (out & 0x00FF) as u8;

                    if out & 0xFF00 != 0 {
                        self.registers[0xF] = 1;
                    } else {
                        self.registers[0xF] = 0;
                    }
                }
                0x5 => {
                    let vx = self.registers[x as usize];
                    let vy = self.registers[y as usize];

                    if vx > vy {
                        self.registers[x as usize] = vx - vy;
                        self.registers[0xF] = 1;
                    } else {
                        self.registers[x as usize] = (0x100 - (vy - vx) as u16) as u8;
                        self.registers[0xF] = 0;
                    }
                }
                0x7 => {
                    let vx = self.registers[x as usize];
                    let vy = self.registers[y as usize];

                    if vy > vx {
                        self.registers[x as usize] = vy - vx;
                        self.registers[0xF] = 1;
                    } else {
                        self.registers[x as usize] = (0x100 - (vx - vy) as u16) as u8;
                        self.registers[0xF] = 0;
                    }
                }
                _ => todo!(),
            },
            0x9 if n == 0 => {
                let vx = self.registers[x as usize];
                let vy = self.registers[y as usize];

                if vx != vy {
                    self.pc += 2;
                }
            }
            0xA => {
                let idx = (x as u16) << 8 | (y as u16) << 4 | n as u16;
                self.i = idx;
            }
            0xD => {
                let x = self.registers[x as usize] & (DISPLAY_WIDTH - 1);
                let x_byte = x / 8;
                let x_offset = x % 8;

                let y = self.registers[y as usize] & (DISPLAY_HEIGHT - 1);

                self.registers[0xF] = 0;
                for i in 0..n {
                    if y + i >= DISPLAY_HEIGHT {
                        break;
                    }

                    let di = (y + i) * (DISPLAY_WIDTH / 8) + x_byte;

                    let row = self.memory[self.i as usize + i as usize];

                    self.display[di as usize] ^= row >> x_offset;

                    if x_byte < DISPLAY_WIDTH / 8 && x_offset != 0 {
                        self.display[di as usize + 1] ^= row << (8 - x_offset);
                    }
                }

                draw(self.display);
            }
            _ => panic!("opcode: {:#02x}{:02x}", current, current2),
        }
    }

    pub fn timers(&mut self, mut beep: impl FnMut()) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            beep();
            self.sound_timer -= 1;
        }
    }
}

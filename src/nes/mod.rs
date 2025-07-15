pub mod cpu;

use cpu::Cpu;
use rand::prelude::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::Canvas;
use sdl2::render::Texture;
use sdl2::render::TextureCreator;
use sdl2::video::Window;
use sdl2::video::WindowContext;
use sdl2::EventPump;

pub struct NES<'a> {
    clock: u64,
    cpu: Cpu,
    texture: Texture<'a>,
    canvas: Canvas<Window>,
    screen_state: [u8; 32 * 3 * 32],
    rng: ThreadRng,
}

impl<'a> NES<'a> {
    pub fn new(
        texture_creator: &'a TextureCreator<WindowContext>,
        canvas: Canvas<Window>,
        rng: ThreadRng,
    ) -> NES<'a> {
        let texture = texture_creator
            .create_texture_target(PixelFormatEnum::RGB24, 32, 32)
            .unwrap();
        let mut cpu = Cpu::new();
        cpu.load_test_game();
        cpu.reset();

        NES {
            clock: 0,
            cpu,
            texture,
            canvas,
            rng,
            screen_state: [0u8; 32 * 3 * 32],
        }
    }

    pub fn tick(&mut self, event_pump: &mut EventPump) {
        self.clock += 1;
        let screen_state = &mut self.screen_state;
        let texture = &mut self.texture;
        let canvas = &mut self.canvas;
        let rng = &mut self.rng;

        self.cpu.run_with_callback(|cpu| {
            NES::handle_user_input(cpu, event_pump);
            cpu.mem_write(0xFE, rng.random_range(1..16));

            if NES::read_screen_state(cpu, screen_state) {
                texture.update(None, screen_state, 32 * 3).unwrap();
                canvas.copy(texture, None, None).unwrap();
                canvas.present();
            }

            std::thread::sleep(std::time::Duration::new(0, 700));
        });
    }

    pub fn enable_cpu_debug(&mut self) {
        self.cpu.enable_debug();
    }

    pub fn handle_user_input(cpu: &mut Cpu, event_pump: &mut EventPump) {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    std::process::exit(0);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    ..
                } => {
                    cpu.mem_write(0xFF, 0x77);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    cpu.mem_write(0xFF, 0x73);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    cpu.mem_write(0xFF, 0x61);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    ..
                } => {
                    cpu.mem_write(0xFF, 0x64);
                }
                _ => {}
            }
        }
    }

    fn color(byte: u8) -> Color {
        match byte {
            0 => sdl2::pixels::Color::BLACK,
            1 => sdl2::pixels::Color::WHITE,
            2 | 9 => sdl2::pixels::Color::GREY,
            3 | 10 => sdl2::pixels::Color::RED,
            4 | 11 => sdl2::pixels::Color::GREEN,
            5 | 12 => sdl2::pixels::Color::BLUE,
            6 | 13 => sdl2::pixels::Color::MAGENTA,
            7 | 14 => sdl2::pixels::Color::YELLOW,
            _ => sdl2::pixels::Color::CYAN,
        }
    }

    fn read_screen_state(cpu: &Cpu, frame: &mut [u8; 32 * 3 * 32]) -> bool {
        let mut frame_idx = 0;
        let mut update = false;
        for i in 0x0200..0x0600 {
            let color_idx = cpu.mem_read(i as u16);
            let (b1, b2, b3) = NES::color(color_idx).rgb();
            if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
                frame[frame_idx] = b1;
                frame[frame_idx + 1] = b2;
                frame[frame_idx + 2] = b3;
                update = true;
            }
            frame_idx += 3;
        }
        update
    }
}

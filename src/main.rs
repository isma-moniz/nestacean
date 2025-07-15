use nestacean::nes::NES;

fn main() {
    // init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Snake game", (32.0 * 10.0) as u32, (32.0 * 10.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    canvas.set_scale(10.0, 10.0).unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let texture_creator = canvas.texture_creator();
    let rng = rand::rng();

    let mut nes = NES::new(&texture_creator, canvas, rng);

    // nes.enable_cpu_debug();
    loop {
        //TODO: only interrupted with manual interrupts right now
        nes.tick(&mut event_pump);
    }
}

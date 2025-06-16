use nestacean::nes::NES;

fn main() {
    let mut nes = NES::new();
    nes.enable_cpu_debug();
    loop {
        //TODO: only interrupted with manual interrupts right now
        nes.tick();
    }
}

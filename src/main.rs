use nestacean::nes::NES;

fn main() {
    let mut nes = NES::new();
    loop {
        //TODO: only interrupted with manual interrupts right now
        nes.tick();
    }
}

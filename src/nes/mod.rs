pub mod cpu;

use cpu::Cpu;

pub struct NES {
    clock: u64,
    cpu: Cpu,
}

impl NES {
    pub fn new() -> Self {
        let mut cpu = Cpu::new();
        cpu.load_test_game();
        cpu.reset();

        Self { clock: 0, cpu }
    }

    pub fn tick(&mut self) {
        self.clock += 1;
        self.cpu.tick();
    }

    pub fn enable_cpu_debug(&mut self) {
        self.cpu.enable_debug();
    }
}

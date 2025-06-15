pub mod cpu;

use cpu::Cpu;

pub struct NES {
    clock: u64,
    cpu: Cpu,
}

impl NES {
    pub fn new() -> Self {
        Self {
            clock: 0,
            cpu: Cpu::new(),
        }
    }

    pub fn tick(&mut self) {
        self.clock += 1;
    }
}
pub mod cpu;

use cpu::Cpu;

pub struct NES {
    clock: u64,
    cpu: Cpu,
}

impl NES {
    pub fn new() -> Self {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xA5, 0x00, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0, 0x05);

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

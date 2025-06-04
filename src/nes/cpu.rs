use std::collections::VecDeque;

#[derive(Debug)]
enum MicroOp {
    ReadImmediate,
    LoadAccumulator(u8),
    LoadAccPlaceholder,
    LoadX(u8),
    LoadXPlaceholder,
    LoadY(u8),
    LoadYPlaceholder,
    Break,
    ReadAccumulator,
    LoadAccumulatorImmediate,
    LoadXAccumulator,
    IncrementX,
}

pub struct Cpu {
    accumulator: u8,
    index_x: u8,
    index_y: u8,
    pc: u16,
    sp: u8,
    status_p: u8,
    current_inst: VecDeque<MicroOp>,
    memory: [u8; 0xFFFF],
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            accumulator: 0u8,
            index_x: 0u8,
            index_y: 0u8,
            pc: 0u16,
            sp: 0u8,
            status_p: 0u8,
            current_inst: VecDeque::new(),
            memory: [0u8; 0xFFFF],
        }
    }

    fn mem_read(&self, pos: u16) -> u8 {
        self.memory[pos as usize]
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let low_byte = self.mem_read(pos) as u16;
        let high_byte = self.mem_read(pos + 1) as u16;
        (high_byte << 8) | low_byte
    }

    fn mem_write(&mut self, pos: u16, byte: u8) {
        self.memory[pos as usize] = byte;
    }

    fn mem_write_u16(&mut self, pos: u16, byte: u16) {
        let low_byte = (byte & 0xFF) as u8;
        let high_byte = (byte >> 8) as u8;
        self.mem_write(pos, low_byte);
        self.mem_write(pos + 1, high_byte);
    }

    pub fn load_program(&mut self, program: &[u8]) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.pc = 0x8000;
    }

    pub fn tick(&mut self, mem: &mut [u8]) {
        if self.current_inst.is_empty() {
            let opcode = mem[self.pc as usize];
            self.pc += 1;
            self.current_inst = self.decode_opcode(opcode);
        } else if let Some(op) = self.current_inst.pop_front() {
            self.execute_micro_op(op, mem);
        }
    }

    fn decode_opcode(&mut self, opcode: u8) -> VecDeque<MicroOp> {
        match opcode {
            0xA9 => {
                // LDA
                VecDeque::from(vec![MicroOp::LoadAccumulatorImmediate])
            }
            0xAA => {
                // TAX
                VecDeque::from(vec![MicroOp::LoadXAccumulator])
            }
            0xE8 => {
                // INX
                VecDeque::from(vec![MicroOp::IncrementX])
            }
            0x00 => {
                // BRK
                VecDeque::from(vec![MicroOp::Break])
            }
            _ => unimplemented!(),
        }
    }

    fn execute_micro_op(&mut self, operation: MicroOp, mem: &[u8]) {
        match operation {
            MicroOp::ReadImmediate => {
                let value = mem[self.pc as usize];
                self.pc += 1;

                match self.current_inst.pop_front() {
                    Some(MicroOp::LoadAccPlaceholder) => {
                        self.current_inst
                            .push_front(MicroOp::LoadAccumulator(value));
                    }
                    Some(other) => panic!("Unexpected micro-op after ReadImmediate: {:?}", other),
                    None => panic!("No micro-op after ReadImmediate"),
                }
            }
            MicroOp::ReadAccumulator => {
                let value = self.accumulator;
                match self.current_inst.pop_front() {
                    Some(MicroOp::LoadXPlaceholder) => {
                        self.current_inst.push_front(MicroOp::LoadX(value));
                    }
                    Some(other) => panic!("Unexpected micro-op after ReadAccumulator: {:?}", other),
                    None => panic!("No micro-op after ReadAccumulator"),
                }
            }
            MicroOp::LoadAccumulator(value) => {
                self.accumulator = value;
                // set zero flag
                if self.accumulator == 0x00 {
                    self.status_p = self.status_p | 0b0000_0010;
                } else {
                    self.status_p = self.status_p & 0b1111_1101;
                }
                // set negative flag
                if self.accumulator & 0b1000_0000 != 0 {
                    self.status_p = self.status_p | 0b1000_0000;
                } else {
                    self.status_p = self.status_p & 0b0111_1111;
                }
            }
            MicroOp::LoadX(value) => {
                self.index_x = value;
                // set zero flag
                if self.index_x == 0x00 {
                    self.status_p = self.status_p | 0b0000_0010;
                } else {
                    self.status_p = self.status_p & 0b1111_1101;
                }
                // set negative flag
                if self.index_x & 0b1000_0000 != 0 {
                    self.status_p = self.status_p | 0b1000_0000;
                } else {
                    self.status_p = self.status_p & 0b0111_1111;
                }
            }
            MicroOp::LoadAccumulatorImmediate => {
                let value = mem[self.pc as usize];
                self.pc += 1;
                self.accumulator = value;

                //set zero flag
                if self.accumulator == 0x00 {
                    self.status_p = self.status_p | 0b0000_0010;
                } else {
                    self.status_p = self.status_p & 0b1111_1101;
                }
                // set negative flag
                if self.accumulator & 0b1000_0000 != 0 {
                    self.status_p = self.status_p | 0b1000_0000;
                } else {
                    self.status_p = self.status_p & 0b0111_1111;
                }
            }
            MicroOp::LoadXAccumulator => {
                let value = self.accumulator;
                self.index_x = value;

                // set zero flag
                if self.index_x == 0x00 {
                    self.status_p = self.status_p | 0b0000_0010;
                } else {
                    self.status_p = self.status_p & 0b1111_1101;
                }
                // set negative flag
                if self.index_x & 0b1000_0000 != 0 {
                    self.status_p = self.status_p | 0b1000_0000;
                } else {
                    self.status_p = self.status_p & 0b0111_1111;
                }
            }
            MicroOp::IncrementX => {
                self.index_x = self.index_x.wrapping_add(1);

                // set zero flag
                if self.index_x == 0x00 {
                    self.status_p = self.status_p | 0b0000_0010;
                } else {
                    self.status_p = self.status_p & 0b1111_1101;
                }
                // set negative flag
                if self.index_x & 0b1000_0000 != 0 {
                    self.status_p = self.status_p | 0b1000_0000;
                } else {
                    self.status_p = self.status_p & 0b0111_1111;
                }
            }
            MicroOp::Break => {
                //TODO: this op is more complex. research and implement.
                return;
            }
            _ => unimplemented!(),
        }
    }
}

// TESTING AREA

#[cfg(test)]
mod test {
    use super::*;

    // LDA tests
    #[test]
    fn test_lda() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xA9, 0x05, 0xFF];
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadAccumulatorImmediate
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.status_p & 0b0000_0010, 0);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }

    #[test]
    fn test_lda_zeroflag() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xA9, 0x00, 0xFF];
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadAccumulatorImmediate
        assert_eq!(cpu.accumulator, 0x00);
        assert_eq!(cpu.status_p & 0b0000_0010, 0b10);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }

    #[test]
    fn test_lda_negflag() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xA9, 0xFF, 0xFF];
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadAccumulatorImmediate
        assert_eq!(cpu.accumulator, 0xFF);
        assert_eq!(cpu.status_p & 0b0000_0010, 0);
        assert_eq!(cpu.status_p & 0b1000_0000, 0b1000_0000);
    }

    // TAX tests
    #[test]
    fn test_tax() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xAA, 0x00, 0xFF];
        cpu.accumulator = 0x05;
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadXAccumulator
        assert_eq!(cpu.index_x, 0x05);
        assert_eq!(cpu.status_p & 0b0000_0010, 0);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }

    #[test]
    fn test_tax_zeroflag() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xAA, 0x00, 0xFF];
        cpu.accumulator = 0x00;
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadXAccumulator
        assert_eq!(cpu.index_x, 0x00);
        assert_eq!(cpu.status_p & 0b0000_0010, 0b10);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }

    #[test]
    fn test_tax_negflag() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xAA, 0x00, 0xFF];
        cpu.accumulator = 0xFF;
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadXAccumulator
        assert_eq!(cpu.index_x, 0xFF);
        assert_eq!(cpu.status_p & 0b0000_0010, 0);
        assert_eq!(cpu.status_p & 0b1000_0000, 0b1000_0000);
    }

    // INX tests
    #[test]
    fn test_inx() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xE8, 0xFF, 0xFF];
        cpu.index_x = 0x00;
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //IncrementX
        assert_eq!(cpu.index_x, 0b01);
        assert_eq!(cpu.status_p & 0b0000_0010, 0);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }

    #[test]
    fn test_inx_zeroflag() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xE8, 0xFF, 0xFF];
        cpu.index_x = 0xFF;
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //IncrementX
        assert_eq!(cpu.index_x, 0x00);
        assert_eq!(cpu.status_p & 0b0000_0010, 0b10);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }

    #[test]
    fn test_inx_negflag() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 3] = [0xE8, 0xFF, 0xFF];
        cpu.index_x = 0x7F;
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //IncrementX
        assert_eq!(cpu.index_x, 0x80);
        assert_eq!(cpu.status_p & 0b0000_0010, 0);
        assert_eq!(cpu.status_p & 0b1000_0000, 0b1000_0000);
    }

    // general testing
    #[test]
    fn test_5_ops() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 5] = [0xa9, 0xc0, 0xaa, 0xe8, 0x00];
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadAccumulatorImmediate
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //LoadXAccumulator
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //IncrementX
        cpu.tick(&mut mem); //fetch and decode
        cpu.tick(&mut mem); //Break

        assert_eq!(cpu.index_x, 0xc1);
    }
}

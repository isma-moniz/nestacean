use std::collections::VecDeque;

enum MicroOp {
    ReadImmediate,
    LoadAccumulator(u8),
    Return,
}

pub struct Cpu {
    accumulator: u8,
    index_x: u8,
    index_y: u8,
    pc: u16,
    sp: u8,
    status_p: u8,
    current_inst: VecDeque<MicroOp>,
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
        }
    }

    pub fn tick(&mut self, mem: &mut [u8]) {
        if self.current_inst.is_empty() {
            let opcode = mem[self.pc as usize];
            self.pc += 1;
            self.current_inst = self.decode_opcode(opcode);
        }
        if let Some(op) = self.current_inst.pop_front() {
            self.execute_micro_op(op, mem);
        }
    }

    fn decode_opcode(&mut self, opcode: u8) -> VecDeque<MicroOp> {
        match opcode {
            0xA9 => {
                // LDA
                VecDeque::from(vec![MicroOp::ReadImmediate])
            }
            0x00 => {
                // RETURN
                VecDeque::from(vec![MicroOp::Return])
            }
            _ => unimplemented!(),
        }
    }

    fn execute_micro_op(&mut self, operation: MicroOp, mem: &[u8]) {
        match operation {
            MicroOp::ReadImmediate => {
                let value = mem[self.pc as usize];
                // self.pc += 1;
                self.current_inst
                    .push_front(MicroOp::LoadAccumulator(value));
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
            MicroOp::Return => {
                return;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lda() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 2] = [0xA9, 0x05];
        cpu.tick(&mut mem); //fetch
        cpu.tick(&mut mem); //ReadImmediate
        cpu.tick(&mut mem); // LoadAccumulator
        assert_eq!(cpu.accumulator, 0x05);
        assert_eq!(cpu.status_p & 0b0000_0010, 0);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }

    #[test]
    fn test_lda_zeroflag() {
        let mut cpu = Cpu::new();
        let mut mem: [u8; 2] = [0xA9, 0x00];
        cpu.tick(&mut mem); //fetch
        cpu.tick(&mut mem); //ReadImmediate
        cpu.tick(&mut mem); // LoadAccumulator
        assert_eq!(cpu.accumulator, 0x00);
        assert_eq!(cpu.status_p & 0b0000_0010, 0b10);
        assert_eq!(cpu.status_p & 0b1000_0000, 0);
    }
}

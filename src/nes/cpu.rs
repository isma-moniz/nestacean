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
    Return,
    ReadAccumulator,
    LoadAccumulatorImmediate,
    LoadXAccumulator,
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
            MicroOp::Return => {
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
}

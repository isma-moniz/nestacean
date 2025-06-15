use std::collections::VecDeque;

#[derive(Debug)]
pub enum MicroOp {
    LoadAccPlaceholder,
    Break,
    ReadAccumulator,
    LoadAccumulatorImmediate,
    LoadAccumulatorFromAddress(u16),
    FetchLowAddrByte,
    FetchHighAddrByte,
    FetchHighAddrByteWithX,
    FetchHighAddrByteWithY,
    AddXtoAddressPlaceholder,
    AddXtoAddress(u16),
    AddXLoadImmediatePlaceholder,
    AddXLoadImmediate(u16),
    AddYLoadImmediatePlaceholder,
    AddYLoadImmediate(u16),
    FetchZeroPage,
    LoadXAccumulator,
    IncrementX,
    DummyCycle,
    FetchPointer,
    AddXtoPointerPlaceholder,
    AddXtoPointer(u8),
    FetchPointerBytePlaceholder,
    FetchPointerByteWithYPlaceholder,
    FetchPointerLowByte(u8),
    FetchPointerHighByte(u8),
    FetchPointerHighByteWithY(u8),
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
    temp_addr: u16,
    page_crossed: bool,
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
            temp_addr: 0u16,
            page_crossed: false,
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

    //TODO: might be redundant to have this and the self initializer. see load_program
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.index_x = 0;
        self.index_y = 0;
        self.sp = 0;
        self.sp = 0;
        self.status_p = 0;
        self.temp_addr = 0;
        self.page_crossed = false;
        self.current_inst = VecDeque::new();
        self.pc = self.mem_read_u16(0xFFFC);
    }

    pub fn load_program(&mut self, program: &[u8]) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000); // why not load the pc directly?
    }

    pub fn tick(&mut self) {
        if self.current_inst.is_empty() {
            let opcode = self.memory[self.pc as usize];
            self.pc += 1;
            self.current_inst = self.decode_opcode(opcode);
        } else if let Some(op) = self.current_inst.pop_front() {
            self.execute_micro_op(op);
        }
    }

    fn decode_opcode(&mut self, opcode: u8) -> VecDeque<MicroOp> {
        match opcode {
            0xA9 => {
                // LDA
                VecDeque::from(vec![MicroOp::LoadAccumulatorImmediate])
            }
            0xA5 => {
                // LDA zero page
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::LoadAccumulatorImmediate,
                ])
            }
            0xB5 => {
                // LDA zero page + x
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoAddressPlaceholder,
                    MicroOp::LoadAccumulatorImmediate,
                ])
            }
            0xAD => {
                // LDA absolute
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByte,
                    MicroOp::LoadAccumulatorImmediate,
                ])
            }
            0xBD => {
                // LDA absolute + x
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithX,
                    MicroOp::LoadAccumulatorImmediate,
                ])
            }
            0xB9 => {
                // LDA absolute + y
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithY,
                    MicroOp::LoadAccumulatorImmediate,
                ])
            }
            0xA1 => {
                // LDA indexed indirect
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage, // does the same thing
                    MicroOp::AddXtoPointerPlaceholder,
                    MicroOp::FetchPointerBytePlaceholder, // will be 2 ops after processed
                    MicroOp::LoadAccumulatorImmediate,
                ])
            }
            0xB1 => {
                // LDA indirect indexed
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::FetchPointerByteWithYPlaceholder,
                    MicroOp::LoadAccumulatorImmediate, // may add dummy cycle
                ])
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
            _ => unimplemented!("{}", opcode),
        }
    }

    fn push_micro_from_placeholder(&mut self, value: u16) {
        match self.current_inst.pop_front() {
            Some(MicroOp::LoadAccumulatorImmediate) => {
                self.current_inst
                    .push_front(MicroOp::LoadAccumulatorFromAddress(value));
                if self.page_crossed {
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::AddXtoAddressPlaceholder) => {
                self.current_inst.push_front(MicroOp::AddXtoAddress(value));
            }
            Some(MicroOp::AddXtoPointerPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::AddXtoPointer(value as u8));
            }
            Some(MicroOp::FetchPointerBytePlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::FetchPointerHighByte(value as u8));
                self.current_inst
                    .push_front(MicroOp::FetchPointerLowByte(value as u8));
            }
            Some(MicroOp::FetchPointerByteWithYPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::FetchPointerHighByteWithY(value as u8));
                self.current_inst
                    .push_front(MicroOp::FetchPointerLowByte(value as u8));
            }
            Some(other) => panic!("Unexpected micro-op: {:?}", other),
            None => panic!("No micro-op!"),
        }
    }

    fn execute_micro_op(&mut self, operation: MicroOp) {
        match operation {
            MicroOp::FetchZeroPage => {
                let address = self.memory[self.pc as usize];
                self.pc += 1;

                self.push_micro_from_placeholder(address as u16);
            }
            MicroOp::AddXtoAddress(address) => {
                let new_address = address.wrapping_add(self.index_x as u16);
                self.push_micro_from_placeholder(new_address);
            }
            MicroOp::AddXtoPointer(pointer) => {
                let new_pointer: u8 = pointer.wrapping_add(self.index_x);
                self.push_micro_from_placeholder(new_pointer as u16);
            }
            MicroOp::FetchLowAddrByte => {
                self.temp_addr = self.mem_read(self.pc) as u16;
                self.pc += 1;
            }
            MicroOp::FetchHighAddrByte => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::FetchHighAddrByteWithX => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                let new_addr = self.temp_addr.wrapping_add(self.index_x as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::FetchHighAddrByteWithY => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                let new_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::FetchPointerLowByte(pointer) => {
                self.temp_addr = self.mem_read(pointer as u16) as u16;
            }
            MicroOp::FetchPointerHighByte(pointer) => {
                self.temp_addr |= (self.mem_read((pointer as u16).wrapping_add(1)) as u16) << 8;
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::FetchPointerHighByteWithY(pointer) => {
                self.temp_addr |= (self.mem_read((pointer as u16).wrapping_add(1)) as u16) << 8;
                self.temp_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::LoadAccumulatorImmediate => {
                let value = self.memory[self.pc as usize];
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
            MicroOp::LoadAccumulatorFromAddress(address) => {
                let value = self.memory[address as usize];
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
            MicroOp::DummyCycle => {
                return;
            }
            _ => unimplemented!(),
        }
    }

    pub fn get_accumulator(&self) -> u8 {
        self.accumulator
    }

    pub fn get_index_x(&self) -> u8 {
        self.index_x
    }

    pub fn get_index_y(&self) -> u8 {
        self.index_y
    }

    pub fn get_pc(&self) -> u16 {
        self.pc
    }

    pub fn get_sp(&self) -> u8 {
        self.sp
    }

    pub fn get_status_p(&self) -> u8 {
        self.status_p
    }

    pub fn get_current_inst(&self) -> &VecDeque<MicroOp> {
        &self.current_inst
    }

    pub fn get_memory(&mut self) -> &mut [u8; 0xFFFF] {
        &mut self.memory
    }

    pub fn get_temp_addr(&self) -> u16 {
        self.temp_addr
    }

    pub fn is_page_crossed(&self) -> bool {
        self.page_crossed
    }

    pub fn set_accumulator(&mut self, val: u8) {
        self.accumulator = val;
    }

    pub fn set_index_x(&mut self, val: u8) {
        self.index_x = val;
    }
}

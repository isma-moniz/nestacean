use std::collections::VecDeque;
use std::io::{self, Write};

const CLS: &str = "\x1B[2J\x1B[1;1H";

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
    AddXtoZeroPageAddressPlaceholder,
    AddXtoZeroPageAddress(u8),
    AddXLoadImmediatePlaceholder,
    AddXLoadImmediate(u16),
    AddYLoadImmediatePlaceholder,
    AddYLoadImmediate(u16),
    FetchZeroPage,
    LoadXAccumulator,
    IncrementX,
    IncrementY,
    DecrementX,
    DecrementY,
    DummyCycle,
    FixAddressPlaceholder, // just a dummy cycle but with passthrough of the provided value
    FixAddress(u16),
    FetchPointer,
    AddXtoPointerPlaceholder,
    AddXtoPointer(u8),
    FetchPointerBytePlaceholder,
    FetchPointerByteWithYPlaceholder,
    FetchPointerLowByte(u8),
    FetchPointerHighByte(u8),
    FetchPointerHighByteWithY(u8),
    ReadAddressPlaceholder,
    ReadAddress(u16),
    WriteBackAndIncrementPlaceholder,
    WriteBackAndIncrement(u8),
    WriteBackAndDecrementPlaceholder,
    WriteBackAndDecrement(u8),
    WriteToAddressPlaceholder,
    WriteToAddress(u8),
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
    debug_active: bool,
    debug_mem_page: u8,
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
            debug_active: false,
            debug_mem_page: 0u8,
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

    pub fn enable_debug(&mut self) {
        self.debug_active = true;
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

    fn set_flags_zero_neg(&mut self, value: u8) {
        // set zero flag
        if value == 0x00 {
            self.status_p = self.status_p | 0b0000_0010;
        } else {
            self.status_p = self.status_p & 0b1111_1101;
        }

        // set negative flag
        if value & 0b1000_0000 != 0 {
            self.status_p = self.status_p | 0b1000_0000;
        } else {
            self.status_p = self.status_p & 0b0111_1111;
        }
    }

    //TODO: might be redundant to have this and the self initializer. see load_program
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.index_x = 0;
        self.index_y = 0;
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
        if self.debug_active {
            loop {
                self.print_debug_info();
                print!(
                    "Enter command (n = next mempage, p = previous mempage, <Enter> = continue): "
                );
                io::stdout().flush().unwrap();
                let mut input = String::new();
                if let Ok(_) = io::stdin().read_line(&mut input) {
                    match input.trim() {
                        "n" => self.debug_mem_page = self.debug_mem_page.wrapping_add(1),
                        "p" => self.debug_mem_page = self.debug_mem_page.wrapping_sub(1),
                        "" => break,
                        _ => continue,
                    }
                }
            }
        }
        self.execute_current_cycle();
    }

    fn execute_current_cycle(&mut self) {
        if self.current_inst.is_empty() {
            let opcode = self.memory[self.pc as usize];
            self.pc += 1;
            self.current_inst = self.decode_opcode(opcode);
        } else if let Some(op) = self.current_inst.pop_front() {
            self.execute_micro_op(op);
        }
    }

    fn print_debug_info(&self) {
        print!("{}", CLS);
        println!("PC: {:04X} | SP: {:02X}", self.pc, self.sp);
        println!(
            "X: {:02X} | Y: {:02X} | A: {:02X}",
            self.index_x, self.index_y, self.accumulator
        );
        println!("P: {:b}", self.status_p);
        println!(
            "temp_addr: {:04X} val: {:02X}",
            self.temp_addr,
            self.mem_read(self.temp_addr)
        );

        println!("Memory page {:02X}:", self.debug_mem_page);
        for i in 0..=0xFF {
            print!(
                "{:02X} ",
                self.memory[(self.debug_mem_page << 2 | i) as usize]
            );
        }
        println!("");
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
            0xE6 => {
                // INC zero page
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
            }
            0xF6 => {
                // INC zero page + x
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddressPlaceholder,
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
            }
            0xEE => {
                // INC absolute
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByte,
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
            }
            0xFE => {
                // INC absolute + x
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithX,
                    MicroOp::FixAddressPlaceholder, // always happens with this instruction
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
            }
            0xE8 => {
                // INX
                VecDeque::from(vec![MicroOp::IncrementX])
            }
            0xCA => {
                // DEX
                VecDeque::from(vec![MicroOp::DecrementX])
            }
            0xC8 => {
                // INY
                VecDeque::from(vec![MicroOp::IncrementY])
            }
            0x88 => {
                // DEY
                VecDeque::from(vec![MicroOp::DecrementY])
            }
            0xC6 => {
                // DEC zero page
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
            }
            0xD6 => {
                // DEC zero page + x
                VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddressPlaceholder,
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
            }
            0xCE => {
                // DEC absolute
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByte,
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
            }
            0xDE => {
                // DEC absolute + x
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithX,
                    MicroOp::FixAddressPlaceholder, // always happens with this instruction
                    MicroOp::ReadAddressPlaceholder,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    MicroOp::WriteToAddressPlaceholder,
                ])
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
            Some(MicroOp::WriteToAddressPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::WriteToAddress(value as u8));
            }
            Some(MicroOp::ReadAddressPlaceholder) => {
                self.current_inst.push_front(MicroOp::ReadAddress(value));
            }
            Some(MicroOp::WriteBackAndIncrementPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::WriteBackAndIncrement(value as u8));
            }
            Some(MicroOp::WriteBackAndDecrementPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::WriteBackAndDecrement(value as u8));
            }
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
            Some(MicroOp::AddXtoZeroPageAddressPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::AddXtoZeroPageAddress(value as u8));
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
            Some(MicroOp::FixAddressPlaceholder) => {
                self.current_inst.push_front(MicroOp::FixAddress(value));
            }
            Some(other) => panic!("Unexpected micro-op: {:?}", other),
            None => panic!("No micro-op!"),
        }
    }

    fn execute_micro_op(&mut self, operation: MicroOp) {
        match operation {
            MicroOp::ReadAddress(address) => {
                let value = self.mem_read(address);

                self.push_micro_from_placeholder(value as u16);
            }
            MicroOp::FetchZeroPage => {
                self.temp_addr = self.memory[self.pc as usize] as u16;
                self.pc += 1;

                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::AddXtoAddress(address) => {
                let new_address = address.wrapping_add(self.index_x as u16);
                self.push_micro_from_placeholder(new_address);
            }
            MicroOp::AddXtoZeroPageAddress(address) => {
                self.temp_addr = address.wrapping_add(self.index_x as u8) as u16;
                self.push_micro_from_placeholder(self.temp_addr);
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
                self.temp_addr = new_addr;
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::FetchHighAddrByteWithY => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                let new_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.temp_addr = new_addr;
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
                let new_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.push_micro_from_placeholder(new_addr);
            }
            MicroOp::LoadAccumulatorImmediate => {
                let value = self.memory[self.pc as usize];
                self.pc += 1;
                self.accumulator = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadAccumulatorFromAddress(address) => {
                let value = self.memory[address as usize];
                self.accumulator = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadXAccumulator => {
                let value = self.accumulator;
                self.index_x = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::IncrementX => {
                self.index_x = self.index_x.wrapping_add(1);

                self.set_flags_zero_neg(self.index_x);
            }
            MicroOp::DecrementX => {
                self.index_x = self.index_x.wrapping_sub(1);

                self.set_flags_zero_neg(self.index_x);
            }
            MicroOp::IncrementY => {
                self.index_y = self.index_y.wrapping_add(1);

                self.set_flags_zero_neg(self.index_y);
            }
            MicroOp::DecrementY => {
                self.index_y = self.index_y.wrapping_sub(1);

                self.set_flags_zero_neg(self.index_y);
            }
            MicroOp::WriteBackAndIncrement(value) => {
                self.mem_write(self.temp_addr, value);
                let updated_value = value.wrapping_add(1);
                self.push_micro_from_placeholder(updated_value as u16);
            }
            MicroOp::WriteBackAndDecrement(value) => {
                self.mem_write(self.temp_addr, value);
                let updated_value = value.wrapping_sub(1);
                self.push_micro_from_placeholder(updated_value as u16);
            }
            MicroOp::WriteToAddress(value) => {
                self.mem_write(self.temp_addr, value);

                self.set_flags_zero_neg(value);
            }
            MicroOp::Break => {
                //TODO: this op is more complex. research and implement.
                return;
            }
            MicroOp::DummyCycle => {
                return;
            }
            MicroOp::FixAddress(passthrough) => {
                self.push_micro_from_placeholder(passthrough);
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
        self.index_x = val
    }

    pub fn set_index_y(&mut self, val: u8) {
        self.index_y = val
    }
}

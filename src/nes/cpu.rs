use std::collections::VecDeque;
use std::io::{self, Write};

const CLS: &str = "\x1B[2J\x1B[1;1H";

const FLAG_ZERO: u8 = 0b0000_0010;
const FLAG_NEGATIVE: u8 = 0b1000_0000;
const FLAG_CARRY: u8 = 0b0000_0001;
const FLAG_OVERFLOW: u8 = 0b0100_0000;

enum AddressingMode {
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndexedIndirect,
    IndirectIndexed,
}

enum InstType {
    Read,
    RMW,
    Write,
}

#[derive(Debug)]
pub enum MicroOp {
    ExclusiveOr,
    ExclusiveOrAddress(u16),
    LogicalAnd,
    LogicalAndAddress(u16),
    InclusiveOr,
    InclusiveOrAddress(u16),
    BitTestPlaceholder,
    BitTest(u16),
    AddWithCarry,
    AddWithCarryAddress(u16),
    LoadAccPlaceholder,
    Break,
    ReadAccumulator,
    StoreAccumulator,
    StoreX,
    StoreY,
    LoadAccumulator,
    LoadAccumulatorFromAddress(u16),
    LoadX,
    LoadXfromAddress(u16),
    LoadY,
    LoadYfromAddress(u16),
    FetchLowAddrByte,
    FetchHighAddrByte,
    FetchHighAddrByteWithX,
    FetchHighAddrByteWithY,
    AddXtoAddressPlaceholder,
    AddXtoAddress(u16),
    AddXtoZeroPageAddressPlaceholder,
    AddXtoZeroPageAddress(u8),
    AddYtoZeroPageAddressPlaceholder,
    AddYtoZeroPageAddress(u8),
    AddXLoadImmediatePlaceholder,
    AddXLoadImmediate(u16),
    AddYLoadImmediatePlaceholder,
    AddYLoadImmediate(u16),
    FetchZeroPage,
    LoadXAccumulator,
    LoadYAccumulator,
    LoadXStackPointer,
    LoadAccumulatorX,
    LoadStackPointerX,
    LoadAccumulatorY,
    PushAccumulator,
    PushStatus,
    PullAccumulator,
    PullStatus,
    IncrementSP(u8),
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
    memory: Box<[u8; 0x10000]>,
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
            memory: Box::new([0u8; 0x10000]),
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

    pub fn mem_write(&mut self, pos: u16, byte: u8) {
        self.memory[pos as usize] = byte;
    }

    pub fn mem_write_u16(&mut self, pos: u16, byte: u16) {
        let low_byte = (byte & 0xFF) as u8;
        let high_byte = (byte >> 8) as u8;
        self.mem_write(pos, low_byte);
        self.mem_write(pos + 1, high_byte);
    }

    fn set_flags_zero_neg(&mut self, value: u8) {
        // set zero flag
        if value == 0x00 {
            self.status_p |= FLAG_ZERO;
        } else {
            self.status_p &= !FLAG_ZERO;
        }

        // set negative flag
        if value & 0b1000_0000 != 0 {
            self.status_p |= FLAG_NEGATIVE;
        } else {
            self.status_p &= !FLAG_NEGATIVE;
        }
    }

    fn dispatch_generic_instruction(
        address_mode: AddressingMode,
        inst: MicroOp,
        inst_type: InstType,
    ) -> VecDeque<MicroOp> {
        match address_mode {
            AddressingMode::ZeroPage => match inst_type {
                InstType::Read => VecDeque::from(vec![MicroOp::FetchZeroPage, inst]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![MicroOp::FetchZeroPage, inst]),
            },
            AddressingMode::ZeroPageX => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddressPlaceholder,
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddressPlaceholder,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddressPlaceholder,
                    inst,
                ]),
            },
            AddressingMode::ZeroPageY => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddYtoZeroPageAddressPlaceholder,
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddYtoZeroPageAddressPlaceholder,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddYtoZeroPageAddressPlaceholder,
                    inst,
                ]),
            },
            AddressingMode::Absolute => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByte,
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByte,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByte,
                    inst,
                ]),
            },
            AddressingMode::AbsoluteX => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithX, // might add dummy cycle
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithX,
                    MicroOp::FixAddressPlaceholder, // always happens with this instruction
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithX,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                ]),
            },
            AddressingMode::AbsoluteY => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithY, // might add dummy cycle
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithY,
                    MicroOp::FixAddressPlaceholder, // always happens with this instruction
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithY,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                ]),
            },
            AddressingMode::IndexedIndirect => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoPointerPlaceholder,
                    MicroOp::FetchPointerBytePlaceholder, // will be 2 ops after processed
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoPointerPlaceholder,
                    MicroOp::FetchPointerBytePlaceholder,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoPointerPlaceholder,
                    MicroOp::FetchPointerBytePlaceholder,
                    inst,
                ]),
            },
            AddressingMode::IndirectIndexed => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::FetchPointerByteWithYPlaceholder, // may add dummy cycle
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::FetchPointerByteWithYPlaceholder,
                    MicroOp::FixAddressPlaceholder,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::FetchPointerByteWithYPlaceholder,
                    MicroOp::ReadAddressPlaceholder,
                    inst,
                ]),
            },
        }
    }

    //TODO: might be redundant to have this and the self initializer. see load_program
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.index_x = 0;
        self.index_y = 0;
        self.sp = 0xFF; // TOP OF THE STACK! goes from 0x0100 to 0x01FF.
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
                VecDeque::from(vec![MicroOp::LoadAccumulator])
            }
            0xA5 => {
                // LDA zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LoadAccumulator,
                    InstType::Read,
                )
            }
            0xB5 => {
                // LDA zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LoadAccumulator,
                    InstType::Read,
                )
            }
            0xAD => {
                // LDA absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadAccumulator,
                    InstType::Read,
                )
            }
            0xBD => {
                // LDA absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LoadAccumulator,
                    InstType::Read,
                )
            }
            0xB9 => {
                // LDA absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LoadAccumulator,
                    InstType::Read,
                )
            }
            0xA1 => {
                // LDA indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::LoadAccumulator,
                    InstType::Read,
                )
            }
            0xB1 => {
                // LDA indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::LoadAccumulator,
                    InstType::Read,
                )
            }
            0xA2 => {
                // LDX
                VecDeque::from(vec![MicroOp::LoadX])
            }
            0xA6 => {
                // LDX zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LoadX,
                    InstType::Read,
                )
            }
            0xB6 => {
                // LDX zero page + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::LoadX,
                    InstType::Read,
                )
            }
            0xAE => {
                // LDX absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadX,
                    InstType::Read,
                )
            }
            0xBE => {
                // LDX absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LoadX,
                    InstType::Read,
                )
            }
            0xA0 => {
                // LDY immediate
                VecDeque::from(vec![MicroOp::LoadY])
            }
            0xA4 => {
                // LDY zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LoadY,
                    InstType::Read,
                )
            }
            0xB4 => {
                // LDY zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::LoadY,
                    InstType::Read,
                )
            }
            0xAC => {
                // LDY absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadY,
                    InstType::Read,
                )
            }
            0xBC => {
                // LDY absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LoadY,
                    InstType::Read,
                )
            }
            0x85 => {
                // STA zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                )
            }
            0x95 => {
                // STA zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                )
            }
            0x8D => {
                // STA absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                )
            }
            0x9D => {
                // STA absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                )
            }
            0x99 => {
                // STA absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                )
            }
            0x81 => {
                // STA indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                )
            }
            0x91 => {
                //STA indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                )
            }
            0x86 => {
                // STX zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::StoreX,
                    InstType::Write,
                )
            }
            0x96 => {
                // STX zero page + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::StoreX,
                    InstType::Write,
                )
            }
            0x8E => {
                // STX absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::StoreX,
                    InstType::Write,
                )
            }
            0x84 => {
                // STY zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::StoreY,
                    InstType::Write,
                )
            }
            0x94 => {
                // STY zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::StoreY,
                    InstType::Write,
                )
            }
            0x8C => {
                // STY absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::StoreY,
                    InstType::Write,
                )
            }
            0xAA => {
                // TAX
                VecDeque::from(vec![MicroOp::LoadXAccumulator])
            }
            0xA8 => {
                // TAY
                VecDeque::from(vec![MicroOp::LoadYAccumulator])
            }
            0xBA => {
                // TSX
                VecDeque::from(vec![MicroOp::LoadXStackPointer])
            }
            0x8A => {
                // TXA
                VecDeque::from(vec![MicroOp::LoadAccumulatorX])
            }
            0x9A => {
                // TXS
                VecDeque::from(vec![MicroOp::LoadStackPointerX])
            }
            0x98 => {
                // TYA
                VecDeque::from(vec![MicroOp::LoadAccumulatorY])
            }
            0x48 => {
                // PHA
                VecDeque::from(vec![
                    MicroOp::DummyCycle, // reads next inst byte, throws it away
                    MicroOp::PushAccumulator,
                ])
            }
            0x08 => {
                // PHP
                VecDeque::from(vec![MicroOp::DummyCycle, MicroOp::PushStatus])
            }
            0x68 => {
                // PLA
                VecDeque::from(vec![
                    MicroOp::DummyCycle,
                    MicroOp::IncrementSP(1),
                    MicroOp::PullAccumulator,
                ])
            }
            0x28 => {
                // PLP
                VecDeque::from(vec![
                    MicroOp::DummyCycle,
                    MicroOp::IncrementSP(1),
                    MicroOp::PullStatus,
                ])
            }
            0x29 => {
                // AND Immediate
                VecDeque::from(vec![MicroOp::LogicalAnd])
            }
            0x25 => {
                // AND zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LogicalAnd,
                    InstType::Read,
                )
            }
            0x35 => {
                // AND zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LogicalAnd,
                    InstType::Read,
                )
            }
            0x2D => {
                // AND absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LogicalAnd,
                    InstType::Read,
                )
            }
            0x3D => {
                // AND absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LogicalAnd,
                    InstType::Read,
                )
            }
            0x39 => {
                // AND absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LogicalAnd,
                    InstType::Read,
                )
            }
            0x21 => {
                // AND indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::LogicalAnd,
                    InstType::Read,
                )
            }
            0x31 => {
                // AND indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::LogicalAnd,
                    InstType::Read,
                )
            }
            0x49 => {
                // EOR immediate
                VecDeque::from(vec![MicroOp::ExclusiveOr])
            }
            0x45 => {
                // EOR zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::ExclusiveOr,
                    InstType::Read,
                )
            }
            0x55 => {
                // EOR zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::ExclusiveOr,
                    InstType::Read,
                )
            }
            0x4D => {
                // EOR absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::ExclusiveOr,
                    InstType::Read,
                )
            }
            0x5D => {
                // EOR absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::ExclusiveOr,
                    InstType::Read,
                )
            }
            0x59 => {
                // EOR absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::ExclusiveOr,
                    InstType::Read,
                )
            }
            0x41 => {
                // EOR indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::ExclusiveOr,
                    InstType::Read,
                )
            }
            0x51 => {
                // EOR indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::ExclusiveOr,
                    InstType::Read,
                )
            }
            0x09 => {
                // ORA immediate
                VecDeque::from(vec![MicroOp::InclusiveOr])
            }
            0x05 => {
                // ORA zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::InclusiveOr,
                    InstType::Read,
                )
            }
            0x15 => {
                // ORA zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::InclusiveOr,
                    InstType::Read,
                )
            }
            0x0D => {
                // ORA absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::InclusiveOr,
                    InstType::Read,
                )
            }
            0x1D => {
                // ORA absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::InclusiveOr,
                    InstType::Read,
                )
            }
            0x19 => {
                // ORA absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::InclusiveOr,
                    InstType::Read,
                )
            }
            0x01 => {
                // ORA indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::InclusiveOr,
                    InstType::Read,
                )
            }
            0x11 => {
                // ORA indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::InclusiveOr,
                    InstType::Read,
                )
            }
            0x24 => {
                // BIT zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::BitTestPlaceholder,
                    InstType::Read,
                )
            }
            0x2C => {
                // BIT absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::BitTestPlaceholder,
                    InstType::Read,
                )
            }
            0x69 => {
                // ADC
                VecDeque::from(vec![MicroOp::AddWithCarry])
            }
            0x65 => {
                // ADC zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::AddWithCarry,
                    InstType::Read,
                )
            }
            0x75 => {
                // ADC zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::AddWithCarry,
                    InstType::Read,
                )
            }
            0x6D => {
                // ADC absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::AddWithCarry,
                    InstType::Read,
                )
            }
            0x7D => {
                // ADC absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::AddWithCarry,
                    InstType::Read,
                )
            }
            0x79 => {
                // ADC absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::AddWithCarry,
                    InstType::Read,
                )
            }
            0x61 => {
                // ADC indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::AddWithCarry,
                    InstType::Read,
                )
            }
            0x71 => {
                // ADC indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::AddWithCarry,
                    InstType::Read,
                )
            }
            0xE6 => {
                // INC zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    InstType::RMW,
                )
            }
            0xF6 => {
                // INC zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    InstType::RMW,
                )
            }
            0xEE => {
                // INC absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    InstType::RMW,
                )
            }
            0xFE => {
                // INC absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::WriteBackAndIncrementPlaceholder,
                    InstType::RMW,
                )
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
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    InstType::RMW,
                )
            }
            0xD6 => {
                // DEC zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    InstType::RMW,
                )
            }
            0xCE => {
                // DEC absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    InstType::RMW,
                )
            }
            0xDE => {
                // DEC absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::WriteBackAndDecrementPlaceholder,
                    InstType::RMW,
                )
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
            Some(MicroOp::StoreX) => {
                self.current_inst.push_front(MicroOp::StoreX);
            }
            Some(MicroOp::StoreY) => {
                self.current_inst.push_front(MicroOp::StoreY);
            }
            Some(MicroOp::StoreAccumulator) => {
                self.current_inst.push_front(MicroOp::StoreAccumulator);
            }
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
            Some(MicroOp::LoadAccumulator) => {
                self.current_inst
                    .push_front(MicroOp::LoadAccumulatorFromAddress(value));
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::LogicalAnd) => {
                self.current_inst
                    .push_front(MicroOp::LogicalAndAddress(value));
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::ExclusiveOr) => {
                self.current_inst
                    .push_front(MicroOp::ExclusiveOrAddress(value));
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::InclusiveOr) => {
                self.current_inst
                    .push_front(MicroOp::InclusiveOrAddress(value));
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::BitTestPlaceholder) => {
                self.current_inst.push_front(MicroOp::BitTest(value));
            }
            Some(MicroOp::AddWithCarry) => {
                self.current_inst
                    .push_front(MicroOp::AddWithCarryAddress(value));
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::LoadX) => {
                self.current_inst
                    .push_front(MicroOp::LoadXfromAddress(value));
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::LoadY) => {
                self.current_inst
                    .push_front(MicroOp::LoadYfromAddress(value));
                if self.page_crossed {
                    self.page_crossed = false;
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
            Some(MicroOp::AddYtoZeroPageAddressPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::AddYtoZeroPageAddress(value as u8));
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
                self.temp_addr = address.wrapping_add(self.index_x as u16);
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::AddXtoZeroPageAddress(address) => {
                self.temp_addr = address.wrapping_add(self.index_x as u8) as u16;
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::AddYtoZeroPageAddress(address) => {
                self.temp_addr = address.wrapping_add(self.index_y as u8) as u16;
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
                self.temp_addr = new_addr;
                self.push_micro_from_placeholder(self.temp_addr);
            }
            MicroOp::LoadAccumulator => {
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
            MicroOp::LoadX => {
                let value = self.memory[self.pc as usize];
                self.pc += 1;
                self.index_x = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadXfromAddress(address) => {
                let value = self.memory[address as usize];
                self.index_x = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadY => {
                let value = self.memory[self.pc as usize];
                self.pc += 1;
                self.index_y = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadYfromAddress(address) => {
                let value = self.memory[address as usize];
                self.index_y = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadXAccumulator => {
                self.index_x = self.accumulator;

                self.set_flags_zero_neg(self.index_x);
            }
            MicroOp::LoadYAccumulator => {
                self.index_y = self.accumulator;

                self.set_flags_zero_neg(self.index_y);
            }
            MicroOp::LoadXStackPointer => {
                self.index_x = self.sp;
                self.set_flags_zero_neg(self.index_x);
            }
            MicroOp::LoadAccumulatorX => {
                self.accumulator = self.index_x;
                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::LoadAccumulatorY => {
                self.accumulator = self.index_y;
                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::LoadStackPointerX => {
                self.sp = self.index_x;
            }
            MicroOp::PushAccumulator => {
                let address: u16 = 0x0100 + self.sp as u16;
                self.mem_write(address, self.accumulator);
                self.sp = self.sp.wrapping_sub(1);
            }
            MicroOp::PushStatus => {
                let address: u16 = 0x0100 + self.sp as u16;
                self.mem_write(address, self.status_p);
                self.sp = self.sp.wrapping_sub(1);
            }
            MicroOp::IncrementSP(value) => {
                self.sp = self.sp.wrapping_add(value);
            }
            MicroOp::PullAccumulator => {
                let address: u16 = 0x0100 + self.sp as u16;
                self.accumulator = self.mem_read(address);

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::PullStatus => {
                let address: u16 = 0x0100 + self.sp as u16;
                self.status_p = self.mem_read(address);
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
            MicroOp::StoreAccumulator => {
                self.mem_write(self.temp_addr, self.accumulator);
            }
            MicroOp::StoreX => {
                self.mem_write(self.temp_addr, self.index_x);
            }
            MicroOp::StoreY => {
                self.mem_write(self.temp_addr, self.index_y);
            }
            MicroOp::LogicalAnd => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.accumulator &= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::LogicalAndAddress(address) => {
                let value = self.mem_read(address);
                self.accumulator &= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::ExclusiveOr => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.accumulator ^= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::ExclusiveOrAddress(address) => {
                let value = self.mem_read(address);
                self.accumulator ^= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::InclusiveOr => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.accumulator |= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::InclusiveOrAddress(address) => {
                let value = self.mem_read(address);
                self.accumulator |= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::BitTest(address) => {
                let value = self.mem_read(address);
                let temp = value & self.accumulator;

                // set zero flag
                if temp == 0x00 {
                    self.status_p |= FLAG_ZERO;
                } else {
                    self.status_p &= !FLAG_ZERO;
                }

                self.status_p = self.status_p & !(0b1100_0000); // clear neg and overflow flags
                self.status_p |= value & 0b1100_0000;
            }
            MicroOp::AddWithCarry => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                let carry_in: u16 = if self.status_p & FLAG_CARRY != 0 {
                    1
                } else {
                    0
                };

                let sum = self.accumulator as u16 + value as u16 + carry_in;
                let result = sum as u8;
                if sum > 0xFF {
                    self.status_p |= FLAG_CARRY;
                } else {
                    self.status_p &= !FLAG_CARRY;
                }

                self.set_flags_zero_neg(result);

                if ((self.accumulator ^ result) & (value ^ result) & 0x80) != 0 {
                    self.status_p |= FLAG_OVERFLOW;
                } else {
                    self.status_p &= !FLAG_OVERFLOW;
                }

                self.accumulator = result;
            }
            MicroOp::AddWithCarryAddress(address) => {
                let value = self.mem_read(address);
                let carry_in = if self.status_p & FLAG_CARRY != 0 {
                    1
                } else {
                    0
                };
                let sum = self.accumulator as u16 + value as u16 + carry_in;
                let result = sum as u8;
                if sum > 0xFF {
                    self.status_p |= FLAG_CARRY;
                } else {
                    self.status_p &= !FLAG_CARRY;
                }
                self.set_flags_zero_neg(result);
                if ((self.accumulator ^ result) & (value ^ result) & 0x80) != 0 {
                    self.status_p |= FLAG_OVERFLOW;
                } else {
                    self.status_p &= !FLAG_OVERFLOW;
                }
                self.accumulator = result;
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

    pub fn get_memory(&self) -> &[u8; 0x10000] {
        &self.memory
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

    pub fn set_index_y(&mut self, val: u8) {
        self.index_y = val;
    }

    pub fn set_status_p(&mut self, val: u8) {
        self.status_p = val;
    }

    pub fn set_sp(&mut self, val: u8) {
        self.sp = val;
    }
}

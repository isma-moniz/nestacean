use std::collections::VecDeque;
use std::io::{self, Write};

const CLS: &str = "\x1B[2J\x1B[1;1H";

const FLAG_ZERO: u8 = 0b0000_0010;
const FLAG_NEGATIVE: u8 = 0b1000_0000;
const FLAG_CARRY: u8 = 0b0000_0001;
const FLAG_OVERFLOW: u8 = 0b0100_0000;
const FLAG_DECIMAL: u8 = 0b0000_1000;
const FLAG_INTERRUPT: u8 = 0b0000_0100;
const FLAG_BREAK: u8 = 0b0001_0000;
const BIT_7: u8 = 0b1000_0000;
const STACK_PTR_TOP: u8 = 0xFF;
const STACK_BOTTOM: u16 = 0x0100;
const PROGRAM_START: u16 = 0x8000;
const PC_INIT_LOCATION: u16 = 0xFFFC;
const INTERRUPT_VEC_LOW: u16 = 0xFFFE;
const INTERRUPT_VEC_HIGH: u16 = 0xFFFF;

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
    TakeBranch(u8),
    ExclusiveOr,
    ExclusiveOrAddress,
    LogicalAnd,
    LogicalAndAddress,
    InclusiveOr,
    InclusiveOrAddress,
    BitTestAddress,
    AddWithCarry,
    AddWithCarryAddress,
    SubWithCarry,
    SubWithCarryAddress,
    Compare,
    CompareAddress,
    CompareX,
    CompareXAddress,
    CompareY,
    CompareYAddress,
    ArithmeticShiftLeft,
    ArithmeticShiftLeftAddress(Option<u16>),
    LogicalShiftRight,
    LogicalShiftRightAddress(Option<u16>),
    RotateLeft,
    RotateLeftAddress(Option<u16>),
    RotateRight,
    RotateRightAddress(Option<u16>),
    LoadAccPlaceholder,
    Break,
    ReadAccumulator,
    StoreAccumulator,
    StoreX,
    StoreY,
    LoadAccumulator,
    LoadAccumulatorFromAddress,
    LoadX,
    LoadXfromAddress,
    LoadY,
    LoadYfromAddress,
    FetchLowAddrByte,
    FetchHighAddrByte,
    FetchInterruptLow,
    FetchInterruptHigh,
    CopyLowFetchHightoPC,
    FetchHighAddrByteWithX,
    FetchHighAddrByteWithY,
    AddXtoZeroPageAddress,
    AddYtoZeroPageAddress,
    AddXLoadImmediatePlaceholder,
    AddXLoadImmediate(u16),
    AddYLoadImmediatePlaceholder,
    AddYLoadImmediate(u16),
    FetchZeroPage,
    FetchRelativeOffset(u8, u8),
    LoadXAccumulator,
    LoadYAccumulator,
    LoadXStackPointer,
    LoadAccumulatorX,
    LoadStackPointerX,
    LoadAccumulatorY,
    PushAccumulator,
    PushStatusBrkPhp,
    PullAccumulator,
    PullStatus,
    PushPCH,
    PushPCL,
    PullPCL,
    PullPCHPlaceholder,
    PullPCH(Option<u16>),
    IncrementPCPlaceholder,
    IncrementPC(Option<u16>),
    IncrementSP(u8),
    IncrementX,
    IncrementY,
    DecrementX,
    DecrementY,
    DummyCycle,
    FixAddressPlaceholder, // just a dummy cycle but with passthrough of the provided value
    FixAddress(Option<u16>),
    AddXtoPointer,
    FetchPointerHighBytePlaceholder,
    FetchPointerHighByteWithYPlaceholder,
    FetchPointerLowByte,
    FetchPointerHighByte(Option<u16>),
    FetchPointerHighByteWithY(Option<u16>),
    ReadHighFromIndirectPlaceholder,
    ReadHighFromIndirectLatch(Option<u16>),
    ReadLowFromIndirect,
    ReadAddress,
    WriteBackAndIncrementPlaceholder,
    WriteBackAndIncrement(Option<u16>),
    WriteBackAndDecrementPlaceholder,
    WriteBackAndDecrement(Option<u16>),
    WriteToAddressPlaceholder,
    WriteToAddress(Option<u16>),
    SetCarry,
    ClearCarry,
    ClearDecimalMode,
    SetDecimalMode,
    ClearInterrupt,
    SetInterrupt,
    ClearOverflow,
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

    pub fn mem_write_u16(&mut self, pos: u16, bytes: u16) {
        let low_byte = (bytes & 0xFF) as u8;
        let high_byte = (bytes >> 8) as u8;
        self.mem_write(pos, low_byte);
        self.mem_write(pos + 1, high_byte);
    }

    fn compare(&mut self, a: u8, b: u8) {
        let result = a.wrapping_sub(b);
        self.set_flags_zero_neg(result);
        if a >= b {
            self.status_p |= FLAG_CARRY;
        } else {
            self.status_p &= !FLAG_CARRY;
        }
    }

    fn swc(&mut self, value: u8) {
        let carry_in: u16 = if self.status_p & FLAG_CARRY != 0 {
            1
        } else {
            0
        };
        let sub = self.accumulator as u16 - value as u16 - !carry_in;
        let result = sub as u8;
        self.set_flags_zero_neg(result);
        if sub > 0xFF {
            self.status_p &= !FLAG_CARRY;
        } else {
            self.status_p |= FLAG_CARRY;
        }
        if ((self.accumulator ^ result) & (value ^ result) & 0x80) != 0 {
            self.status_p |= FLAG_OVERFLOW;
        } else {
            self.status_p &= !FLAG_OVERFLOW;
        }
        self.accumulator = result;
    }

    fn awc(&mut self, value: u8) {
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

    fn asl(&mut self, value: u8) -> u8 {
        if value & FLAG_NEGATIVE != 0 {
            self.status_p |= FLAG_CARRY;
        } else {
            self.status_p &= !FLAG_CARRY;
        }
        let result = value << 1;
        self.set_flags_zero_neg(result);
        result
    }

    fn lsr(&mut self, value: u8) -> u8 {
        if value & FLAG_CARRY != 0 {
            self.status_p |= FLAG_CARRY;
        } else {
            self.status_p &= !FLAG_CARRY;
        }
        let result = value >> 1;
        self.set_flags_zero_neg(result);
        result
    }

    fn rol(&mut self, value: u8) -> u8 {
        let carry = self.status_p & FLAG_CARRY;
        let result = (value << 1) | carry;
        if value & FLAG_NEGATIVE != 0 {
            self.status_p |= FLAG_CARRY;
        } else {
            self.status_p &= !FLAG_CARRY;
        }
        self.set_flags_zero_neg(result);
        result
    }

    fn ror(&mut self, value: u8) -> u8 {
        let carry = self.status_p & FLAG_CARRY;
        let result = (value >> 1) | (carry << 7);
        if value & FLAG_CARRY != 0 {
            self.status_p |= FLAG_CARRY;
        } else {
            self.status_p &= !FLAG_CARRY;
        }
        self.set_flags_zero_neg(result);
        result
    }

    fn schedule_branch(&mut self, value: u8, cond: u8, offset: u8) {
        if value == cond {
            self.current_inst.push_back(MicroOp::TakeBranch(offset));
        }
    }

    fn set_flags_zero_neg(&mut self, value: u8) {
        // set zero flag
        if value == 0x00 {
            self.status_p |= FLAG_ZERO;
        } else {
            self.status_p &= !FLAG_ZERO;
        }

        // set negative flag
        if value & BIT_7 != 0 {
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
                    MicroOp::ReadAddress,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![MicroOp::FetchZeroPage, inst]),
            },
            AddressingMode::ZeroPageX => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddress,
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddress,
                    MicroOp::ReadAddress,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoZeroPageAddress,
                    inst,
                ]),
            },
            AddressingMode::ZeroPageY => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddYtoZeroPageAddress,
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddYtoZeroPageAddress,
                    MicroOp::ReadAddress,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddYtoZeroPageAddress,
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
                    MicroOp::ReadAddress,
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
                    MicroOp::DummyCycle, // always happens with this instruction, "fixing the
                    // address"
                    MicroOp::ReadAddress,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithX,
                    MicroOp::ReadAddress,
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
                    MicroOp::DummyCycle, // always happens with this instruction
                    MicroOp::ReadAddress,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByteWithY,
                    MicroOp::ReadAddress,
                    inst,
                ]),
            },
            AddressingMode::IndexedIndirect => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoPointer,
                    MicroOp::FetchPointerLowByte,
                    MicroOp::FetchPointerHighBytePlaceholder,
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoPointer,
                    MicroOp::FetchPointerLowByte,
                    MicroOp::FetchPointerHighBytePlaceholder,
                    MicroOp::ReadAddress,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::AddXtoPointer,
                    MicroOp::FetchPointerLowByte,
                    MicroOp::FetchPointerHighBytePlaceholder,
                    inst,
                ]),
            },
            AddressingMode::IndirectIndexed => match inst_type {
                InstType::Read => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::FetchPointerLowByte,
                    MicroOp::FetchPointerHighByteWithYPlaceholder, // may add dummy cycle
                    inst,
                ]),
                InstType::RMW => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::FetchPointerLowByte,
                    MicroOp::FetchPointerHighByteWithYPlaceholder,
                    MicroOp::FixAddressPlaceholder,
                    MicroOp::ReadAddress,
                    inst,
                    MicroOp::WriteToAddressPlaceholder,
                ]),
                InstType::Write => VecDeque::from(vec![
                    MicroOp::FetchZeroPage,
                    MicroOp::FetchPointerLowByte,
                    MicroOp::FetchPointerHighByteWithYPlaceholder,
                    MicroOp::ReadAddress,
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
        self.sp = STACK_PTR_TOP;
        self.status_p = 0;
        self.temp_addr = 0;
        self.page_crossed = false;
        self.current_inst = VecDeque::new();
        self.pc = self.mem_read_u16(PC_INIT_LOCATION);
    }

    pub fn load_program(&mut self, program: &[u8]) {
        self.memory[PROGRAM_START as usize..(PROGRAM_START as usize + program.len())]
            .copy_from_slice(&program[..]);
        self.mem_write_u16(PC_INIT_LOCATION, PROGRAM_START);
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
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xB5 => {
                // LDA zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xAD => {
                // LDA absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xBD => {
                // LDA absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xB9 => {
                // LDA absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xA1 => {
                // LDA indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xB1 => {
                // LDA indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::LoadAccumulatorFromAddress,
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
                    MicroOp::LoadXfromAddress,
                    InstType::Read,
                )
            }
            0xB6 => {
                // LDX zero page + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::LoadXfromAddress,
                    InstType::Read,
                )
            }
            0xAE => {
                // LDX absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadXfromAddress,
                    InstType::Read,
                )
            }
            0xBE => {
                // LDX absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LoadXfromAddress,
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
                    MicroOp::LoadYfromAddress,
                    InstType::Read,
                )
            }
            0xB4 => {
                // LDY zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::LoadYfromAddress,
                    InstType::Read,
                )
            }
            0xAC => {
                // LDY absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadYfromAddress,
                    InstType::Read,
                )
            }
            0xBC => {
                // LDY absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LoadYfromAddress,
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
                VecDeque::from(vec![MicroOp::DummyCycle, MicroOp::PushStatusBrkPhp])
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
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                )
            }
            0x35 => {
                // AND zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                )
            }
            0x2D => {
                // AND absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                )
            }
            0x3D => {
                // AND absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                )
            }
            0x39 => {
                // AND absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                )
            }
            0x21 => {
                // AND indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                )
            }
            0x31 => {
                // AND indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::LogicalAndAddress,
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
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x55 => {
                // EOR zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x4D => {
                // EOR absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x5D => {
                // EOR absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x59 => {
                // EOR absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x41 => {
                // EOR indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x51 => {
                // EOR indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::ExclusiveOrAddress,
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
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x15 => {
                // ORA zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x0D => {
                // ORA absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x1D => {
                // ORA absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x19 => {
                // ORA absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x01 => {
                // ORA indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x11 => {
                // ORA indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                )
            }
            0x24 => {
                // BIT zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::BitTestAddress,
                    InstType::Read,
                )
            }
            0x2C => {
                // BIT absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::BitTestAddress,
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
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                )
            }
            0x75 => {
                // ADC zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                )
            }
            0x6D => {
                // ADC absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                )
            }
            0x7D => {
                // ADC absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                )
            }
            0x79 => {
                // ADC absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                )
            }
            0x61 => {
                // ADC indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                )
            }
            0x71 => {
                // ADC indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                )
            }
            0xE9 => {
                // SBC
                VecDeque::from(vec![MicroOp::SubWithCarry])
            }
            0xE5 => {
                // SBC zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                )
            }
            0xF5 => {
                // SBC zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                )
            }
            0xED => {
                // SBC absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                )
            }
            0xFD => {
                // SBC absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                )
            }
            0xF9 => {
                // SBC absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                )
            }
            0xE1 => {
                // SBC indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                )
            }
            0xF1 => {
                // SBC indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                )
            }
            0xC9 => {
                // CMP
                VecDeque::from(vec![MicroOp::Compare])
            }
            0xC5 => {
                // CMP zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::Compare,
                    InstType::Read,
                )
            }
            0xD5 => {
                // CMP zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::Compare,
                    InstType::Read,
                )
            }
            0xCD => {
                // CMP absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::Compare,
                    InstType::Read,
                )
            }
            0xDD => {
                // CMP absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::Compare,
                    InstType::Read,
                )
            }
            0xD9 => {
                // CMP absolute + y
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::Compare,
                    InstType::Read,
                )
            }
            0xC1 => {
                // CMP indexed indirect
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::Compare,
                    InstType::Read,
                )
            }
            0xD1 => {
                // CMP indirect indexed
                Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::Compare,
                    InstType::Read,
                )
            }
            0xE0 => {
                // CPX
                VecDeque::from(vec![MicroOp::CompareX])
            }
            0xE4 => {
                // CPX zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::CompareXAddress,
                    InstType::Read,
                )
            }
            0xEC => {
                // CPX absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::CompareXAddress,
                    InstType::Read,
                )
            }
            0xC0 => {
                // CPY
                VecDeque::from(vec![MicroOp::CompareY])
            }
            0xC4 => {
                // CPY zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::CompareYAddress,
                    InstType::Read,
                )
            }
            0xCC => {
                // CPY absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::CompareYAddress,
                    InstType::Read,
                )
            }
            0x0A => {
                // ASL
                VecDeque::from(vec![MicroOp::ArithmeticShiftLeft])
            }
            0x06 => {
                // ASL zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::ArithmeticShiftLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x16 => {
                // ASL zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::ArithmeticShiftLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x0E => {
                // ASL absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::ArithmeticShiftLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x1E => {
                // ASL absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::ArithmeticShiftLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x4A => {
                // LSR
                VecDeque::from(vec![MicroOp::LogicalShiftRight])
            }
            0x46 => {
                // LSR zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LogicalShiftRightAddress(None),
                    InstType::RMW,
                )
            }
            0x56 => {
                // LSR zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LogicalShiftRightAddress(None),
                    InstType::RMW,
                )
            }
            0x4E => {
                // LSR absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LogicalShiftRightAddress(None),
                    InstType::RMW,
                )
            }
            0x5E => {
                // LSR absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LogicalShiftRightAddress(None),
                    InstType::RMW,
                )
            }
            0x2A => {
                // ROL
                VecDeque::from(vec![MicroOp::RotateLeft])
            }
            0x26 => {
                // ROL zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::RotateLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x36 => {
                // ROL zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::RotateLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x2E => {
                // ROL absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::RotateLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x3E => {
                // ROL absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::RotateLeftAddress(None),
                    InstType::RMW,
                )
            }
            0x6A => {
                // ROR
                VecDeque::from(vec![MicroOp::RotateRight])
            }
            0x66 => {
                // ROR zero page
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::RotateRightAddress(None),
                    InstType::RMW,
                )
            }
            0x76 => {
                // ROR zero page + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::RotateRightAddress(None),
                    InstType::RMW,
                )
            }
            0x6E => {
                // ROR absolute
                Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::RotateRightAddress(None),
                    InstType::RMW,
                )
            }
            0x7E => {
                // ROR absolute + x
                Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::RotateRightAddress(None),
                    InstType::RMW,
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
            0x4C => {
                // JMP absolute
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::CopyLowFetchHightoPC,
                ])
            }
            0x6C => {
                // JMP indirect
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::FetchHighAddrByte,
                    MicroOp::ReadLowFromIndirect,
                    MicroOp::ReadHighFromIndirectPlaceholder,
                ])
            }
            0x20 => {
                // JSR
                VecDeque::from(vec![
                    MicroOp::FetchLowAddrByte,
                    MicroOp::DummyCycle, //TODO: this isn't actually performing a dummy read. see
                    //if it brings problems.
                    MicroOp::PushPCH,
                    MicroOp::PushPCL,
                    MicroOp::CopyLowFetchHightoPC,
                ])
            }
            0x60 => {
                // RTS
                VecDeque::from(vec![
                    MicroOp::DummyCycle,
                    MicroOp::IncrementSP(1),
                    MicroOp::PullPCL,
                    MicroOp::PullPCHPlaceholder,
                    MicroOp::IncrementPCPlaceholder,
                ])
            }
            0x90 => {
                // BCC
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_CARRY,
                    0x00,
                )])
            }
            0xB0 => {
                // BCS
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_CARRY,
                    FLAG_CARRY,
                )])
            }
            0xF0 => {
                // BEQ
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_ZERO,
                    FLAG_ZERO,
                )])
            }
            0xD0 => {
                // BNE
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_ZERO,
                    0x00,
                )])
            }
            0x30 => {
                // BMI
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_NEGATIVE,
                    FLAG_NEGATIVE,
                )])
            }
            0x10 => {
                // BPL
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_NEGATIVE,
                    0x00,
                )])
            }
            0x50 => {
                // BVC
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_OVERFLOW,
                    0x00,
                )])
            }
            0x70 => {
                // BVS
                VecDeque::from(vec![MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_OVERFLOW,
                    FLAG_OVERFLOW,
                )])
            }
            0x18 => {
                // CLC
                VecDeque::from(vec![MicroOp::ClearCarry])
            }
            0x38 => {
                // SEC
                VecDeque::from(vec![MicroOp::SetCarry])
            }
            0xD8 => {
                // CLD
                VecDeque::from(vec![MicroOp::ClearDecimalMode])
            }
            0xF8 => {
                // SED
                VecDeque::from(vec![MicroOp::SetDecimalMode])
            }
            0x78 => {
                // SEI
                VecDeque::from(vec![MicroOp::SetInterrupt])
            }
            0x58 => {
                // CLI
                VecDeque::from(vec![MicroOp::ClearInterrupt])
            }
            0xB8 => {
                // CLV
                VecDeque::from(vec![MicroOp::ClearOverflow])
            }
            0xEA => {
                // NOP
                VecDeque::from(vec![MicroOp::DummyCycle])
            }
            0x00 => {
                // BRK
                VecDeque::from(vec![
                    MicroOp::IncrementPC(Some(self.pc)),
                    MicroOp::PushPCH,
                    MicroOp::PushPCL,
                    MicroOp::PushStatusBrkPhp,
                    MicroOp::FetchInterruptLow,
                    MicroOp::FetchInterruptHigh,
                ])
            }
            0x40 => {
                // RTI
                VecDeque::from(vec![
                    MicroOp::DummyCycle,
                    MicroOp::IncrementSP(1),
                    MicroOp::PullStatus,
                    MicroOp::PullPCL,
                    MicroOp::PullPCHPlaceholder,
                ])
            }
            _ => unimplemented!("{}", opcode),
        }
    }

    fn push_micro_from_placeholder(&mut self, value: Option<u16>) {
        match self.current_inst.pop_front() {
            Some(MicroOp::WriteToAddressPlaceholder) => {
                self.current_inst.push_front(MicroOp::WriteToAddress(value));
            }
            Some(MicroOp::WriteBackAndIncrementPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::WriteBackAndIncrement(value));
            }
            Some(MicroOp::WriteBackAndDecrementPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::WriteBackAndDecrement(value));
            }
            Some(MicroOp::ReadHighFromIndirectPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::ReadHighFromIndirectLatch(value));
            }
            Some(MicroOp::LoadAccumulatorFromAddress) => {
                self.current_inst
                    .push_front(MicroOp::LoadAccumulatorFromAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::LogicalAndAddress) => {
                self.current_inst.push_front(MicroOp::LogicalAndAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::ExclusiveOrAddress) => {
                self.current_inst.push_front(MicroOp::ExclusiveOrAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::InclusiveOrAddress) => {
                self.current_inst.push_front(MicroOp::InclusiveOrAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::AddWithCarryAddress) => {
                self.current_inst.push_front(MicroOp::AddWithCarryAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::SubWithCarryAddress) => {
                self.current_inst.push_front(MicroOp::SubWithCarryAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::CompareAddress) => {
                self.current_inst.push_front(MicroOp::CompareAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::CompareXAddress) => {
                self.current_inst.push_front(MicroOp::CompareXAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::CompareYAddress) => {
                self.current_inst.push_front(MicroOp::CompareYAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::ArithmeticShiftLeftAddress(None)) => {
                self.current_inst
                    .push_front(MicroOp::ArithmeticShiftLeftAddress(value));
            }
            Some(MicroOp::LogicalShiftRightAddress(None)) => {
                self.current_inst
                    .push_front(MicroOp::LogicalShiftRightAddress(value));
            }
            Some(MicroOp::RotateLeftAddress(None)) => {
                self.current_inst
                    .push_front(MicroOp::RotateLeftAddress(value));
            }
            Some(MicroOp::RotateRightAddress(None)) => {
                self.current_inst
                    .push_front(MicroOp::RotateRightAddress(value));
            }
            Some(MicroOp::LoadXfromAddress) => {
                self.current_inst.push_front(MicroOp::LoadXfromAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::LoadYfromAddress) => {
                self.current_inst.push_front(MicroOp::LoadYfromAddress);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle);
                }
            }
            Some(MicroOp::FetchPointerHighBytePlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::FetchPointerHighByte(value));
            }
            Some(MicroOp::FetchPointerHighByteWithYPlaceholder) => {
                self.current_inst
                    .push_front(MicroOp::FetchPointerHighByteWithY(value));
            }
            Some(MicroOp::FixAddressPlaceholder) => {
                // TODO: remove if value not used
                self.current_inst.push_front(MicroOp::FixAddress(value));
            }
            Some(MicroOp::PullPCHPlaceholder) => {
                self.current_inst.push_front(MicroOp::PullPCH(value));
            }
            Some(MicroOp::IncrementPCPlaceholder) => {
                self.current_inst.push_front(MicroOp::IncrementPC(value));
            }
            Some(MicroOp::DummyCycle) => {
                self.current_inst.push_front(MicroOp::DummyCycle);
            }
            Some(MicroOp::ReadAddress) => {
                self.current_inst.push_front(MicroOp::ReadAddress);
            }
            Some(other) => panic!("Unexpected micro-op: {:?}", other),
            None => return,
        }
    }

    fn execute_micro_op(&mut self, operation: MicroOp) {
        match operation {
            MicroOp::ReadAddress => {
                let value = self.mem_read(self.temp_addr);

                self.push_micro_from_placeholder(Some(value as u16));
            }
            MicroOp::FetchZeroPage => {
                self.temp_addr = self.memory[self.pc as usize] as u16;
                self.pc += 1;
            }
            MicroOp::AddXtoZeroPageAddress => {
                let address = self.temp_addr as u8;
                self.temp_addr = address.wrapping_add(self.index_x as u8) as u16;
            }
            MicroOp::AddYtoZeroPageAddress => {
                let address = self.temp_addr as u8;
                self.temp_addr = address.wrapping_add(self.index_y as u8) as u16;
            }
            MicroOp::AddXtoPointer => {
                let pointer = self.temp_addr as u8;
                self.temp_addr = pointer.wrapping_add(self.index_x) as u16;
            }
            MicroOp::FetchLowAddrByte => {
                self.temp_addr = self.mem_read(self.pc) as u16;
                self.pc += 1;
            }
            MicroOp::FetchHighAddrByte => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
            }
            MicroOp::FetchInterruptLow => {
                self.pc = self.mem_read(INTERRUPT_VEC_LOW) as u16;
            }
            MicroOp::FetchInterruptHigh => {
                self.pc |= (self.mem_read(INTERRUPT_VEC_HIGH) as u16) << 8;
            }
            MicroOp::CopyLowFetchHightoPC => {
                let high_byte = (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                self.pc = high_byte | self.temp_addr;
            }
            MicroOp::ReadLowFromIndirect => {
                let latch = self.mem_read(self.temp_addr);
                self.push_micro_from_placeholder(Some(latch as u16));
            }
            MicroOp::ReadHighFromIndirectLatch(latch) => {
                match latch {
                    Some(latch) => {
                        // simulates 6502 page wraparound bug
                        let high_addr = if latch == 0xFF {
                            self.temp_addr & 0xFF00
                        } else {
                            self.temp_addr + 1
                        };
                        let high_byte = (self.mem_read(high_addr) as u16) << 8;
                        self.pc = high_byte | latch as u16;
                    }
                    None => panic!("PC latch empty in ReadHighFromIndirectLatch!"),
                }
            }
            MicroOp::FetchHighAddrByteWithX => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                let new_addr = self.temp_addr.wrapping_add(self.index_x as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.temp_addr = new_addr;
                self.push_micro_from_placeholder(None);
            }
            MicroOp::FetchHighAddrByteWithY => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                let new_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.temp_addr = new_addr;
                self.push_micro_from_placeholder(None);
            }
            MicroOp::FetchPointerLowByte => {
                let pointer = self.temp_addr;
                self.temp_addr = self.mem_read(pointer) as u16;
                self.push_micro_from_placeholder(Some(pointer));
            }
            MicroOp::FetchPointerHighByte(pointer) => match pointer {
                Some(pointer) => {
                    self.temp_addr |= (self.mem_read(pointer.wrapping_add(1)) as u16) << 8;
                }
                None => panic!("Expected pointer value in FetchPointerHighByte"),
            },
            MicroOp::FetchPointerHighByteWithY(pointer) => match pointer {
                Some(pointer) => {
                    self.temp_addr |= (self.mem_read(pointer.wrapping_add(1)) as u16) << 8;
                    let new_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                    self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                    self.temp_addr = new_addr;
                    self.push_micro_from_placeholder(None);
                }
                None => panic!("Expected pointer value in FetchPointerHighByteWithY"),
            },
            MicroOp::FetchRelativeOffset(value, cond) => {
                let offset = self.mem_read(self.pc);
                self.pc += 1;
                self.schedule_branch(value, cond, offset);
            }
            MicroOp::TakeBranch(offset) => {
                let new_addr = self.pc.wrapping_add(offset as u16);
                self.page_crossed = (self.pc & 0xFF00) != (new_addr & 0xFF00);
                if self.page_crossed {
                    self.page_crossed = false;
                    self.current_inst.push_front(MicroOp::DummyCycle); // this is added after the pc getting updated. shouldn't be a problem but beware.
                }
                self.pc = new_addr;
            }
            MicroOp::LoadAccumulator => {
                let value = self.memory[self.pc as usize];
                self.pc += 1;
                self.accumulator = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadAccumulatorFromAddress => {
                let value = self.memory[self.temp_addr as usize];
                self.accumulator = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadX => {
                let value = self.memory[self.pc as usize];
                self.pc += 1;
                self.index_x = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadXfromAddress => {
                let value = self.memory[self.temp_addr as usize];
                self.index_x = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadY => {
                let value = self.memory[self.pc as usize];
                self.pc += 1;
                self.index_y = value;

                self.set_flags_zero_neg(value);
            }
            MicroOp::LoadYfromAddress => {
                let value = self.memory[self.temp_addr as usize];
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
                let address: u16 = STACK_BOTTOM + self.sp as u16;
                self.mem_write(address, self.accumulator);
                self.sp = self.sp.wrapping_sub(1);
            }
            MicroOp::PushStatusBrkPhp => {
                let status_w_b = self.status_p | FLAG_BREAK;
                let address: u16 = STACK_BOTTOM + self.sp as u16;
                self.mem_write(address, status_w_b);
                self.sp = self.sp.wrapping_sub(1);
            }
            MicroOp::PushPCH => {
                let address = STACK_BOTTOM + self.sp as u16;
                let pch: u8 = (self.pc >> 8) as u8;
                self.mem_write(address, pch);
                self.sp = self.sp.wrapping_sub(1);
            }
            MicroOp::PushPCL => {
                let address = STACK_BOTTOM + self.sp as u16;
                let pcl = self.pc as u8;
                self.mem_write(address, pcl);
                self.sp = self.sp.wrapping_sub(1);
            }
            MicroOp::PullPCL => {
                let address = STACK_BOTTOM + self.sp as u16;
                let pcl = self.mem_read(address);
                self.sp = self.sp.wrapping_add(1);
                self.push_micro_from_placeholder(Some(pcl as u16));
            }
            MicroOp::PullPCH(pcl) => match pcl {
                Some(pcl) => {
                    let address = STACK_BOTTOM + self.sp as u16;
                    let pch = (self.mem_read(address) as u16) << 8;
                    let pc = pch | pcl;
                    self.push_micro_from_placeholder(Some(pc));
                }
                None => panic!("Expected pcl value in instruction PullPCH"),
            },
            MicroOp::IncrementPC(pc) => match pc {
                Some(pc) => {
                    self.pc = pc.wrapping_add(1);
                }
                None => panic!("Expected pc value in instruction IncrementPC"),
            },
            MicroOp::IncrementSP(value) => {
                self.sp = self.sp.wrapping_add(value);
            }
            MicroOp::PullAccumulator => {
                let address: u16 = STACK_BOTTOM + self.sp as u16;
                self.accumulator = self.mem_read(address);

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::PullStatus => {
                let address: u16 = STACK_BOTTOM + self.sp as u16;
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
            MicroOp::WriteBackAndIncrement(value) => match value {
                Some(to_write) => {
                    let value = to_write as u8;
                    self.mem_write(self.temp_addr, value);
                    let updated_value = value.wrapping_add(1);
                    self.push_micro_from_placeholder(Some(updated_value as u16));
                }
                None => panic!("Expected a value in instruction WriteBackAndIncrement."),
            },
            MicroOp::WriteBackAndDecrement(value) => match value {
                Some(to_write) => {
                    let value = to_write as u8;
                    self.mem_write(self.temp_addr, value);
                    let updated_value = value.wrapping_sub(1);
                    self.push_micro_from_placeholder(Some(updated_value as u16));
                }
                None => panic!("Expected a value in instruction WriteBackAndDecrement."),
            },
            MicroOp::WriteToAddress(value) => match value {
                Some(to_write) => {
                    let value = to_write as u8;
                    self.mem_write(self.temp_addr, value);
                    self.set_flags_zero_neg(value);
                }
                None => panic!("Expected a value in instruction WriteToAddress."),
            },
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
            MicroOp::LogicalAndAddress => {
                let value = self.mem_read(self.temp_addr);
                self.accumulator &= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::ExclusiveOr => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.accumulator ^= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::ExclusiveOrAddress => {
                let value = self.mem_read(self.temp_addr);
                self.accumulator ^= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::InclusiveOr => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.accumulator |= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::InclusiveOrAddress => {
                let value = self.mem_read(self.temp_addr);
                self.accumulator |= value;

                self.set_flags_zero_neg(self.accumulator);
            }
            MicroOp::BitTestAddress => {
                let value = self.mem_read(self.temp_addr);
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
                self.awc(value);
            }
            MicroOp::AddWithCarryAddress => {
                let value = self.mem_read(self.temp_addr);
                self.awc(value);
            }
            MicroOp::SubWithCarry => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.swc(value);
            }
            MicroOp::SubWithCarryAddress => {
                let value = self.mem_read(self.temp_addr);
                self.swc(value);
            }
            MicroOp::Compare => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.compare(self.accumulator, value);
            }
            MicroOp::CompareAddress => {
                let value = self.mem_read(self.temp_addr);
                self.compare(self.accumulator, value);
            }
            MicroOp::CompareX => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.compare(self.index_x, value);
            }
            MicroOp::CompareXAddress => {
                let value = self.mem_read(self.temp_addr);
                self.compare(self.index_x, value);
            }
            MicroOp::CompareY => {
                let value = self.mem_read(self.pc);
                self.pc += 1;
                self.compare(self.index_y, value);
            }
            MicroOp::CompareYAddress => {
                let value = self.mem_read(self.temp_addr);
                self.compare(self.index_y, value);
            }
            MicroOp::ArithmeticShiftLeft => {
                self.accumulator = self.asl(self.accumulator);
            }
            MicroOp::ArithmeticShiftLeftAddress(value) => match value {
                Some(to_shift) => {
                    let value = to_shift as u8;
                    let result = self.asl(value);
                    self.mem_write(self.temp_addr, result);
                }
                None => panic!("Expected value in instruction ArithmeticShiftLeftAddress."),
            },
            MicroOp::LogicalShiftRight => {
                self.accumulator = self.lsr(self.accumulator);
            }
            MicroOp::LogicalShiftRightAddress(value) => match value {
                Some(to_shift) => {
                    let value = to_shift as u8;
                    let result = self.lsr(value);
                    self.mem_write(self.temp_addr, result);
                }
                None => panic!("Expected value in instruction LogicalShiftRightAddress"),
            },
            MicroOp::RotateLeft => {
                self.accumulator = self.rol(self.accumulator);
            }
            MicroOp::RotateLeftAddress(value) => match value {
                Some(to_rotate) => {
                    let value = to_rotate as u8;
                    let result = self.rol(value);
                    self.mem_write(self.temp_addr, result);
                }
                None => panic!("Expected value in instruction RotateLeftAddress"),
            },
            MicroOp::RotateRight => {
                self.accumulator = self.ror(self.accumulator);
            }
            MicroOp::RotateRightAddress(value) => match value {
                Some(to_rotate) => {
                    let value = to_rotate as u8;
                    let result = self.ror(value);
                    self.mem_write(self.temp_addr, result);
                }
                None => panic!("Expected value in instruction RotateRightAddress"),
            },
            MicroOp::ClearCarry => {
                self.status_p &= !FLAG_CARRY;
            }
            MicroOp::SetCarry => {
                self.status_p |= FLAG_CARRY;
            }
            MicroOp::ClearDecimalMode => {
                self.status_p &= !FLAG_DECIMAL;
            }
            MicroOp::SetDecimalMode => {
                self.status_p |= FLAG_DECIMAL;
            }
            MicroOp::ClearInterrupt => {
                self.status_p &= !FLAG_INTERRUPT;
            }
            MicroOp::SetInterrupt => {
                self.status_p |= FLAG_INTERRUPT;
            }
            MicroOp::ClearOverflow => {
                self.status_p &= !FLAG_OVERFLOW;
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

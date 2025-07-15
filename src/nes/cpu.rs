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

#[derive(Clone, Copy)]
#[derive(Debug)]
#[derive(PartialEq)]
pub enum MicroOp {
    None,
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
    ArithmeticShiftLeftAddress,
    LogicalShiftRight,
    LogicalShiftRightAddress,
    RotateLeft,
    RotateLeftAddress,
    RotateRight,
    RotateRightAddress,
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
    PullPCH,
    IncrementPC,
    IncrementPC2,
    IncrementSP(u8),
    IncrementX,
    IncrementY,
    DecrementX,
    DecrementY,
    DummyCycle,
    AddXtoPointer,
    FetchPointerLowByte,
    FetchPointerHighByte,
    FetchPointerHighByteWithY,
    ReadHighFromIndirectLatch,
    ReadLowFromIndirect,
    ReadAddress,
    WriteBackAndIncrement,
    WriteBackAndDecrement,
    WriteToAddress,
    SetCarry,
    ClearCarry,
    ClearDecimalMode,
    SetDecimalMode,
    ClearInterrupt,
    SetInterrupt,
    ClearOverflow,
}

struct InstructionQueue {
    ops: [MicroOp; 8],
    front: usize,
    back: usize,
    len: usize,
}

impl InstructionQueue {
    fn new() -> Self {
        Self {
            ops: [MicroOp::None; 8],
            front: 0,
            back: 0,
            len: 0,
        }
    }

    fn push_back(&mut self, op: MicroOp) {
        self.ops[self.back] = op;
        self.back = (self.back + 1) % 8;
        self.len += 1;
    }

    fn push_front(&mut self, op: MicroOp) {
        self.front = if self.front == 0 {7} else { self.front - 1 };
        self.ops[self.front] = op;
        self.len += 1;
    }

    fn pop_front(&mut self) -> Option<MicroOp> {
        if self.len == 0 { return None; }
        let op = self.ops[self.front];
        self.front = (self.front + 1) % 8;
        self.len -= 1;
        Some(op)
    }

    fn is_empty(&self) -> bool { self.len == 0}

    fn clear(&mut self) {
        self.front = 0;
        self.back = 0;
        self.len = 0;
    }
}

pub struct Cpu {
    accumulator: u8,
    index_x: u8,
    index_y: u8,
    pc: u16,
    sp: u8,
    status_p: u8,
    current_inst: InstructionQueue,
    memory: Box<[u8; 0x10000]>,
    temp_addr: u16,
    temp_val: u8,
    temp_ptr: u16,
    page_crossed: bool,
    debug_active: bool,
    debug_mem_page: u8,
    current_opcode: u8,
    running: bool,
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
            current_inst: InstructionQueue::new(),
            memory: Box::new([0u8; 0x10000]),
            temp_addr: 0u16,
            temp_val: 0u8,
            temp_ptr: 0u16,
            page_crossed: false,
            running: true,
            debug_active: false,
            debug_mem_page: 0u8,
            current_opcode: 0u8, // doesn't really conflict with BRK, because current_inst is empty so the first opcode will be fetched
        }
    }

    pub fn mem_read(&self, pos: u16) -> u8 {
        self.memory[pos as usize]
    }

    pub fn mem_read_u16(&self, pos: u16) -> u16 {
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

    fn add_page_cross_penalty(&mut self) {
        self.page_crossed = false;
        if self.current_inst.ops[self.current_inst.front] == MicroOp::DummyCycle {
            return;
        }
        self.current_inst.push_front(MicroOp::DummyCycle);
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
        let carry_in: u8 = if self.status_p & FLAG_CARRY != 0 {
            1
        } else {
            0
        };
        let (x1, o1) = self.accumulator.overflowing_sub(value);
        let (x2, o2) = x1.overflowing_sub(1 - carry_in);
        let result = x2;

        if !(o1 | o2) {
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

    fn awc(&mut self, value: u8) {
        let carry_in: u8 = if self.status_p & FLAG_CARRY != 0 {
            1
        } else {
            0
        };

        let (x1, o1) = value.overflowing_add(self.accumulator);
        let (x2, o2) = x1.overflowing_add(carry_in);
        let result = x2;
        if o1 | o2 {
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
    ) -> InstructionQueue {
        let mut queue = InstructionQueue::new();
        match address_mode {
            AddressingMode::ZeroPage => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(inst);
                }
            },
            AddressingMode::ZeroPageX => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddXtoZeroPageAddress);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddXtoZeroPageAddress);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddXtoZeroPageAddress);
                    queue.push_back(inst);
                }
            },
            AddressingMode::ZeroPageY => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddYtoZeroPageAddress);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddYtoZeroPageAddress);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddYtoZeroPageAddress);
                    queue.push_back(inst);
                }
            },
            AddressingMode::Absolute => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByte);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByte);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByte);
                    queue.push_back(inst);
                }
            },
            AddressingMode::AbsoluteX => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByteWithX);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByteWithX);
                    queue.push_back(MicroOp::DummyCycle);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByteWithX);
                    queue.push_back(MicroOp::DummyCycle);
                    queue.push_back(inst);
                }
            },
            AddressingMode::AbsoluteY => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByteWithY);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByteWithY);
                    queue.push_back(MicroOp::DummyCycle);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchLowAddrByte);
                    queue.push_back(MicroOp::FetchHighAddrByteWithY);
                    queue.push_back(MicroOp::DummyCycle);
                    queue.push_back(inst);
                }
            },
            AddressingMode::IndexedIndirect => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddXtoPointer);
                    queue.push_back(MicroOp::FetchPointerLowByte);
                    queue.push_back(MicroOp::FetchPointerHighByte);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddXtoPointer);
                    queue.push_back(MicroOp::FetchPointerLowByte);
                    queue.push_back(MicroOp::FetchPointerHighByte);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::AddXtoPointer);
                    queue.push_back(MicroOp::FetchPointerLowByte);
                    queue.push_back(MicroOp::FetchPointerHighByte);
                    queue.push_back(inst);
                }
            },
            AddressingMode::IndirectIndexed => match inst_type {
                InstType::Read => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::FetchPointerLowByte);
                    queue.push_back(MicroOp::FetchPointerHighByteWithY);
                    queue.push_back(inst);
                }
                InstType::RMW => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::FetchPointerLowByte);
                    queue.push_back(MicroOp::FetchPointerHighByteWithY);
                    queue.push_back(MicroOp::DummyCycle);
                    queue.push_back(MicroOp::ReadAddress);
                    queue.push_back(inst);
                    queue.push_back(MicroOp::WriteToAddress);
                }
                InstType::Write => {
                    queue.push_back(MicroOp::FetchZeroPage);
                    queue.push_back(MicroOp::FetchPointerLowByte);
                    queue.push_back(MicroOp::FetchPointerHighByteWithY);
                    queue.push_back(MicroOp::DummyCycle);
                    queue.push_back(inst);
                }
            },
        }
        queue
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
        self.current_inst = InstructionQueue::new();
        self.pc = self.mem_read_u16(PC_INIT_LOCATION);
        self.running = true;
    }

    pub fn load_test_game(&mut self) {
        let game_code = vec![
            0x20, 0x06, 0x06, 0x20, 0x38, 0x06, 0x20, 0x0d, 0x06, 0x20, 0x2a, 0x06, 0x60, 0xa9,
            0x02, 0x85, 0x02, 0xa9, 0x04, 0x85, 0x03, 0xa9, 0x11, 0x85, 0x10, 0xa9, 0x10, 0x85,
            0x12, 0xa9, 0x0f, 0x85, 0x14, 0xa9, 0x04, 0x85, 0x11, 0x85, 0x13, 0x85, 0x15, 0x60,
            0xa5, 0xfe, 0x85, 0x00, 0xa5, 0xfe, 0x29, 0x03, 0x18, 0x69, 0x02, 0x85, 0x01, 0x60,
            0x20, 0x4d, 0x06, 0x20, 0x8d, 0x06, 0x20, 0xc3, 0x06, 0x20, 0x19, 0x07, 0x20, 0x20,
            0x07, 0x20, 0x2d, 0x07, 0x4c, 0x38, 0x06, 0xa5, 0xff, 0xc9, 0x77, 0xf0, 0x0d, 0xc9,
            0x64, 0xf0, 0x14, 0xc9, 0x73, 0xf0, 0x1b, 0xc9, 0x61, 0xf0, 0x22, 0x60, 0xa9, 0x04,
            0x24, 0x02, 0xd0, 0x26, 0xa9, 0x01, 0x85, 0x02, 0x60, 0xa9, 0x08, 0x24, 0x02, 0xd0,
            0x1b, 0xa9, 0x02, 0x85, 0x02, 0x60, 0xa9, 0x01, 0x24, 0x02, 0xd0, 0x10, 0xa9, 0x04,
            0x85, 0x02, 0x60, 0xa9, 0x02, 0x24, 0x02, 0xd0, 0x05, 0xa9, 0x08, 0x85, 0x02, 0x60,
            0x60, 0x20, 0x94, 0x06, 0x20, 0xa8, 0x06, 0x60, 0xa5, 0x00, 0xc5, 0x10, 0xd0, 0x0d,
            0xa5, 0x01, 0xc5, 0x11, 0xd0, 0x07, 0xe6, 0x03, 0xe6, 0x03, 0x20, 0x2a, 0x06, 0x60,
            0xa2, 0x02, 0xb5, 0x10, 0xc5, 0x10, 0xd0, 0x06, 0xb5, 0x11, 0xc5, 0x11, 0xf0, 0x09,
            0xe8, 0xe8, 0xe4, 0x03, 0xf0, 0x06, 0x4c, 0xaa, 0x06, 0x4c, 0x35, 0x07, 0x60, 0xa6,
            0x03, 0xca, 0x8a, 0xb5, 0x10, 0x95, 0x12, 0xca, 0x10, 0xf9, 0xa5, 0x02, 0x4a, 0xb0,
            0x09, 0x4a, 0xb0, 0x19, 0x4a, 0xb0, 0x1f, 0x4a, 0xb0, 0x2f, 0xa5, 0x10, 0x38, 0xe9,
            0x20, 0x85, 0x10, 0x90, 0x01, 0x60, 0xc6, 0x11, 0xa9, 0x01, 0xc5, 0x11, 0xf0, 0x28,
            0x60, 0xe6, 0x10, 0xa9, 0x1f, 0x24, 0x10, 0xf0, 0x1f, 0x60, 0xa5, 0x10, 0x18, 0x69,
            0x20, 0x85, 0x10, 0xb0, 0x01, 0x60, 0xe6, 0x11, 0xa9, 0x06, 0xc5, 0x11, 0xf0, 0x0c,
            0x60, 0xc6, 0x10, 0xa5, 0x10, 0x29, 0x1f, 0xc9, 0x1f, 0xf0, 0x01, 0x60, 0x4c, 0x35,
            0x07, 0xa0, 0x00, 0xa5, 0xfe, 0x91, 0x00, 0x60, 0xa6, 0x03, 0xa9, 0x00, 0x81, 0x10,
            0xa2, 0x00, 0xa9, 0x01, 0x81, 0x10, 0x60, 0xa2, 0x00, 0xea, 0xea, 0xca, 0xd0, 0xfb,
            0x60,
        ];

        self.memory[0x0600..(0x0600 + game_code.len())].copy_from_slice(&game_code[..]);
        self.mem_write_u16(PC_INIT_LOCATION, 0x0600);
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

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut Cpu),
    {
        if !self.running {
            std::process::exit(0);
        }
        if self.current_inst.is_empty() {
            callback(self);
            self.current_opcode = self.mem_read(self.pc);
            self.pc += 1;
            self.current_inst = self.decode_opcode(self.current_opcode);
        } else if let Some(op) = self.current_inst.pop_front() {
            self.execute_micro_op(op);
        }
    }

    fn execute_current_cycle(&mut self) {
        if self.current_inst.is_empty() {
            self.current_opcode = self.mem_read(self.pc);
            self.pc += 1;
            self.current_inst = self.decode_opcode(self.current_opcode);
        } else if let Some(op) = self.current_inst.pop_front() {
            self.execute_micro_op(op);
        }
    }

    fn print_debug_info(&self) {
        print!("{}", CLS);
        println!(
            "PC: {:04X} | SP: {:02X} | OP: {:02X}",
            self.pc, self.sp, self.current_opcode
        );
        for i in 0..self.current_inst.len {
            print!("{:?}", self.current_inst.ops[i]);
            println!();
        }
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

    fn decode_opcode(&mut self, opcode: u8) -> InstructionQueue {
        let mut queue = InstructionQueue::new();
        match opcode {
            0xA9 => {
                // LDA
                queue.push_back(MicroOp::LoadAccumulator);
            }
            0xA5 => {
                // LDA zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xB5 => {
                // LDA zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                )
            }
            0xAD => {
                // LDA absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                );
            }
            0xBD => {
                // LDA absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                );
            }
            0xB9 => {
                // LDA absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                );
            }
            0xA1 => {
                // LDA indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                );
            }
            0xB1 => {
                // LDA indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::LoadAccumulatorFromAddress,
                    InstType::Read,
                );
            }
            0xA2 => {
                // LDX
                queue.push_back(MicroOp::LoadX);
            }
            0xA6 => {
                // LDX zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LoadXfromAddress,
                    InstType::Read,
                );
            }
            0xB6 => {
                // LDX zero page + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::LoadXfromAddress,
                    InstType::Read,
                );
            }
            0xAE => {
                // LDX absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadXfromAddress,
                    InstType::Read,
                );
            }
            0xBE => {
                // LDX absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LoadXfromAddress,
                    InstType::Read,
                );
            }
            0xA0 => {
                // LDY immediate
                queue.push_back(MicroOp::LoadY);
            }
            0xA4 => {
                // LDY zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LoadYfromAddress,
                    InstType::Read,
                );
            }
            0xB4 => {
                // LDY zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::LoadYfromAddress,
                    InstType::Read,
                );
            }
            0xAC => {
                // LDY absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LoadYfromAddress,
                    InstType::Read,
                );
            }
            0xBC => {
                // LDY absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LoadYfromAddress,
                    InstType::Read,
                );
            }
            0x85 => {
                // STA zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                );
            }
            0x95 => {
                // STA zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                );
            }
            0x8D => {
                // STA absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                );
            }
            0x9D => {
                // STA absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                );
            }
            0x99 => {
                // STA absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                );
            }
            0x81 => {
                // STA indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                );
            }
            0x91 => {
                //STA indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::StoreAccumulator,
                    InstType::Write,
                );
            }
            0x86 => {
                // STX zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::StoreX,
                    InstType::Write,
                );
            }
            0x96 => {
                // STX zero page + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageY,
                    MicroOp::StoreX,
                    InstType::Write,
                );
            }
            0x8E => {
                // STX absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::StoreX,
                    InstType::Write,
                );
            }
            0x84 => {
                // STY zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::StoreY,
                    InstType::Write,
                );
            }
            0x94 => {
                // STY zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::StoreY,
                    InstType::Write,
                );
            }
            0x8C => {
                // STY absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::StoreY,
                    InstType::Write,
                );
            }
            0xAA => {
                // TAX
                queue.push_back(MicroOp::LoadXAccumulator);
            }
            0xA8 => {
                // TAY
                queue.push_back(MicroOp::LoadYAccumulator);
            }
            0xBA => {
                // TSX
                queue.push_back(MicroOp::LoadXStackPointer);
            }
            0x8A => {
                // TXA
                queue.push_back(MicroOp::LoadAccumulatorX);
            }
            0x9A => {
                // TXS
                queue.push_back(MicroOp::LoadStackPointerX);
            }
            0x98 => {
                // TYA
                queue.push_back(MicroOp::LoadAccumulatorY);
            }
            0x48 => {
                // PHA
                queue.push_back(MicroOp::DummyCycle);
                queue.push_back(MicroOp::PushAccumulator);
            }
            0x08 => {
                // PHP
                queue.push_back(MicroOp::DummyCycle);
                queue.push_back(MicroOp::PushStatusBrkPhp);
            }
            0x68 => {
                // PLA
                queue.push_back(MicroOp::DummyCycle);
                queue.push_back(MicroOp::IncrementSP(1));
                queue.push_back(MicroOp::PullAccumulator);
            }
            0x28 => {
                // PLP
                queue.push_back(MicroOp::DummyCycle);
                queue.push_back(MicroOp::IncrementSP(1));
                queue.push_back(MicroOp::PullStatus);
            }
            0x29 => {
                // AND Immediate
                queue.push_back(MicroOp::LogicalAnd);
            }
            0x25 => {
                // AND zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                );
            }
            0x35 => {
                // AND zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                );
            }
            0x2D => {
                // AND absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                );
            }
            0x3D => {
                // AND absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                );
            }
            0x39 => {
                // AND absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                );
            }
            0x21 => {
                // AND indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                );
            }
            0x31 => {
                // AND indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::LogicalAndAddress,
                    InstType::Read,
                );
            }
            0x49 => {
                // EOR immediate
                queue.push_back(MicroOp::ExclusiveOr);
            }
            0x45 => {
                // EOR zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x55 => {
                // EOR zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x4D => {
                // EOR absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x5D => {
                // EOR absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x59 => {
                // EOR absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x41 => {
                // EOR indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x51 => {
                // EOR indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::ExclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x09 => {
                // ORA immediate
                queue.push_back(MicroOp::InclusiveOr);
            }
            0x05 => {
                // ORA zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x15 => {
                // ORA zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x0D => {
                // ORA absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x1D => {
                // ORA absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x19 => {
                // ORA absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x01 => {
                // ORA indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x11 => {
                // ORA indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::InclusiveOrAddress,
                    InstType::Read,
                );
            }
            0x24 => {
                // BIT zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::BitTestAddress,
                    InstType::Read,
                );
            }
            0x2C => {
                // BIT absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::BitTestAddress,
                    InstType::Read,
                );
            }
            0x69 => {
                // ADC
                queue.push_back(MicroOp::AddWithCarry);
            }
            0x65 => {
                // ADC zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                );
            }
            0x75 => {
                // ADC zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                );
            }
            0x6D => {
                // ADC absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                );
            }
            0x7D => {
                // ADC absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                );
            }
            0x79 => {
                // ADC absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                );
            }
            0x61 => {
                // ADC indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                );
            }
            0x71 => {
                // ADC indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::AddWithCarryAddress,
                    InstType::Read,
                );
            }
            0xE9 => {
                // SBC
                queue.push_back(MicroOp::SubWithCarry);
            }
            0xE5 => {
                // SBC zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                );
            }
            0xF5 => {
                // SBC zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                );
            }
            0xED => {
                // SBC absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                );
            }
            0xFD => {
                // SBC absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                );
            }
            0xF9 => {
                // SBC absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                );
            }
            0xE1 => {
                // SBC indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                );
            }
            0xF1 => {
                // SBC indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::SubWithCarryAddress,
                    InstType::Read,
                );
            }
            0xC9 => {
                // CMP
                queue.push_back(MicroOp::Compare);
            }
            0xC5 => {
                // CMP zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::CompareAddress,
                    InstType::Read,
                );
            }
            0xD5 => {
                // CMP zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::CompareAddress,
                    InstType::Read,
                );
            }
            0xCD => {
                // CMP absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::CompareAddress,
                    InstType::Read,
                );
            }
            0xDD => {
                // CMP absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::CompareAddress,
                    InstType::Read,
                );
            }
            0xD9 => {
                // CMP absolute + y
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteY,
                    MicroOp::CompareAddress,
                    InstType::Read,
                );
            }
            0xC1 => {
                // CMP indexed indirect
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndexedIndirect,
                    MicroOp::CompareAddress,
                    InstType::Read,
                );
            }
            0xD1 => {
                // CMP indirect indexed
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::IndirectIndexed,
                    MicroOp::CompareAddress,
                    InstType::Read,
                );
            }
            0xE0 => {
                // CPX
                queue.push_back(MicroOp::CompareX);
            }
            0xE4 => {
                // CPX zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::CompareXAddress,
                    InstType::Read,
                );
            }
            0xEC => {
                // CPX absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::CompareXAddress,
                    InstType::Read,
                );
            }
            0xC0 => {
                // CPY
                queue.push_back(MicroOp::CompareY);
            }
            0xC4 => {
                // CPY zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::CompareYAddress,
                    InstType::Read,
                );
            }
            0xCC => {
                // CPY absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::CompareYAddress,
                    InstType::Read,
                );
            }
            0x0A => {
                // ASL
                queue.push_back(MicroOp::ArithmeticShiftLeft);
            }
            0x06 => {
                // ASL zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::ArithmeticShiftLeftAddress,
                    InstType::RMW,
                );
            }
            0x16 => {
                // ASL zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::ArithmeticShiftLeftAddress,
                    InstType::RMW,
                );
            }
            0x0E => {
                // ASL absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::ArithmeticShiftLeftAddress,
                    InstType::RMW,
                );
            }
            0x1E => {
                // ASL absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::ArithmeticShiftLeftAddress,
                    InstType::RMW,
                );
            }
            0x4A => {
                // LSR
                queue.push_back(MicroOp::LogicalShiftRight);
            }
            0x46 => {
                // LSR zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::LogicalShiftRightAddress,
                    InstType::RMW,
                );
            }
            0x56 => {
                // LSR zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::LogicalShiftRightAddress,
                    InstType::RMW,
                );
            }
            0x4E => {
                // LSR absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::LogicalShiftRightAddress,
                    InstType::RMW,
                );
            }
            0x5E => {
                // LSR absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::LogicalShiftRightAddress,
                    InstType::RMW,
                );
            }
            0x2A => {
                // ROL
                queue.push_back(MicroOp::RotateLeft);
            }
            0x26 => {
                // ROL zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::RotateLeftAddress,
                    InstType::RMW,
                );
            }
            0x36 => {
                // ROL zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::RotateLeftAddress,
                    InstType::RMW,
                );
            }
            0x2E => {
                // ROL absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::RotateLeftAddress,
                    InstType::RMW,
                );
            }
            0x3E => {
                // ROL absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::RotateLeftAddress,
                    InstType::RMW,
                );
            }
            0x6A => {
                // ROR
                queue.push_back(MicroOp::RotateRight);
            }
            0x66 => {
                // ROR zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::RotateRightAddress,
                    InstType::RMW,
                );
            }
            0x76 => {
                // ROR zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::RotateRightAddress,
                    InstType::RMW,
                );
            }
            0x6E => {
                // ROR absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::RotateRightAddress,
                    InstType::RMW,
                );
            }
            0x7E => {
                // ROR absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::RotateRightAddress,
                    InstType::RMW,
                );
            }
            0xE6 => {
                // INC zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::WriteBackAndIncrement,
                    InstType::RMW,
                );
            }
            0xF6 => {
                // INC zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::WriteBackAndIncrement,
                    InstType::RMW,
                );
            }
            0xEE => {
                // INC absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::WriteBackAndIncrement,
                    InstType::RMW,
                );
            }
            0xFE => {
                // INC absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::WriteBackAndIncrement,
                    InstType::RMW,
                );
            }
            0xE8 => {
                // INX
                queue.push_back(MicroOp::IncrementX);
            }
            0xCA => {
                // DEX
                queue.push_back(MicroOp::DecrementX);
            }
            0xC8 => {
                // INY
                queue.push_back(MicroOp::IncrementY);
            }
            0x88 => {
                // DEY
                queue.push_back(MicroOp::DecrementY);
            }
            0xC6 => {
                // DEC zero page
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPage,
                    MicroOp::WriteBackAndDecrement,
                    InstType::RMW,
                );
            }
            0xD6 => {
                // DEC zero page + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::ZeroPageX,
                    MicroOp::WriteBackAndDecrement,
                    InstType::RMW,
                );
            }
            0xCE => {
                // DEC absolute
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::Absolute,
                    MicroOp::WriteBackAndDecrement,
                    InstType::RMW,
                );
            }
            0xDE => {
                // DEC absolute + x
                return Cpu::dispatch_generic_instruction(
                    AddressingMode::AbsoluteX,
                    MicroOp::WriteBackAndDecrement,
                    InstType::RMW,
                );
            }
            0x4C => {
                // JMP absolute
                queue.push_back(MicroOp::FetchLowAddrByte);
                queue.push_back(MicroOp::CopyLowFetchHightoPC);
            }
            0x6C => {
                // JMP indirect
                queue.push_back(MicroOp::FetchLowAddrByte);
                queue.push_back(MicroOp::FetchHighAddrByte);
                queue.push_back(MicroOp::ReadLowFromIndirect);
                queue.push_back(MicroOp::ReadHighFromIndirectLatch);
            }
            0x20 => {
                // JSR
                queue.push_back(MicroOp::FetchLowAddrByte);
                queue.push_back(MicroOp::DummyCycle); //TODO: this isn't actually performing a dummy read. see if it brings problems.
                queue.push_back(MicroOp::PushPCH);
                queue.push_back(MicroOp::PushPCL);
                queue.push_back(MicroOp::CopyLowFetchHightoPC);
            }
            0x60 => {
                // RTS
                queue.push_back(MicroOp::DummyCycle);
                queue.push_back(MicroOp::IncrementSP(1));
                queue.push_back(MicroOp::PullPCL);
                queue.push_back(MicroOp::PullPCH);
                queue.push_back(MicroOp::IncrementPC);
            }
            0x90 => {
                // BCC
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_CARRY,
                    0x00,
                ));
            }
            0xB0 => {
                // BCS
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_CARRY,
                    FLAG_CARRY,
                ));
            }
            0xF0 => {
                // BEQ
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_ZERO,
                    FLAG_ZERO,
                ));
            }
            0xD0 => {
                // BNE
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_ZERO,
                    0x00,
                ));
            }
            0x30 => {
                // BMI
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_NEGATIVE,
                    FLAG_NEGATIVE,
                ));
            }
            0x10 => {
                // BPL
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_NEGATIVE,
                    0x00,
                ));
            }
            0x50 => {
                // BVC
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_OVERFLOW,
                    0x00,
                ));
            }
            0x70 => {
                // BVS
                queue.push_back(MicroOp::FetchRelativeOffset(
                    self.status_p & FLAG_OVERFLOW,
                    FLAG_OVERFLOW,
                ));
            }
            0x18 => {
                // CLC
                queue.push_back(MicroOp::ClearCarry);
            }
            0x38 => {
                // SEC
                queue.push_back(MicroOp::SetCarry);
            }
            0xD8 => {
                // CLD
                queue.push_back(MicroOp::ClearDecimalMode);
            }
            0xF8 => {
                // SED
                queue.push_back(MicroOp::SetDecimalMode);
            }
            0x78 => {
                // SEI
                queue.push_back(MicroOp::SetInterrupt);
            }
            0x58 => {
                // CLI
                queue.push_back(MicroOp::ClearInterrupt);
            }
            0xB8 => {
                // CLV
                queue.push_back(MicroOp::ClearOverflow);
            }
            0xEA => {
                // NOP
                queue.push_back(MicroOp::DummyCycle);
            }
            0x00 => {
                // BRK
                queue.push_back(MicroOp::IncrementPC2);
                queue.push_back(MicroOp::PushPCH);
                queue.push_back(MicroOp::PushPCL);
                queue.push_back(MicroOp::PushStatusBrkPhp);
                queue.push_back(MicroOp::FetchInterruptLow);
                queue.push_back(MicroOp::FetchInterruptHigh);
            }
            0x40 => {
                // RTI
                queue.push_back(MicroOp::DummyCycle);
                queue.push_back(MicroOp::IncrementSP(1));
                queue.push_back(MicroOp::PullStatus);
                queue.push_back(MicroOp::PullPCL);
                queue.push_back(MicroOp::PullPCH);
            }
            _ => unimplemented!("{}", opcode),
        }
        queue
    }

    fn execute_micro_op(&mut self, operation: MicroOp) {
        match operation {
            MicroOp::ReadAddress => {
                self.temp_val = self.mem_read(self.temp_addr);
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
                self.running = false; // TODO: research this better
            }
            MicroOp::CopyLowFetchHightoPC => {
                let high_byte = (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                self.pc = high_byte | self.temp_addr;
            }
            MicroOp::ReadLowFromIndirect => {
                self.temp_ptr = self.mem_read(self.temp_addr) as u16;
            }
            MicroOp::ReadHighFromIndirectLatch => {
                let high_addr = if self.temp_ptr as u8 == 0xFF {
                    self.temp_addr & 0xFF00
                } else {
                    self.temp_addr + 1
                };
                let high_byte = (self.mem_read(high_addr) as u16) << 8;
                self.pc = high_byte | self.temp_ptr;
            }
            MicroOp::FetchHighAddrByteWithX => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                let new_addr = self.temp_addr.wrapping_add(self.index_x as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.temp_addr = new_addr;
                if self.page_crossed {
                    self.add_page_cross_penalty();
                }
            }
            MicroOp::FetchHighAddrByteWithY => {
                self.temp_addr |= (self.mem_read(self.pc) as u16) << 8;
                self.pc += 1;
                let new_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.temp_addr = new_addr;
                if self.page_crossed {
                    self.add_page_cross_penalty();
                }
            }
            MicroOp::FetchPointerLowByte => {
                self.temp_ptr = self.temp_addr;
                self.temp_addr = self.mem_read(self.temp_ptr) as u16;
            }
            MicroOp::FetchPointerHighByte => {
                self.temp_addr |= (self.mem_read(self.temp_ptr.wrapping_add(1)) as u16) << 8; 
            }
            MicroOp::FetchPointerHighByteWithY => {
                self.temp_addr |= (self.mem_read(self.temp_ptr.wrapping_add(1)) as u16) << 8;
                let new_addr = self.temp_addr.wrapping_add(self.index_y as u16);
                self.page_crossed = (self.temp_addr & 0xFF00) != (new_addr & 0xFF00);
                self.temp_addr = new_addr;
                if self.page_crossed {
                    self.add_page_cross_penalty();
                }
            }
            MicroOp::FetchRelativeOffset(value, cond) => {
                let offset = self.mem_read(self.pc);
                self.pc += 1;
                self.schedule_branch(value, cond, offset);
            }
            MicroOp::TakeBranch(offset) => {
                let new_addr = if offset & 0x80 == 0x80 {
                    self.pc.wrapping_add(offset as u16 | 0xFF00)
                } else {
                    self.pc.wrapping_add(offset as u16)
                };
                self.page_crossed = (self.pc & 0xFF00) != (new_addr & 0xFF00);
                if self.page_crossed {
                    self.add_page_cross_penalty();
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
                self.temp_addr = pcl as u16;
            }
            MicroOp::PullPCH => {
                let address = STACK_BOTTOM + self.sp as u16;
                let pch = (self.mem_read(address) as u16) << 8;
                self.temp_addr |= pch;
            }
            MicroOp::IncrementPC => {
                self.pc = self.temp_addr.wrapping_add(1);
            }
            MicroOp::IncrementPC2 => {
                self.pc += 1;
            }
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
            MicroOp::WriteBackAndIncrement => {
                self.mem_write(self.temp_addr, self.temp_val);
                self.temp_val = self.temp_val.wrapping_add(1);
            }
            MicroOp::WriteBackAndDecrement => {
                self.mem_write(self.temp_addr, self.temp_val);
                self.temp_val = self.temp_val.wrapping_sub(1);
            }
            MicroOp::WriteToAddress => {
                self.mem_write(self.temp_addr, self.temp_val);
                self.set_flags_zero_neg(self.temp_val);
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
            MicroOp::ArithmeticShiftLeftAddress => {
                let result = self.asl(self.temp_val);
                self.mem_write(self.temp_addr, result);
            }
            MicroOp::LogicalShiftRight => {
                self.accumulator = self.lsr(self.accumulator);
            }
            MicroOp::LogicalShiftRightAddress => {
                let result = self.lsr(self.temp_val);
                self.mem_write(self.temp_addr, result);
            }
            MicroOp::RotateLeft => {
                self.accumulator = self.rol(self.accumulator);
            }
            MicroOp::RotateLeftAddress => {
                let result = self.rol(self.temp_val);
                self.mem_write(self.temp_addr, result);
            }
            MicroOp::RotateRight => {
                self.accumulator = self.ror(self.accumulator);
            }
            MicroOp::RotateRightAddress => {
                let result = self.ror(self.temp_val);
                self.mem_write(self.temp_addr, result);
            }
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

    pub fn is_running(&self) -> bool {
        self.running
    }
}

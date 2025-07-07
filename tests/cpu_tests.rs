use nestacean::nes::cpu::Cpu;

#[cfg(test)]
mod test {
    use super::*;

    // LDA tests
    #[test]
    fn test_lda() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xA9, 0x05, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x05);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0);
    }

    #[test]
    fn test_lda_zeroflag() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xA9, 0x00, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x00);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0b10);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0);
    }

    #[test]
    fn test_lda_negflag() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xA9, 0xFF, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0xFF);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0b1000_0000);
    }

    #[test]
    fn test_lda_zeropage() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xA5, 0x00, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0, 0x05);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x05);
    }

    #[test]
    fn test_lda_zeropage_x() {
        let mut cpu = Cpu::new();
        let mem: [u8; 2] = [0xB5, 0x10];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(0x04);
        cpu.mem_write(0x14, 0x99);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // AddXtoAddressPlaceholder
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x99);
    }

    #[test]
    fn test_lda_absolute() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xAD, 0x00, 0x30]; // LDA $3000
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0x3000, 0x55);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByte
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_absolute_x() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xBD, 0x00, 0x30];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(2u8);
        cpu.mem_write(0x3002, 0x55);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_absolute_x_pagecross() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xBD, 0xFF, 0x30];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(1u8);
        cpu.mem_write(0x3100, 0x55);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // DummyCycle
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_absolute_y() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xB9, 0x00, 0x30];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(2u8);
        cpu.mem_write(0x3002, 0x55);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_absolute_y_pagecross() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xB9, 0xFF, 0x30];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(1u8);
        cpu.mem_write(0x3100, 0x55);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // DummyCycle
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_indexed_indirect() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xA1, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(2u8);
        cpu.mem_write_u16(0x0052, 0x6523);
        cpu.mem_write(0x6523, 0x69);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // AddXtoPointer
        cpu.tick(); // FetchPointerLowByte
        cpu.tick(); // FetchPointerHighByte
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x69);
    }

    #[test]
    fn test_lda_indirect_indexed() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xB1, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(5u8);
        cpu.mem_write_u16(0x50, 0x1234);
        cpu.mem_write(0x1239, 0xAB);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // FetchPointerWithYLowByte
        cpu.tick(); // FetchPointerWithYHighByte
        cpu.tick(); // LoadImmediate
        assert_eq!(cpu.get_accumulator(), 0xAB);
    }

    #[test]
    fn test_lda_indirect_indexed_pagecross() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xB1, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(1u8);
        cpu.mem_write_u16(0x50, 0x12FF);
        cpu.mem_write(0x1300, 0xAB);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // FetchPointerWithYLowByte
        cpu.tick(); // FetchPointerWithYHighByte
        cpu.tick(); // DummyCycle
        cpu.tick(); // LoadImmediate
        assert_eq!(cpu.get_accumulator(), 0xAB);
    }

    // STA tests
    #[test]
    fn test_sta_zeropage() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0x85, 0x55, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_accumulator(0x69);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // WriteAccumulatorToAddress
        assert_eq!(cpu.get_accumulator(), 0x69);
    }

    // TAX tests
    #[test]
    fn test_tax() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xAA, 0x00, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_accumulator(0x05);
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadXAccumulator
        assert_eq!(cpu.get_index_x(), 0x05);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0);
    }

    #[test]
    fn test_tax_zeroflag() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xAA, 0x00, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_accumulator(0x00);
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadXAccumulator
        assert_eq!(cpu.get_index_x(), 0x00);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0b10);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0);
    }

    #[test]
    fn test_tax_negflag() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xAA, 0x00, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_accumulator(0xFF);
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadXAccumulator
        assert_eq!(cpu.get_index_x(), 0xFF);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0b1000_0000);
    }

    // INX/INY/DEX/DEY tests
    #[test]
    fn test_inx() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xE8, 0xFF, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(0x00);
        cpu.tick(); //fetch and decode
        cpu.tick(); //IncrementX
        assert_eq!(cpu.get_index_x(), 0b01);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0);
    }

    #[test]
    fn test_inx_zeroflag() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xE8, 0xFF, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(0xFF);
        cpu.tick(); //fetch and decode
        cpu.tick(); //IncrementX
        assert_eq!(cpu.get_index_x(), 0x00);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0b10);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0);
    }

    #[test]
    fn test_inx_negflag() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xE8, 0xFF, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(0x7F);
        cpu.tick(); //fetch and decode
        cpu.tick(); //IncrementX
        assert_eq!(cpu.get_index_x(), 0x80);
        assert_eq!(cpu.get_status_p() & 0b0000_0010, 0);
        assert_eq!(cpu.get_status_p() & 0b1000_0000, 0b1000_0000);
    }

    #[test]
    fn test_dex() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xCA, 0xFF, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(0x01);
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.get_index_x(), 0x00);
    }

    #[test]
    fn test_dey() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0x88, 0xFF, 0xFF];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(0x01);
        cpu.tick();
        cpu.tick();
        assert_eq!(cpu.get_index_y(), 0x00);
    }

    // INC tests
    #[test]
    fn test_inc_zeropage() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xE6, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0x50, 0x10);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndIncrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x50], 0x11);
    }

    #[test]
    fn test_inc_zeropage_x() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xF6, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(2);
        cpu.mem_write(0x52, 0x10);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // AddXtoZeroPageAddress
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndIncrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x52], 0x11);
    }

    #[test]
    fn test_inc_zeropage_x_no_overflow() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xF6, 0xFF, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(2);
        cpu.mem_write(0x01, 0x10);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // AddXtoZeroPageAddress
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndIncrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x01], 0x11);
    }

    #[test]
    fn test_inc_absolute() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xEE, 0xFF, 0x10];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0x10FF, 0x10);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByte
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndIncrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x10FF], 0x11);
    }

    #[test]
    fn test_inc_absolute_x() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xFE, 0xFF, 0x10];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0x1100, 0x10);
        cpu.set_index_x(1);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // DummyCycle
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndIncrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x1100], 0x11);
    }

    // DEC tests
    #[test]
    fn test_dec_zeropage() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xC6, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0x50, 0x0A);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndDecrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x50], 0x09);
    }

    #[test]
    fn test_dec_zeropage_x() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xD6, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(2);
        cpu.mem_write(0x52, 0x0A);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // AddXtoZeroPageAddress
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndDecrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x52], 0x09);
    }

    #[test]
    fn test_dec_absolute() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xCE, 0xFF, 0x10];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0x10FF, 0x0A);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByte
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndDecrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x10FF], 0x09);
    }

    #[test]
    fn test_dec_absolute_x() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xDE, 0xFF, 0x10];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.mem_write(0x1100, 0x0A);
        cpu.set_index_x(1);
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // DummyCycle
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndDecrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x1100], 0x09);
    }

    // stack tests
    #[test]
    fn test_pha() {
        let mut cpu = Cpu::new();
        let mem: [u8; 2] = [0x48, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_accumulator(0x01);
        cpu.tick(); // fetch and decode
        cpu.tick(); // DummyCycle
        cpu.tick(); // PushAccumulator
        assert_eq!(cpu.get_memory()[0x01FF], 0x01);
        assert_eq!(cpu.get_sp(), 0xFE);
    }

    #[test]
    fn test_php() {
        let mut cpu = Cpu::new();
        let mem: [u8; 2] = [0x08, 0x00]; // PHP, BRK
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_status_p(0b1010_1010);
        cpu.tick(); // fetch and decode
        cpu.tick(); // DummyCycle
        cpu.tick(); // PushStatus
        assert_eq!(cpu.get_memory()[0x01FF], 0b1010_1010);
        assert_eq!(cpu.get_sp(), 0xFE);
    }

    #[test]
    fn test_pla() {
        let mut cpu = Cpu::new();
        let mem: [u8; 2] = [0x68, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_sp(0xFE);
        cpu.mem_write(0x01FF, 0x01);
        cpu.tick(); // fetch and decode
        cpu.tick(); // DummyCycle
        cpu.tick(); // IncrementSP
        cpu.tick(); // PullAccumulator
        assert_eq!(cpu.get_sp(), 0xFF);
        assert_eq!(cpu.get_accumulator(), 0x01);
    }

    #[test]
    fn test_plp() {
        let mut cpu = Cpu::new();
        let mem: [u8; 2] = [0x28, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_sp(0xFE);
        cpu.mem_write(0x01FF, 0x01);
        cpu.tick(); // fetch and decode
        cpu.tick(); // DummyCycle
        cpu.tick(); // IncrementSP
        cpu.tick(); // PullStatus
        assert_eq!(cpu.get_sp(), 0xFF);
        assert_eq!(cpu.get_status_p(), 0x01);
    }

    // general testing
    #[test]
    fn test_5_ops() {
        let mut cpu = Cpu::new();
        let mem: [u8; 5] = [0xa9, 0xc0, 0xaa, 0xe8, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadAccumulatorImmediate
        cpu.tick(); //fetch and decode
        cpu.tick(); //LoadXAccumulator
        cpu.tick(); //fetch and decode
        cpu.tick(); //IncrementX
        cpu.tick(); //fetch and decode
        cpu.tick(); //Break

        assert_eq!(cpu.get_index_x(), 0xc1);
    }
}

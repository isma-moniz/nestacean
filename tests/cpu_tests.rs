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
        cpu.get_memory()[0] = 0x05;
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
        cpu.get_memory()[0x14] = 0x99;
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
        cpu.get_memory()[0x3000] = 0x55;
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
        cpu.get_memory()[0x3002] = 0x55;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.is_page_crossed(), false);
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_absolute_x_pagecross() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xBD, 0xFF, 0x30];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(1u8);
        cpu.get_memory()[0x3100] = 0x55;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // DummyCycle
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.is_page_crossed(), true);
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_absolute_y() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xB9, 0x00, 0x30];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(2u8);
        cpu.get_memory()[0x3002] = 0x55;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.is_page_crossed(), false);
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_absolute_y_pagecross() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xB9, 0xFF, 0x30];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(1u8);
        cpu.get_memory()[0x3100] = 0x55;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByteWithX
        cpu.tick(); // DummyCycle
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.is_page_crossed(), true);
        assert_eq!(cpu.get_accumulator(), 0x55);
    }

    #[test]
    fn test_lda_indexed_indirect() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xA1, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_x(2u8);
        cpu.get_memory()[0x0052] = 0x23;
        cpu.get_memory()[0x0053] = 0x65;
        cpu.get_memory()[0x6523] = 0x69;
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
        cpu.get_memory()[0x50] = 0x34;
        cpu.get_memory()[0x51] = 0x12;
        cpu.get_memory()[0x1239] = 0xAB;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // FetchPointerWithYLowByte
        cpu.tick(); // FetchPointerWithYHighByte
        cpu.tick(); // LoadImmediate
        assert_eq!(cpu.get_accumulator(), 0xAB);
        assert_eq!(cpu.is_page_crossed(), false);
    }

    #[test]
    fn test_lda_indirect_indexed_pagecross() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xB1, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.set_index_y(1u8);
        cpu.get_memory()[0x50] = 0xFF;
        cpu.get_memory()[0x51] = 0x12;
        cpu.get_memory()[0x1300] = 0xAB;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // FetchPointerWithYLowByte
        cpu.tick(); // FetchPointerWithYHighByte
        cpu.tick(); // DummyCycle
        cpu.tick(); // LoadImmediate
        assert_eq!(cpu.get_accumulator(), 0xAB);
        assert_eq!(cpu.is_page_crossed(), true);
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

    // INX tests
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
    fn test_inc_zeropage() {
        let mut cpu = Cpu::new();
        let mem: [u8; 3] = [0xE6, 0x50, 0x00];
        cpu.load_program(&mem);
        cpu.reset();
        cpu.get_memory()[0x50] = 0x10;
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
        cpu.get_memory()[0x52] = 0x10;
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
        cpu.get_memory()[0x01] = 0x10;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchZeroPage
        cpu.tick(); // AddXtoZeroPageAddress
        cpu.tick(); // ReadAddress
        cpu.tick(); // WriteBackAndIncrement
        cpu.tick(); // WriteToAddress
        assert_eq!(cpu.get_memory()[0x01], 0x11);
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

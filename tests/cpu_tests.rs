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
        let mem: [u8; 3] = [0xAD, 0x00, 0x80]; // LDA $8000
        cpu.load_program(&mem);
        cpu.reset();
        cpu.get_memory()[0x8000] = 0x55;
        cpu.tick(); // fetch and decode
        cpu.tick(); // FetchLowAddrByte
        cpu.tick(); // FetchHighAddrByte
        cpu.tick(); // LoadAccumulatorImmediate
        assert_eq!(cpu.get_accumulator(), 0x55);
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

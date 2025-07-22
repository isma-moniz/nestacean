use crate::{
    nes::mem::{Read, Write},
    nes::cart::Cart,
};

const RAM_BEGIN: u16 = 0x0000;
const RAM_END: u16 = 0x1FFF;
const PPU_REG_BEGIN: u16 = 0x2000;
const PPU_REG_MIRROR_END: u16 = 0x3FFF;
const RAM_MIRROR_BITS: u16 = 0b00000111_11111111;
const PPU_MIRROR_BITS: u16 = 0b00100000_00000111;

pub struct Bus {
    pub ram: [u8; 0x0800], // TODO: check if stack allocation is fine for this
    rom: Cart,
}

impl Bus {
    pub fn new(rom: Cart) -> Self {
        Bus {
            ram: [0; 0x0800],
            rom
        }
    }

    fn mem_write_ram(&mut self, addr: u16, byte: u8) {
        self.ram[(addr & RAM_MIRROR_BITS) as usize] =  byte;
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM_BEGIN..=RAM_END => {
                let real_addr = addr & RAM_MIRROR_BITS;
                self.ram[real_addr as usize] = data;
            }
            PPU_REG_BEGIN..=PPU_REG_MIRROR_END => {
                let real_addr = addr & PPU_MIRROR_BITS;
                todo!("PPU is not supported yet");
            }
            _ => {
                println!("Ignoring mem-write at {}", addr);
            }
        }
    }
}

impl Read for Bus {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            RAM_BEGIN..=RAM_END => self.ram[(addr & RAM_MIRROR_BITS) as usize],
            PPU_REG_BEGIN..=PPU_REG_MIRROR_END => {
                todo!("PPU is not supported yet");
            }
            _ => {
                println!("Ignoring mem-read at {}", addr);
                0
            }
        }
    }
}

impl Write for Bus {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            RAM_BEGIN..=RAM_END => self.ram[(addr & RAM_MIRROR_BITS) as usize] = val,
            PPU_REG_BEGIN..=PPU_REG_MIRROR_END => {
                todo!("PPU is not implemented yet");
            }
            _ => {
                println!("Ignoring mem-write at {}", addr);
            }
        }
    }
}
const NES_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const CTRL_BYTE_1_IDX: usize = 6;
const CTRL_BYTE_2_IDX: usize = 7;
const PRG_SIZE_IDX: usize = 4;
const CHR_SIZE_IDX: usize = 5;
const MAPPER_TYPE_MASK: u8 = 0b1111_0000;
const INES_VER_MASK: u8 = 0b0000_1100;
const FOUR_SCREEN_MASK: u8 = 0b0000_1000;
const VERTICAL_MIRRORING_MASK: u8 = 0b0000_0001;
const SKIP_TRAINER_MASK: u8 = 0b100;
const PRG_ROM_PAGE_SIZE: usize = 16384;
const CHR_ROM_PAGE_SIZE: usize = 8192;

#[derive(Debug, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
}

pub struct Cart {
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub mapper: u8,
    pub screen_mirroring: Mirroring,
}

impl Cart {
    pub fn new(raw: &Vec<u8>) -> Result<Cart, String> {
        if &raw[0..4] != NES_TAG {
            return Err("File is not in iNES file format".to_string());
        }

        let mapper = (raw[CTRL_BYTE_2_IDX] & MAPPER_TYPE_MASK) | (raw[CTRL_BYTE_1_IDX] >> 4);
        
        let ines_ver = raw[CTRL_BYTE_2_IDX] & 0b0000_1100;
        if ines_ver != 0 {
            return Err("NES2.0 format not supported".to_string());
        }

        let four_screen = raw[CTRL_BYTE_1_IDX] & FOUR_SCREEN_MASK != 0;
        let vertical_mirroring = raw[CTRL_BYTE_1_IDX] & VERTICAL_MIRRORING_MASK != 0;
        let screen_mirroring = match(four_screen, vertical_mirroring) {
            (true, _) => Mirroring::FourScreen,
            (false, true) => Mirroring::Vertical,
            (false, false) => Mirroring::Horizontal,
        };

        let prg_rom_size = raw[PRG_SIZE_IDX] as usize * PRG_ROM_PAGE_SIZE;
        let chr_rom_size = raw[CHR_SIZE_IDX] as usize * CHR_ROM_PAGE_SIZE;
        let skip_trainer = raw[CTRL_BYTE_1_IDX] & SKIP_TRAINER_MASK != 0;
        let prg_rom_start = 16 + if skip_trainer { 512 } else { 0 };
        let chr_rom_start = prg_rom_start + prg_rom_size;

        Ok(Cart {
            prg_rom: raw[prg_rom_start..(prg_rom_start + prg_rom_size)].to_vec(),
            chr_rom: raw[chr_rom_start..(chr_rom_start + chr_rom_size)].to_vec(),
            mapper: mapper,
            screen_mirroring: screen_mirroring,
        })
  }
}
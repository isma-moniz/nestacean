const NES_TAG: u32 = 0x4E45531A;

#[derive(Debug, PartialEq)]
pub enum Mirroring {
    Vertical,
    Horizontal,
    FourScreen,
}

pub struct Cart {
    pub name: String,
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
}

impl Cart {
    pub fn new(raw: &Vec<u8>) -> Result<Cart, String> {
        if &raw[0..4] != NES_TAG {
            return Err("File is not in iNES file format".to_string());
        }
  }
}
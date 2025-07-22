#[derive(Default)]
pub struct Memory<D> {
    is_ram: bool,
    data: D,
}

impl<D> Memory<D> {
    pub fn new() -> Self
    where
        D: Default,
    {
        Self::default()
    }

    pub fn is_ram(&self) -> bool {
        self.is_ram
    }
}

impl Memory<Vec<u8>> {
    pub fn rom() -> Self {
        Self::default()
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.resize(size);
        self
    }

    pub fn resize(&mut self, size: usize) {
        self.data.resize(size,0);
    }
}

pub trait Read {
   fn read(&self, addr: u16) -> u8;
   
   fn read_u16(&self, addr: u16) -> u16 {
        let low = self.read(addr);
        let high = self.read(addr.wrapping_add(1));
        u16::from_le_bytes([low, high])
    }
}

pub trait Write {
    fn write(&mut self, addr: u16, val: u8);

    fn write_u16(&mut self, addr: u16, val: u16) {
        let [low, high] = val.to_le_bytes();
        self.write(addr, low);
        self.write(addr, high);
    }
}
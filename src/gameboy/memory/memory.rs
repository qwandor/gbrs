use crate::gameboy::constants::*;
use crate::gameboy::memory::ram::Ram;
use crate::gameboy::memory::rom::Rom;

pub struct Memory {
    rom: Rom,
    wram: Ram
}

impl Memory {
    pub fn read (&self, address: u16) -> u8 {
        // println!("MEMORY READ AT {} ({:#x})", address, address);

        match address {
            ROM_START ..= ROM_END => self.rom.read(address - ROM_START),
            WRAM_START ..= WRAM_END => self.wram.read(address - WRAM_START),
            _ => panic!("Unsupported memory read at {} ({:#x})", address, address)
        }
    }

    pub fn write (&mut self, address: u16, value: u8) {
        match address {
            ROM_START ..= ROM_END => panic!("ROM is read only"),
            WRAM_START ..= WRAM_END => self.wram.write(address, value),
            _ => panic!("Unsupported memory write at {} ({:#x})", address, address)
        }
    }

    pub fn from_rom (rom_path: String) -> Memory {
        Memory {
            rom: Rom::from_file(rom_path),
            wram: Ram::new(WRAM_SIZE)
        }
    }
}

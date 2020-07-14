use crate::gameboy::constants::*;
use crate::gameboy::lcd::*;
use crate::gameboy::memory::ram::Ram;
use crate::gameboy::memory::memory::Memory;
use crate::gameboy::interrupts::*;

pub struct Gpu {
    // This is the WIP frame that the GPU draws to
    frame: [GreyShade; SCREEN_BUFFER_SIZE],
    // This is the frame read by the GUI,
    // it's only updated in VBlank
    finished_frame: [GreyShade; SCREEN_BUFFER_SIZE],

    // X and Y of background position
    scy: u8,
    scx: u8,

    // X and Y of the Window
    wy: u8,
    wx: u8,

    // The scan-line Y co-ordinate
    ly: u8,

    // Scan-line X co-ordinate
    // This isn't a real readable Gameboy address, it's just for internal tracking
    lx: u16,

    bg_pallette: u8,
    sprite_pallete_1: u8,
    sprite_pallete_2: u8,

    status: LcdStatus,
    control: LcdControl,

    // "Object Attribute Memory" - Sprite properties
    oam: Ram,

    dma_source: u8,
    dma_cycles: u8
}

impl Gpu {
    pub fn raw_write (&mut self, raw_address: u16, value: u8) {
        match raw_address {
            OAM_START ..= OAM_END => self.oam.write(raw_address - OAM_START, value),

            0xFF40 => {
                println!("{:08b} was written to the LCD Control register", value);
                self.control = LcdControl::from(value)
            },
            0xFF41 => self.status = LcdStatus::from(value),
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,

            0xFF46 => self.begin_dma(value),

            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,

            0xFF47 => self.bg_pallette = value,
            0xFF48 => self.sprite_pallete_1 = value,
            0xFF49 => self.sprite_pallete_2 = value,

            // CGB only
            0xFF4F => {},
            _ => panic!("Unsupported GPU write at {:#06x}", raw_address)
        }
    }
    pub fn raw_read (&self, raw_address: u16) -> u8 {
        match raw_address {
            OAM_START ..= OAM_END => self.oam.read(raw_address - OAM_START),

            0xFF40 => u8::from(self.control),
            0xFF41 => u8::from(self.status),
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,

            0xFF46 => self.dma_source,

            0xFF4A => self.wy,
            0xFF4B => self.wx,

            0xFF47 => self.bg_pallette,
            0xFF48 => self.sprite_pallete_1,
            0xFF49 => self.sprite_pallete_2,
            _ => panic!("Unsupported GPU read at {:#06x}", raw_address)
        }
    }

    fn begin_dma(&mut self, source: u8) {
        // Really, we should be disabling access to anything but HRAM now,
        // but if the rom is nice then there shouldn't be an issue.
        self.dma_source = source;
        self.dma_cycles = gpu_timing::DMA_CYCLES;
    }

    fn update_dma (&mut self, ints: &mut Interrupts, mem: &mut Memory) {
        // There isn't one pending
        if self.dma_cycles == 0 { return; }

        self.dma_cycles -= 1;
        // Ready to actually perform DMA?
        if self.dma_cycles == 0 {
            let source = (self.dma_source as u16) << 4;

            for i in 0x00..=0x9F {
                let data = mem.read(ints, self, source + i);
                mem.write(ints, self, 0xFE00 + i, data);
            }
        }
    }

    fn enter_vblank (&mut self, ints: &mut Interrupts) {
        ints.raise_interrupt(InterruptReason::VBlank);
        self.finished_frame = self.frame.clone();
    }

    pub fn step(&mut self, cycles: usize, ints: &mut Interrupts, mem: &mut Memory) {
        // TODO: Check that a DMA is performed even with display off
        for _ in 0..cycles {
            self.update_dma(ints, mem);
        }

        if !self.control.display_enable {
            return;
        }

        self.lx = (self.lx + 1) % gpu_timing::HTOTAL;

        let mode = self.status.get_mode();

        let new_mode = match mode {
            LcdMode::VBlank => {
                if self.lx == 0 {
                    self.ly = (self.ly + 1) % gpu_timing::VTOTAL;

                    if self.ly == 0 {
                        LcdMode::OAMSearch
                    } else { mode }
                } else { mode }
            },
            _ => {
                match self.lx {
                    0 => {
                        self.ly += 1;
                        // Done with frame, enter VBlank
                        if self.ly == gpu_timing::VBLANK_ON {
                            self.enter_vblank(ints);
                            LcdMode::VBlank
                        } else { LcdMode::OAMSearch }
                    }
                    gpu_timing::HTRANSFER_ON => LcdMode::Transfer,
                    gpu_timing::HBLANK_ON => LcdMode::HBlank,
                    _ => mode
                }
            }
        };
        self.status.set_mode(new_mode);

        // The first line takes longer to draw
        let line_start = gpu_timing::HTRANSFER_ON +
            if self.ly == 0 { 160 } else { 48 };

        // println!("[{}, {}], mode: {}", self.lx, self.ly, self.status.mode_flag);
        if self.lx == line_start && self.status.get_mode() != LcdMode::VBlank {
            // println!("Draw current line");
            // Draw the current line
            for x in 0..(SCREEN_WIDTH as u8) {
                self.draw_pixel(x, self.ly);
            }
        }
    }

    fn draw_pixel (&mut self, x: u8, y: u8) {
        let ux = x as usize; let uy = y as usize;
        let idx = uy * SCREEN_WIDTH + ux;
        // println!("Setting {}, {} to black", x, y);
        self.frame[idx] = GreyShade::Black;
    }

    pub fn get_sfml_frame (&self) -> [u8; SCREEN_RGBA_SLICE_SIZE] {
        let mut out_array = [0; SCREEN_RGBA_SLICE_SIZE];
        for i in 0..SCREEN_BUFFER_SIZE {
            let start = i * 4;
            match &self.finished_frame[i] {
                GreyShade::White => {
                    out_array[start] = 0xDD;
                    out_array[start + 1] = 0xDD;
                    out_array[start + 2] = 0xDD;
                    out_array[start + 3] = 0xFF;
                },
                GreyShade::LightGrey => {
                    out_array[start] = 0xAA;
                    out_array[start + 1] = 0xAA;
                    out_array[start + 2] = 0xAA;
                    out_array[start + 3] = 0xFF;
                },
                GreyShade::DarkGrey => {
                    out_array[start] = 0x88;
                    out_array[start + 1] = 0x88;
                    out_array[start + 2] = 0x88;
                    out_array[start + 3] = 0xFF;
                },
                GreyShade::Black => {
                    out_array[start] = 0x55;
                    out_array[start + 1] = 0x55;
                    out_array[start + 2] = 0x55;
                    out_array[start + 3] = 0xFF;
                }
            }
        }
        out_array
    }

    pub fn new () -> Gpu {
        let empty_frame = [GreyShade::White; SCREEN_BUFFER_SIZE];
        Gpu {
            frame: empty_frame,
            finished_frame: empty_frame.clone(),
            scy: 0, scx: 0, ly: 0, lx: 0, wy: 0, wx: 0,
            bg_pallette: 0, sprite_pallete_1: 0, sprite_pallete_2: 0,
            status: LcdStatus::new(),
            control: LcdControl::new(),
            oam: Ram::new(OAM_SIZE),
            dma_source: 0, dma_cycles: 0
        }
    }
}

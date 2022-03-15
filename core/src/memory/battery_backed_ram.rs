// RAM with a save file
use crate::{callbacks::CALLBACKS, cartridge::Cartridge, memory::ram::Ram};

// The amount of milliseconds we wait before saving our save file
// (otherwise eg. Link's Awakening would write 2,700 save files
//  on its first frame)
const DEBOUNCE_MILLIS: usize = 1000;

pub struct BatteryBackedRam {
    ram: Ram,
    pub size: usize,

    cart: Cartridge,

    battery_enabled: bool,
    changed_since_last_save: bool,
    last_saved_at: usize
}

impl BatteryBackedRam {
    pub fn read(&self, address: u16) -> u8 {
        self.ram.read(address)
    }

    pub fn write(&mut self, address: u16, value: u8) {
        self.ram.write(address, value);
        self.changed_since_last_save = true;
    }

    pub fn step(&mut self) {
        if !self.changed_since_last_save || !self.battery_enabled {
            return;
        }

        let current_timestamp = unsafe { (CALLBACKS.get_ms_timestamp)() };
        let millis_since_last_save = current_timestamp - self.last_saved_at;

        if millis_since_last_save >= DEBOUNCE_MILLIS {
            self.save_ram_contents()
        }
    }

    fn save_ram_contents(&mut self) {
        self.changed_since_last_save = false;

        unsafe {
            self.last_saved_at = (CALLBACKS.get_ms_timestamp)();
            (CALLBACKS.save)(
                &self.cart.title[..],
                &self.cart.rom_path[..],
                &self.ram.bytes
            );
        }
    }

    pub fn new(cart: Cartridge, battery_enabled: bool) -> BatteryBackedRam {
        let save_contents = unsafe {
            (CALLBACKS.load)(&cart.title[..], &cart.rom_path[..], cart.ram_size)
        };
        let current_timestamp = unsafe { (CALLBACKS.get_ms_timestamp)() };

        let ram = Ram::from_bytes(save_contents, cart.ram_size);

        BatteryBackedRam {
            ram,
            size: cart.ram_size,

            cart,
            battery_enabled,
            changed_since_last_save: false,

            last_saved_at: current_timestamp
        }
    }
}
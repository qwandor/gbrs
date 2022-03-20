use crate::sound::apu::APUChannel;
use crate::sound::volume_envelope::VolumeEnvelope;

const WAVEFORM_TABLE: [u8; 4] = [
  0b00000001,
  0b00000011,
  0b00001111,
  0b11111100
];
// 256Hz
const LENGTH_CLOCKS: usize = 16_392;

pub struct APUChannel2 {
  enabled: bool,
  frequency: usize,
  frequency_timer: usize,
  wave_duty: usize,
  wave_duty_position: usize,
  length_timer: usize,
  length_data: usize,
  length_timer_enabled: bool,
  length_clock_timer: usize,
  volume_envelope: VolumeEnvelope
}

impl APUChannel2 {
  pub fn new () -> APUChannel2 {
    APUChannel2 {
      enabled: false,
      frequency: 0,
      frequency_timer: 1,
      wave_duty: 2,
      wave_duty_position: 0,
      length_timer: 0,
      length_data: 0,
      length_timer_enabled: false,
      length_clock_timer: 0,
      volume_envelope: VolumeEnvelope::new()
    }
  }

  // This is called when a game writes a 1 in bit 7 of the NR24 register.
  // That means the game is issuing a "restart sound" command
  fn restart_triggered (&mut self) {
    self.volume_envelope.restart_triggered();
    self.length_timer = 64 - self.length_data;
    self.enabled = true;
  }

  // Called at 256Hz
  fn length_clock (&mut self) {
    if self.length_timer > 0 {
      self.length_timer -= 1;
    }

    if self.length_timer == 0 {
      if self.length_timer_enabled {
        self.length_timer = 64 - self.length_data;
        self.enabled = false;
      }
    }
  }
}

impl APUChannel for APUChannel2 {
  fn step (&mut self) {
    // TODO: I think the Frame Sequencer timers should still be ticking even
    //   if this channel is not enabled. The Frame Sequencer exists outside
    //   the channel.
    if !self.enabled { return }
    
    self.frequency_timer -= 1;

    if self.frequency_timer == 0 {
      self.frequency_timer = (2048 - self.frequency) * 4;

      // Wrapping pointer into the bits of the WAVEFORM_TABLE value
      self.wave_duty_position += 1;
      if self.wave_duty_position == 8 {
        self.wave_duty_position = 0
      }
    }

    self.volume_envelope.step();

    self.length_clock_timer += 1;
    if self.length_clock_timer == LENGTH_CLOCKS {
      self.length_clock_timer = 0;
      self.length_clock();
    }
  }

  fn read (&self, address: u16) -> u8 {
    match address {
      _ => 0//panic!("Unimplemented APU Channel 2 read {:#06x}", address)
    }
  }

  fn write (&mut self, address: u16, value: u8) {
    match address {
      0xFF16 => {
        let wave_duty = (value & 0b1100_0000) >> 6;
        let length = value & 0b0011_1111;
        self.wave_duty = wave_duty as usize;
        self.length_data = length as usize;
      },
      0xFF17 => self.volume_envelope.register_write(value),
      0xFF18 => {
        // This register sets the bottom 8 bits of the 11-bit
        // frequency register.
        self.frequency = 
          (self.frequency & 0b111_0000_0000)
          | value as usize;
      },
      0xFF19 => {
        // Among other things, this register sets the top 3 bits
        // of the 11-bit frequency register.
        let frequency_bits = value & 0b0000_0111;
        self.frequency =
          (self.frequency & 0b000_1111_1111)
          | ((frequency_bits as usize) << 8);
        
        self.length_timer = 0;
        self.length_clock_timer = 0;
        self.length_timer_enabled = (value & 0b0100_0000) > 0;

        if (value & 0b1000_0000) > 0 {
          self.restart_triggered();
        }
      },
      _ => unreachable!()
    }
  }

  fn sample (&self) -> i16 {
    if !self.enabled { return 0 }

    let wave_pattern = WAVEFORM_TABLE[self.wave_duty];
    let amplitude_bit = (wave_pattern & (1 << self.wave_duty_position)) >> self.wave_duty_position;
    
    let dac_input = amplitude_bit as usize * self.volume_envelope.volume;
    // The DAC in the Gameboy outputs between -1.0 and 1.0
    let dac_output = (dac_input as f64 / 7.5) - 1.0;

    (dac_output * 7500.0) as i16
  }
}

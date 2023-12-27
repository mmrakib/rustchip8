use std::{
    thread,
    time,
    fs,
    io,
    io::Read,
};

const fontset: [[u8; 5]; 16] {
    [0xF0, 0x90, 0x90, 0x90, 0xF0], // 0
    [0x20, 0x60, 0x20, 0x20, 0x70], // 1
    [0xF0, 0x10, 0xF0, 0x80, 0xF0], // 2
    [0xF0, 0x10, 0xF0, 0x10, 0xF0], // 3
    [0x90, 0x90, 0xF0, 0x10, 0x10], // 4
    [0xF0, 0x80, 0xF0, 0x10, 0xF0], // 5
    [0xF0, 0x80, 0xF0, 0x90, 0xF0], // 6
    [0xF0, 0x10, 0x20, 0x40, 0x40], // 7
    [0xF0, 0x90, 0xF0, 0x90, 0xF0], // 8
    [0xF0, 0x90, 0xF0, 0x10, 0xF0], // 9
    [0xF0, 0x90, 0xF0, 0x90, 0x90], // A
    [0xE0, 0x90, 0xE0, 0x90, 0xE0], // B
    [0xF0, 0x80, 0x80, 0x80, 0xF0], // C
    [0xE0, 0x90, 0x90, 0x90, 0xE0], // D
    [0xF0, 0x80, 0xF0, 0x80, 0xF0], // E
    [0xF0, 0x80, 0xF0, 0x80, 0x80], // F
};


#[derive(Debug)]
pub struct Machine {
    opcode: u16,
    memory: [u8; 4096],
    display: [[u8; 64]; 32], // display[row][col]
    registers: [u8; 16],
    pc: u16,
    index: u16,
    stack: [u16; 16],
    sp: u8,
    delay_timer: u8,
    sound_timer: u8,
}

impl Machine {
    pub fn new() -> Self {
        Self {
            opcode: 0,
            memory: [0; 4096],
            display: [[0; 64]; 32],
            registers: [0; 16],
            pc: 0x200,
            index: 0,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
        }
    }

    pub fn init(&mut self, filename: String) {
        self.load_rom(filename);
        self.load_fontset();
    }

    // TODO: Any way to make this more efficient? 
    // Possibly read file size => read whole file into buffer at once => extend memory as slice?
    fn load_rom(&mut self, filename: String) {
        let mut file = match fs::File::open(filename) {
            Ok(file) => file,
            Err(why) => panic!("{}", why),
        };

        let mut pos = 0;
        let mut byte: [u8; 1] = [0; 1];

        loop {
            match file.read_exact(&mut byte) {
                Ok(_) => {
                    self.memory[0x200 + pos] = byte[0];
                }

                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }

                Err(_) => {
                    break;
                }
            }
            pos += 1;
        }
    }

    fn load_fontset(&mut self) {
        for sprite in 0..16 {
            for row in 0..5 {
                self.memory[(sprite * 5) + row] = fontset[sprite][row];
            }
        }
    }

    fn map_opcode_delay(opcode: u16) -> time::Duration {
        let starts_with = |num, places| -> bool {
            let mask = match places {
                1 => 0x000F,
                2 => 0x00FF,
                3 => 0x0FFF,
                4 => 0xFFFF,
                _ => return false,
            };
            let value = (opcode >> (4 * (4 - places))) & mask;
            if value == num { true } else { false }
        };

        let ends_with = |num, places| -> bool {
            let mask = match places {
                1 => 0x000F,
                2 => 0x00FF,
                3 => 0x0FFF,
                4 => 0xFFFF,
                _ => return false,
            };
            let value = opcode & mask;
            if value == num { true } else { false }
        };

        // https://jackson-s.me/2019/07/13/Chip-8-Instruction-Scheduling-and-Frequency.html
        match opcode {
            0x00E0 => time::Duration::from_micros(109),
            0x00EE => time::Duration::from_micros(105),
            opcode if starts_with(0x1, 1) => time::Duration::from_micros(105),
            opcode if starts_with(0x2, 1) => time::Duration::from_micros(105),
            opcode if starts_with(0x3, 1) => time::Duration::from_micros(55),
            opcode if starts_with(0x4, 1) => time::Duration::from_micros(55),
            opcode if starts_with(0x5, 1) => time::Duration::from_micros(73),
            opcode if starts_with(0x6, 1) => time::Duration::from_micros(27),
            opcode if starts_with(0x7, 1) => time::Duration::from_micros(45),
            opcode if starts_with(0x8, 1) && ends_with(0x0, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0x1, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0x2, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0x3, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0x4, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0x5, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0x6, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0x7, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x8, 1) && ends_with(0xE, 1) => time::Duration::from_micros(200),
            opcode if starts_with(0x9, 1) => time::Duration::from_micros(73),
            opcode if starts_with(0xA, 1) => time::Duration::from_micros(55),
            opcode if starts_with(0xB, 1) => time::Duration::from_micros(105),
            opcode if starts_with(0xC, 1) => time::Duration::from_micros(164),
            opcode if starts_with(0xD, 2) => time::Duration::from_micros(2734),
            opcode if starts_with(0xE, 1) && ends_with(0x9E, 2) => time::Duration::from_micros(73),
            opcode if starts_with(0xE, 1) && ends_with(0xA1, 2) => time::Duration::from_micros(73),
            opcode if starts_with(0xF, 1) && ends_with(0x07, 2) => time::Duration::from_micros(45),
            opcode if starts_with(0xF, 1) && ends_with(0x0A, 2) => time::Duration::from_micros(0),
            opcode if starts_with(0xF, 1) && ends_with(0x15, 2) => time::Duration::from_micros(45),
            opcode if starts_with(0xF, 1) && ends_with(0x18, 2) => time::Duration::from_micros(45),
            opcode if starts_with(0xF, 1) && ends_with(0x1E, 2) => time::Duration::from_micros(86),
            opcode if starts_with(0xF, 1) && ends_with(0x29, 2) => time::Duration::from_micros(91),
            opcode if starts_with(0xF, 1) && ends_with(0x33, 2) => time::Duration::from_micros(927),
            opcode if starts_with(0xF, 1) && ends_with(0x55, 2) => time::Duration::from_micros(605),
            opcode if starts_with(0xF, 1) && ends_with(0x65, 2) => time::Duration::from_micros(605),
            _ => time::Duration::from_micros(2000),
        }
    }

    pub fn cycle(&mut self) {
        self.opcode = self.memory[self.pc as usize] as u16 + self.memory[self.pc as usize + 1] as u16;
        self.pc += 1;

        match self.opcode {
            _ => {}
        };

        let duration = Self::map_opcode_delay(self.opcode);
        thread::sleep(duration);
    }
}

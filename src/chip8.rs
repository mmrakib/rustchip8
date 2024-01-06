use std::{
    thread,
    time,
    fs,
    io,
    io::Read,
    fmt,
};
use macroquad::prelude::*;

const FONTSET: [[u8; 5]; 16] = [
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
];

pub struct Machine {
    opcode: u16,
    memory: [u8; 4096],
    display: [[u8; 32]; 64], // display[x][y]
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
            display: [[0; 32]; 64],
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
                self.memory[(sprite * 5) + row] = FONTSET[sprite][row];
            }
        }
    }

    fn map_key_to_keyboard(keycode: u8) -> macroquad::input::KeyCode {
        use macroquad::input::KeyCode;
        match keycode {
            0x1 => KeyCode::Key1,
            0x2 => KeyCode::Key2,
            0x3 => KeyCode::Key3,
            0xC => KeyCode::Key4,
            0x4 => KeyCode::Q,
            0x5 => KeyCode::W,
            0x6 => KeyCode::E,
            0xD => KeyCode::R,
            0x7 => KeyCode::A,
            0x8 => KeyCode::S,
            0x9 => KeyCode::D,
            0xE => KeyCode::F,
            0xA => KeyCode::Z,
            0x0 => KeyCode::X,
            0xB => KeyCode::C,
            0xF => KeyCode::V,
            _ => panic!("Incorrect key to map"),
        }
    }

    fn map_key_from_keyboard(keycode: macroquad::input::KeyCode) -> u8 {
        use macroquad::input::KeyCode;
        match keycode {
            KeyCode::Key1 => 0x1,
            KeyCode::Key2 => 0x2,
            KeyCode::Key3 => 0x3,
            KeyCode::Key4 => 0xC,
            KeyCode::Q => 0x4,
            KeyCode::W => 0x5,
            KeyCode::E => 0x6,
            KeyCode::R => 0xD,
            KeyCode::A => 0x7,
            KeyCode::S => 0x8,
            KeyCode::D => 0x9,
            KeyCode::F => 0xE,
            KeyCode::Z => 0xA,
            KeyCode::X => 0x0,
            KeyCode::C => 0xB, 
            KeyCode::V => 0xF,
            _ => panic!("Incorrect key to map"),
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
            opcode if starts_with(0xD, 1) => time::Duration::from_micros(22734),
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

    fn cycle(&mut self) {
        let pc = self.pc as usize;

        let opcode_high_byte = self.memory[pc] as u16;
        let opcode_low_byte = self.memory[pc + 1] as u16;
        self.opcode = (opcode_high_byte << 8) | opcode_low_byte;
        let opcode = self.opcode;

        self.pc += 2;

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

        match opcode {
            0x00E0 => self.op_00e0(),
            opcode if starts_with(0x1, 1) => self.op_1nnn(),
            opcode if starts_with(0x6, 1) => self.op_6xnn(),
            opcode if starts_with(0x7, 1) => self.op_7xnn(),
            opcode if starts_with(0xA, 1) => self.op_Annn(),
            opcode if starts_with(0xD, 1) => self.op_Dxyn(),
            _ => {},
        };

        let duration = Self::map_opcode_delay(self.opcode);
        thread::sleep(duration);
    }

    pub async fn run(&mut self) {
        let scale_ratio: f32 = 16.0;
        request_new_screen_size(64.0 * scale_ratio, 32.0 * scale_ratio);

        loop {
            clear_background(BLACK);

            self.cycle();

            for x in 0..64 {
                for y in 0..32 {
                    if self.display[x][y] != 0 {
                        let pw: f32 = (screen_width() as f32) / 64.0;
                        let ph: f32 = (screen_height() as f32) / 32.0;
            
                        draw_rectangle(pw * (x as f32), ph * (y as f32), pw as f32, ph as f32, WHITE);
                    }
                }
            }

            next_frame().await;
        }
    }

    fn op_00e0(&mut self) {
        for x in 0..64 {
            for y in 0..32 {
                self.display[x][y] = 0x00;
            }
        }
    }

    fn op_00ee(&mut self) {
        self.pc = self.stack[self.sp as usize];
        self.sp -= 1;
    }

    fn op_1nnn(&mut self) {
        let addr: u16 = self.opcode & 0x0FFF;
        self.pc = addr;
    }

    fn op_2nnn(&mut self) {
        let addr: u16 = self.opcode & 0x0FFF;
        self.sp += 1;
        self.stack[self.sp as usize] = self.pc;
        self.pc = addr;
    }

    fn op_3xnn(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let nn: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] == nn {
            self.pc += 2;
        }
    }

    fn op_4xnn(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let nn: u8 = (self.opcode & 0x00FF) as u8;

        if self.registers[vx as usize] != nn {
            self.pc += 2;
        }
    }

    fn op_5xy0(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        if self.registers[vx as usize] == self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    fn op_6xnn(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let nn: u8 = (self.opcode & 0x00FF) as u8;

        self.registers[vx as usize] = nn;
    }

    fn op_7xnn(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let nn: u8 = (self.opcode & 0x00FF) as u8;

        let value = self.registers[vx as usize];
        self.registers[vx as usize] = value.wrapping_add(nn);
    }

    fn op_8xy0(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        self.registers[vx as usize] = self.registers[vy as usize];
    }

    fn op_8xy1(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        self.registers[vx as usize] |= self.registers[vy as usize];
    }

    fn op_8xy2(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        self.registers[vx as usize] &= self.registers[vy as usize];
    }

    fn op_8xy3(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;
        self.registers[vx as usize] ^= self.registers[vy as usize];
    }

    fn op_8xy4(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        if (self.registers[vx as usize] + self.registers[vy as usize]) as u32 > 255 as u32 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] += self.registers[vy as usize] & 0xFF;
    }

    fn op_8xy5(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        if self.registers[vx as usize] > self.registers[vy as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] -= self.registers[vy as usize];
    }

    fn op_8xy6(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;

        if self.registers[vx as usize] & 0x01 == 1 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] >>= 1;
    }

    fn op_8xy7(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        if self.registers[vx as usize] > self.registers[vy as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] = self.registers[vy as usize] - self.registers[vx as usize];
    }

    fn op_8xyE(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;

        if self.registers[vx as usize] & 0x80 == 1 {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] <<= 1;
    }

    fn op_9xy0(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let vy: u8 = ((self.opcode >> 4) & 0x000F) as u8;

        if self.registers[vx as usize] != self.registers[vy as usize] {
            self.pc += 2;
        }
    }

    fn op_Annn(&mut self) {
        let addr: u16 = self.opcode & 0x0FFF;
        self.index = addr;
    }

    fn op_Bnnn(&mut self) {
        let addr: u16 = self.opcode & 0x0FFF;
        self.pc = addr + (self.registers[0x0] as u16);
    }

    fn op_Cxnn(&mut self) {
        let vx: u8 = ((self.opcode >> 8) & 0x000F) as u8;
        let nn: u8 = (self.opcode & 0x00FF) as u8;

        let rand_byte = macroquad::rand::gen_range(0, 255);

        self.registers[vx as usize] = rand_byte & nn;
    }

    fn op_Dxyn(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        let vy: usize = ((self.opcode >> 4) & 0x000F) as usize;
    
        let x: usize = self.registers[vx] as usize % 64;
        let y: usize = self.registers[vy] as usize % 32;
    
        let height: usize = (self.opcode & 0x000F) as usize;
    
        self.registers[0xF] = 0;
    
        for row in 0..height {
            let sprite_row = self.memory[(self.index + row as u16) as usize];
    
            for col in 0..8 {
                let sprite_pixel = (sprite_row >> (7 - col)) & 1;
                let display_pixel = &mut self.display[(x + col) % 64][(y + row) % 32];
    
                if sprite_pixel == 1 {
                    if *display_pixel == 1 {
                        self.registers[0xF] = 1;
                    }
                    *display_pixel ^= 1;
                }
            }
        }
    }

    fn op_Ex9E(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        let keycode: u8 = self.registers[vx];

        if macroquad::input::is_key_down( Machine::map_key_to_keyboard(keycode) ) {
            self.pc += 2;
        }
    }

    fn op_ExA1(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        let keycode: u8 = self.registers[vx];

        if !macroquad::input::is_key_down( Machine::map_key_to_keyboard(keycode) ) {
            self.pc += 2;
        }
    }

    fn op_Fx07(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        self.registers[vx] = self.delay_timer;
    }

    fn op_Fx0A(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        let keycode: u8 = self.registers[vx];

        // Blocks and polls input at 500 Hz
        while !macroquad::input::is_key_down( Machine::map_key_to_keyboard(keycode) ) {
            thread::sleep(time::Duration::from_millis(2));
        }
    }

    fn op_Fx15(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        self.delay_timer = self.registers[vx];
    }

    fn op_Fx18(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        self.sound_timer = self.registers[vx];
    }

    fn op_Fx1E(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        self.index += self.registers[vx] as u16;
    }

    fn op_Fx29(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        self.index = (self.registers[vx] * 5) as u16;
    }

    fn op_Fx33(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;
        let value: u8 = self.registers[vx];
        
        self.memory[(self.index) as usize] = (value / 100) % 10;
        self.memory[(self.index + 1) as usize] = (value / 10) % 10;
        self.memory[(self.index + 2) as usize] = value % 10;
    }

    fn op_Fx55(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;

        for reg in 0..=vx {
            self.memory[(self.index as usize + reg) as usize] = self.registers[reg];
        }
    }

    fn op_Fx65(&mut self) {
        let vx: usize = ((self.opcode >> 8) & 0x000F) as usize;

        for reg in 0..=vx {
            self.registers[reg] = self.memory[(self.index as usize + reg) as usize];
        }
    }
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::new();

        output.push_str( &format!("opcode: {}\n", self.opcode) );
        output.push_str( &format!("memory: {:?}\n", self.memory) );
        output.push_str( &format!("registers: {:?}\n", self.registers) );
        output.push_str( &format!("pc: {}\n", self.pc) );
        output.push_str( &format!("index: {}\n", self.index) );
        output.push_str( &format!("stack: {:?}\n", self.stack) );
        output.push_str( &format!("sp: {}\n", self.sp) );
        output.push_str( &format!("delay_timer: {}\n", self.delay_timer) );
        output.push_str( &format!("sound_timer: {}\n", self.sound_timer) );

        let display_str = String::from("display:\n");
        for x in 0..64 {
            output.push_str( &format!("{:?}\n", self.display[x]) );
        }

        write!(f, "{}", output)?;

        Ok(())
    }
}

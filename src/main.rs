mod chip8;

use std::{
    fs,
    io,
    io::Read,
    thread,
    time
};
use macroquad::main;
use crate::chip8::Machine;

#[main("Chip8")]
async fn main() {
    let mut m: Machine = Machine::new();
    m.init(String::from("IBMLogo.ch8"));
    m.run().await;
}

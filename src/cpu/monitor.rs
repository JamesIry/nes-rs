use std::any::Any;

use anyhow::{Ok, Result};

pub trait Monitor: Any {
    #[allow(clippy::too_many_arguments)]
    fn new_instruction(
        &mut self,
        cycle: usize,
        pc: u16,
        sp: u8,
        a: u8,
        x: u8,
        y: u8,
        status: u8,
    ) -> Result<()>;
    fn fetch_instruction_byte(&mut self, byte: u8) -> Result<()>;
    fn read_data_byte(&mut self, addr: u16, data: u8) -> Result<()>;
    fn end_instruction(&mut self) -> Result<()>;

    fn as_any(&mut self) -> &mut dyn Any;
}

pub struct NulMonitor {}

#[allow(unused_variables)]
impl Monitor for NulMonitor {
    fn new_instruction(
        &mut self,
        cycle: usize,
        pc: u16,
        sp: u8,
        a: u8,
        x: u8,
        y: u8,
        status: u8,
    ) -> Result<()> {
        Ok(())
    }
    fn fetch_instruction_byte(&mut self, byte: u8) -> Result<()> {
        Ok(())
    }

    fn read_data_byte(&mut self, addr: u16, data: u8) -> Result<()> {
        Ok(())
    }

    fn end_instruction(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

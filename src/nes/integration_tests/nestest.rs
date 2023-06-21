use std::any::Any;
use std::cell::RefCell;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Cursor, Write};
use std::mem::swap;
use std::rc::Rc;

use crate::cpu::decode::decode;
use crate::cpu::flags::StatusFlags;
use crate::cpu::instructions::Instruction;
use crate::cpu::monitor::Monitor;
use crate::cpu::{CPUType, CPU};
use crate::nes::apu::APU;
use crate::nes::cartridge::{Cartridge, CartridgeCPUPort};
use crate::nes::ppu::PPU;
use crate::ram::RAM;
use anyhow::Result;

/**
 * Runs the test defined at https://github.com/christopherpow/nes-test-roms/blob/master/other/nestest.txt
 */
#[test]
fn test() {
    let cartridge = Rc::new(RefCell::new(
        Cartridge::load("resources/test/nestest.nes").unwrap(),
    ));

    let cpu = Rc::new(RefCell::new(CPU::new(CPUType::RP2A03)));
    let ppu = Rc::new(RefCell::new(PPU::new()));

    // 0x0000 - 0x1FFFF RAM
    // NES ram is physically only 0x0000 - 0x07FF, but it's then "mirrored" 3 more
    // times to 0x1FFF. "Mirroring" can be accomplished by masking off some bits

    cpu.as_ref()
        .borrow_mut()
        .add_device(Rc::new(RefCell::new(RAM::new(0x0000, 0x1FFF, 0x07FF))));
    //0x2000 - 0x3FFF  PPU Registers from 0x2000 to 0x2007 and then mirrored with mask 0x0007
    cpu.borrow_mut().add_device(ppu);
    //0x4000 - 0x4017  APU and IO registers
    //0x4018 - 0x401F  APU and IO functionality that is disabled
    cpu.as_ref()
        .borrow_mut()
        .add_device(Rc::new(RefCell::new(APU::new(cpu.clone()))));
    //0x4020 - 0xFFFF  Cartridge space
    cpu.as_ref()
        .borrow_mut()
        .add_device(Rc::new(RefCell::new(CartridgeCPUPort::new(cartridge))));

    cpu.borrow_mut().reset();

    cpu.borrow_mut().monitor = Box::new(NestestMonitor::new());
    cpu.borrow_mut().reset_to(0xC000);

    // now loop until trapped or halted
    while !cpu.borrow().stuck() && cpu.borrow().cycles <= 26560 {
        cpu.borrow_mut().clock();
    }

    let mut cpu = cpu.borrow_mut();

    assert_eq!(0, cpu.read_bus_byte(0x02));
    assert_eq!(0, cpu.read_bus_byte(0x03));

    let monitor = cpu
        .monitor
        .as_any()
        .downcast_mut::<NestestMonitor>()
        .unwrap();

    let mut buf_writer = BufWriter::new(Vec::new());
    swap(&mut monitor.buf_writer, &mut buf_writer);
    let actual_reader = BufReader::new(Cursor::new(buf_writer.into_inner().unwrap()));

    let file = File::open("resources/test/nestest.log").unwrap();
    let expected_reader = BufReader::new(file);

    let mut actual_lines = actual_reader.lines();
    let mut expected_lines = expected_reader.lines();

    loop {
        let expected = expected_lines.next().map(|r| r.unwrap());
        let actual = actual_lines.next().map(|r| r.unwrap());

        match (expected, actual) {
            (None, None) => break,
            (None, Some(actual)) => {
                panic!("Extra lines in actual output starting with {}", actual);
            }
            (Some(expected), None) => {
                panic!("Extra lines in log starting with {}", expected);
            }
            (Some(expected), Some(actual)) => assert_eq!(expected, actual),
        }
    }
}

pub struct NestestMonitor {
    started: bool,
    cycle: usize,
    pc: u16,
    sp: u8,
    a: u8,
    x: u8,
    y: u8,
    status: u8,
    instruction_bytes: [u8; 3],
    instruction_byte_count: usize,
    data_bytes: [(u16, u8); 5],
    data_byte_count: usize,
    buf_writer: BufWriter<Vec<u8>>,
}

impl NestestMonitor {
    pub fn new() -> Self {
        Self {
            started: false,
            cycle: 0,
            pc: 0,
            sp: 0,
            a: 0,
            x: 0,
            y: 0,
            status: 0,
            instruction_bytes: [0; 3],
            instruction_byte_count: 0,
            data_bytes: [(0, 0); 5],
            data_byte_count: 0,
            buf_writer: BufWriter::new(Vec::new()),
        }
    }

    fn dump(&mut self) -> Result<()> {
        let disassembly = self.disassemble()?;

        let w = &mut self.buf_writer;
        write!(w, "{:04X}  ", self.pc)?;
        for i in 0..self.instruction_byte_count {
            write!(w, "{:02X} ", self.instruction_bytes[i])?;
        }
        for _i in self.instruction_byte_count..3 {
            write!(w, "   ")?;
        }

        write!(w, "{:32}", disassembly)?;

        writeln!(
            w,
            " A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:{:3},{:3} CYC:{}",
            self.a,
            self.x,
            self.y,
            self.status & (!StatusFlags::Break.bits()),
            self.sp,
            self.cycle * 3 / 341,
            self.cycle * 3 % 341,
            self.cycle
        )?;
        Ok(())
    }

    fn translate(&self, instruction: Instruction) -> String {
        match instruction {
            Instruction::ISC => "ISB".to_string(),
            _ => instruction.to_string(),
        }
    }

    fn disassemble(&self) -> Result<String> {
        use crate::cpu::instructions::Mode::*;

        let (instruction, mode, _, _) = decode(self.instruction_bytes[0]);
        let official = if get_official_status(instruction, self.instruction_bytes[0]) {
            ' '
        } else {
            '*'
        };

        let mut result = format!("{}{} ", official, self.translate(instruction));
        match mode {
            Abs => result.push_str(
                format!(
                    "${:02X}{:02X}{}",
                    self.instruction_bytes[2],
                    self.instruction_bytes[1],
                    self.format_abs()
                )
                .as_str(),
            ),
            AbsX => result.push_str(
                format!(
                    "${:02X}{:02X},X{}",
                    self.instruction_bytes[2],
                    self.instruction_bytes[1],
                    self.format_indexed_abs()
                )
                .as_str(),
            ),
            AbsY => result.push_str(
                format!(
                    "${:02X}{:02X},Y{}",
                    self.instruction_bytes[2],
                    self.instruction_bytes[1],
                    self.format_indexed_abs()
                )
                .as_str(),
            ),
            A => result.push('A'),
            Imm => result.push_str(format!("#${:02X}", self.instruction_bytes[1]).as_str()),
            Imp => result.push_str(""),
            AbsInd => result.push_str(
                format!(
                    "(${:02X}{:02X}){}",
                    self.instruction_bytes[2],
                    self.instruction_bytes[1],
                    self.format_abs_ind()
                )
                .as_str(),
            ),
            IndX => result.push_str(
                format!(
                    "(${:02X},X){}",
                    self.instruction_bytes[1],
                    self.format_ind_x()
                )
                .as_str(),
            ),
            IndY => result.push_str(
                format!(
                    "(${:02X}),Y{}",
                    self.instruction_bytes[1],
                    self.format_ind_y()
                )
                .as_str(),
            ),
            Rel => result.push_str(format!("${:02X}", self.compute_rel()).as_str()),
            Zp => result.push_str(
                format!("${:02X}{}", self.instruction_bytes[1], self.format_zp()).as_str(),
            ),
            Zpx => result.push_str(
                format!(
                    "${:02X},X{}",
                    self.instruction_bytes[1],
                    self.format_indexed_zp()
                )
                .as_str(),
            ),
            Zpy => result.push_str(
                format!(
                    "${:02X},Y{}",
                    self.instruction_bytes[1],
                    self.format_indexed_zp()
                )
                .as_str(),
            ),
            Status => unreachable!("Monitoring Status address mode"),
            SP => unreachable!("Monitoring SP address mode"),
            X => unreachable!("Monitoring X address mode"),
            Y => unreachable!("Monitoring Y address mode"),
        };

        Ok(result)
    }

    fn compute_rel(&self) -> u16 {
        let pc = self.pc.wrapping_add(2);
        let offset = self.instruction_bytes[1];
        if offset & 0b10000000 != 0 {
            let offset = (!offset) + 1;
            pc.wrapping_sub(offset as u16)
        } else {
            pc.wrapping_add(offset as u16)
        }
    }

    fn format_abs(&self) -> String {
        if self.data_byte_count >= 1 {
            format!(" = {:02X}", self.data_bytes[0].1)
        } else {
            String::new()
        }
    }

    fn format_indexed_abs(&self) -> String {
        if self.data_byte_count >= 1 {
            format!(
                " @ {:04X} = {:02X}",
                self.data_bytes[0].0, self.data_bytes[0].1
            )
        } else {
            String::new()
        }
    }

    fn format_ind_x(&self) -> String {
        if self.data_byte_count >= 3 {
            format!(
                " @ {:02X} = {:04X} = {:02X}",
                self.data_bytes[0].0 & 0xFF,
                self.data_bytes[2].0,
                self.data_bytes[2].1
            )
        } else {
            String::new()
        }
    }

    fn format_abs_ind(&self) -> String {
        if self.data_byte_count >= 2 {
            format!(
                " = {:02X}{:02X}",
                self.data_bytes[1].1, self.data_bytes[0].1
            )
        } else {
            String::new()
        }
    }

    fn format_ind_y(&self) -> String {
        if self.data_byte_count >= 3 {
            format!(
                " = {:02X}{:02X} @ {:04X} = {:02X}",
                self.data_bytes[1].1,
                self.data_bytes[0].1,
                self.data_bytes[2].0,
                self.data_bytes[2].1
            )
        } else {
            String::new()
        }
    }

    fn format_zp(&self) -> String {
        if self.data_byte_count >= 1 {
            format!(" = {:02X}", self.data_bytes[0].1)
        } else {
            String::new()
        }
    }

    fn format_indexed_zp(&self) -> String {
        if self.data_byte_count >= 1 {
            format!(
                " @ {:02X} = {:02X}",
                self.data_bytes[0].0 & 0xFF,
                self.data_bytes[0].1
            )
        } else {
            String::new()
        }
    }
}

fn get_official_status(instruction: Instruction, op_code: u8) -> bool {
    use Instruction::*;

    !matches!(
        (op_code, instruction),
        (0x1A, NOP)
            | (0x3A, NOP)
            | (0x5A, NOP)
            | (0x7A, NOP)
            | (0xDA, NOP)
            | (0xFA, NOP)
            | (0x80, NOP)
            | (0x82, NOP)
            | (0x89, NOP)
            | (0xC2, NOP)
            | (0xE2, NOP)
            | (0x0C, NOP)
            | (0x1C, NOP)
            | (0x3C, NOP)
            | (0x5C, NOP)
            | (0x7C, NOP)
            | (0xDC, NOP)
            | (0xFC, NOP)
            | (0x04, NOP)
            | (0x44, NOP)
            | (0x64, NOP)
            | (0x14, NOP)
            | (0x34, NOP)
            | (0x54, NOP)
            | (0x74, NOP)
            | (0xD4, NOP)
            | (0xF4, NOP)
            | (0xEB, SBC)
            | (_, ANC)
            | (_, ARR)
            | (_, ASR)
            | (_, DCP)
            | (_, ISC)
            | (_, JAM)
            | (_, LAS)
            | (_, LAX)
            | (_, RLA)
            | (_, RRA)
            | (_, SAX)
            | (_, SBX)
            | (_, SHA)
            | (_, SHS)
            | (_, SHX)
            | (_, SHY)
            | (_, SLO)
            | (_, SRE)
            | (_, XXA)
    )
}

impl Monitor for NestestMonitor {
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
        self.started = true;
        self.cycle = cycle;
        self.pc = pc;
        self.sp = sp;
        self.a = a;
        self.x = x;
        self.y = y;
        self.status = status;
        self.instruction_byte_count = 0;
        self.data_byte_count = 0;
        Ok(())
    }

    fn fetch_instruction_byte(&mut self, byte: u8) -> Result<()> {
        self.instruction_bytes[self.instruction_byte_count] = byte;
        self.instruction_byte_count += 1;
        Ok(())
    }

    fn read_data_byte(&mut self, addr: u16, data: u8) -> Result<()> {
        // read/modify/write instructions read from then write to the same
        // location. In that case we don't need to create another entry
        // for the same address
        if self.data_byte_count == 0 || self.data_bytes[self.data_byte_count - 1].0 != addr {
            self.data_bytes[self.data_byte_count] = (addr, data);
            self.data_byte_count += 1;
        }

        Ok(())
    }

    fn end_instruction(&mut self) -> Result<()> {
        if self.started {
            self.dump()?;
            self.started = false;
        }
        Ok(())
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

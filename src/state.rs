use std::{
    convert::TryInto,
    fmt::{self, Display, Formatter},
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom, Write},
    mem::size_of,
};

use crate::{
    gb::GameBoy,
    info::{name, version},
};

pub trait Serialize {
    fn save(&self, buffer: &mut Vec<u8>);
    fn load(&mut self, data: &mut Cursor<Vec<u8>>);
}

pub trait State {
    fn from_gb(gb: &GameBoy) -> Self;
}

pub struct BeesState {
    footer: BeesFooter,
    name: BeesName,
    info: BeesInfo,
    core: BeesCore,
}

impl BeesState {
    pub fn description(&self, column_length: usize) -> String {
        let emulator_l = format!("{:width$}", "Emulator", width = column_length);
        let title_l: String = format!("{:width$}", "Title", width = column_length);
        format!(
            "{}  {}\n{}  {}\n",
            emulator_l,
            self.name.name,
            title_l,
            self.info.title(),
        )
    }
}

impl Serialize for BeesState {
    fn save(&self, buffer: &mut Vec<u8>) {
        self.name.save(buffer);
        self.info.save(buffer);
        self.core.save(buffer);
        self.footer.save(buffer);
    }

    fn load(&mut self, data: &mut Cursor<Vec<u8>>) {
        // moves the cursor to the end of the file
        // to read the footer, and then places the
        // the cursor in the start of the BEES data
        // according to the footer information
        data.seek(SeekFrom::End(-8)).unwrap();
        self.footer.load(data);
        data.seek(SeekFrom::Start(self.footer.start_offset as u64))
            .unwrap();

        self.name.load(data);
        self.info.load(data);
        self.core.load(data);
    }
}

impl State for BeesState {
    fn from_gb(gb: &GameBoy) -> Self {
        Self {
            footer: BeesFooter::default(), // @TODO: check if this makes sense
            name: BeesName::from_gb(gb),
            info: BeesInfo::from_gb(gb),
            core: BeesCore::from_gb(gb),
        }
    }
}

impl Default for BeesState {
    fn default() -> Self {
        Self {
            footer: BeesFooter::default(),
            name: BeesName::default(),
            info: BeesInfo::default(),
            core: BeesCore::default(),
        }
    }
}

impl Display for BeesState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description(9))
    }
}

pub struct BeesBlockHeader {
    magic: String,
    size: u32,
}

impl BeesBlockHeader {
    pub fn new(magic: String, size: u32) -> Self {
        Self { magic, size }
    }
}

impl Serialize for BeesBlockHeader {
    fn save(&self, buffer: &mut Vec<u8>) {
        buffer.write_all(&self.magic.as_bytes()).unwrap();
        buffer.write_all(&self.size.to_le_bytes()).unwrap();
    }

    fn load(&mut self, data: &mut Cursor<Vec<u8>>) {
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.magic = String::from_utf8(Vec::from(buffer)).unwrap();
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.size = u32::from_le_bytes(buffer.try_into().unwrap());
    }
}

pub struct BeesBuffer {
    size: u32,
    offset: u32,
}

pub struct BeesFooter {
    start_offset: u32,
    magic: u32,
}

impl BeesFooter {
    pub fn new(start_offset: u32, magic: u32) -> Self {
        Self {
            start_offset,
            magic,
        }
    }
}

impl Serialize for BeesFooter {
    fn save(&self, buffer: &mut Vec<u8>) {
        buffer.write_all(&self.start_offset.to_le_bytes()).unwrap();
        buffer.write_all(&self.magic.to_le_bytes()).unwrap();
    }

    fn load(&mut self, data: &mut Cursor<Vec<u8>>) {
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.start_offset = u32::from_le_bytes(buffer.try_into().unwrap());
        let mut buffer = [0x00; 4];
        data.read_exact(&mut buffer).unwrap();
        self.magic = u32::from_le_bytes(buffer.try_into().unwrap());
    }
}

impl Default for BeesFooter {
    fn default() -> Self {
        Self::new(0x00, 0x53534542)
    }
}

pub struct BeesName {
    header: BeesBlockHeader,
    name: String,
}

impl BeesName {
    pub fn new(name: String) -> Self {
        Self {
            header: BeesBlockHeader::new(String::from("NAME"), name.len() as u32),
            name,
        }
    }
}

impl Serialize for BeesName {
    fn save(&self, buffer: &mut Vec<u8>) {
        self.header.save(buffer);
        buffer.write_all(self.name.as_bytes()).unwrap();
    }

    fn load(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.load(data);
        let mut buffer = vec![0x00; self.header.size as usize];
        data.read_exact(&mut buffer).unwrap();
        self.name = String::from_utf8(Vec::from(buffer)).unwrap();
    }
}

impl State for BeesName {
    fn from_gb(_: &GameBoy) -> Self {
        Self::new(format!("{} v{}", name(), version()))
    }
}

impl Default for BeesName {
    fn default() -> Self {
        Self::new(String::from(""))
    }
}

pub struct BeesInfo {
    header: BeesBlockHeader,
    title: [u8; 16],
    checksum: [u8; 2],
}

impl BeesInfo {
    pub fn new(title: &[u8], checksum: &[u8]) -> Self {
        Self {
            header: BeesBlockHeader::new(
                String::from("INFO"),
                title.len() as u32 + checksum.len() as u32,
            ),
            title: title.try_into().unwrap(),
            checksum: checksum.try_into().unwrap(),
        }
    }

    pub fn title(&self) -> String {
        String::from_utf8(Vec::from(&self.title[..])).unwrap()
    }
}

impl Serialize for BeesInfo {
    fn save(&self, buffer: &mut Vec<u8>) {
        self.header.save(buffer);
        buffer.write_all(&self.title).unwrap();
        buffer.write_all(&self.checksum).unwrap();
    }

    fn load(&mut self, data: &mut Cursor<Vec<u8>>) {
        self.header.load(data);
        data.read_exact(&mut self.title).unwrap();
        data.read_exact(&mut self.checksum).unwrap();
    }
}

impl State for BeesInfo {
    fn from_gb(gb: &GameBoy) -> Self {
        Self::new(
            &gb.cartridge_i().rom_data()[0x134..=0x143],
            &gb.cartridge_i().rom_data()[0x14e..=0x14f],
        )
    }
}

impl Default for BeesInfo {
    fn default() -> Self {
        Self::new(&[0_u8; 16], &[0_u8; 2])
    }
}

pub struct BeesCore {
    header: BeesBlockHeader,

    major: u16,
    minor: u16,

    model: u32,

    pc: u16,
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    sp: u16,

    ime: u8,
    ie: u8,
    // 0 = running; 1 = halted; 2 = stopped
    execution_mode: u8,
    _padding: u8,

    io_registers: [u8; 128],

    ram: BeesBuffer,
    vram: BeesBuffer,
    mbc_ram: BeesBuffer,
    oam: BeesBuffer,
    hram: BeesBuffer,
    background_palettes: BeesBuffer,
    object_palettes: BeesBuffer,
}

impl BeesCore {
    pub fn new(model: u32, pc: u16, af: u16, bc: u16, de: u16, hl: u16, sp: u16) -> Self {
        Self {
            header: BeesBlockHeader::new(
                String::from("CORE"),
                ((size_of::<u16>() * 2)
                    + size_of::<u32>()
                    + (size_of::<u16>() * 6)
                    + (size_of::<u8>() * 4)
                    + (size_of::<u8>() * 128)
                    + ((size_of::<u32>() + size_of::<u32>()) * 7)) as u32,
            ),
            major: 1,
            minor: 1,
            model,
            pc,
            af,
            bc,
            de,
            hl,
            sp,
            ime: 0,
            ie: 0,
            execution_mode: 0,
            _padding: 0,
            io_registers: [0x00; 128],
            ram: BeesBuffer { size: 0, offset: 0 },
            vram: BeesBuffer { size: 0, offset: 0 },
            mbc_ram: BeesBuffer { size: 0, offset: 0 },
            oam: BeesBuffer { size: 0, offset: 0 },
            hram: BeesBuffer { size: 0, offset: 0 },
            background_palettes: BeesBuffer { size: 0, offset: 0 },
            object_palettes: BeesBuffer { size: 0, offset: 0 },
        }
    }
}

impl Serialize for BeesCore {
    fn save(&self, buffer: &mut Vec<u8>) {}

    fn load(&mut self, data: &mut Cursor<Vec<u8>>) {}
}

impl State for BeesCore {
    fn from_gb(gb: &GameBoy) -> Self {
        Self::new(
            0x0002_u32, //GB_MODEL_DMG_B
            gb.cpu_i().pc(),
            gb.cpu_i().af(),
            gb.cpu_i().bc(),
            gb.cpu_i().de(),
            gb.cpu_i().hl(),
            gb.cpu_i().sp(),
        )
    }
}

impl Default for BeesCore {
    fn default() -> Self {
        Self::new(
            0x0002_u32, //GB_MODEL_DMG_B
            0x0000_u16, 0x0000_u16, 0x0000_u16, 0x0000_u16, 0x0000_u16, 0x0000_u16,
        )
    }
}

pub fn save_state_file(file_path: &str, gb: &GameBoy) {
    let mut file = File::create(file_path).unwrap();
    let data = save_state(gb);
    file.write_all(&data).unwrap();
}

pub fn save_state(gb: &GameBoy) -> Vec<u8> {
    let mut data: Vec<u8> = vec![];
    BeesState::from_gb(gb).save(&mut data);
    data
}

pub fn load_state_file(file_path: &str, gb: &GameBoy) {
    let mut file = File::open(file_path).unwrap();
    let mut data = vec![];
    file.read_to_end(&mut data).unwrap();
    load_state(&data, gb);
}

pub fn load_state(data: &Vec<u8>, gb: &GameBoy) {
    let mut state = BeesState::default();
    state.load(&mut Cursor::new(data.to_vec()));
    print!("{}", state);
}

#[derive(Copy, Clone)]
pub enum CpuFlag {
    /// carry
    C = 0b00010000,
    /// half-carry
    H = 0b00100000,
    /// substraction
    N = 0b01000000,
    /// zero - indicates that result was zero
    Z = 0b10000000,
}

// TODO remove file
pub fn set_flag(f: u8, flag: CpuFlag, value: bool) -> u8 {
    if value {
        f | flag as u8
    } else {
        f & !(flag as u8)
    }
}

pub fn get_ticks(instruction: u8) -> u32 {
    match instruction {
        0x0 => 4,

        // LD n,nn
        0x01 => 12,
        0x11 => 12,
        0x21 => 12,
        0x31 => 12,

        // LD (nn), SP
        0x08 => 20,

        // LD NN, A
        0x02 => 8,
        0x12 => 8,
        0xea => 16,

        // LDH (n),A
        0xe0 => 12,

        // LDH A,(n)
        0xf0 => 12,

        // LDI (HL), A
        0x22 => 8,
        // LDD (HL), A
        0x32 => 8,

        // LDD A, (HL)
        0x3a => 8,
        // LDI A, (HL)
        0x2a => 8,

        // LDHL SP,n
        0xf8 => 12,

        // LD A,n
        0x7f => 4,
        0x78 => 4,
        0x79 => 4,
        0x7a => 4,
        0x7b => 4,
        0x7c => 4,
        0x7d => 4,
        0x0a => 8,
        0x1a => 8,
        0x7e => 8,
        0x3e => 8,

        // B
        0x47 => 4,
        0x40 => 4,
        0x41 => 4,
        0x42 => 4,
        0x43 => 4,
        0x44 => 4,
        0x45 => 4,
        0x46 => 8,
        0x06 => 8,

        // C
        0x4f => 4,
        0x48 => 4,
        0x49 => 4,
        0x4a => 4,
        0x4b => 4,
        0x4c => 4,
        0x4d => 4,
        0x4e => 8,
        0x0e => 8,

        // D
        0x57 => 4,
        0x50 => 4,
        0x51 => 4,
        0x52 => 4,
        0x53 => 4,
        0x54 => 4,
        0x55 => 4,
        0x56 => 8,
        0x16 => 8,

        // E
        0x5f => 4,
        0x58 => 4,
        0x59 => 4,
        0x5a => 4,
        0x5b => 4,
        0x5c => 4,
        0x5d => 4,
        0x5e => 8,
        0x1e => 8,

        // H
        0x67 => 4,
        0x60 => 4,
        0x61 => 4,
        0x62 => 4,
        0x63 => 4,
        0x64 => 4,
        0x65 => 4,
        0x66 => 8,
        0x26 => 8,

        // L
        0x6f => 4,
        0x68 => 4,
        0x69 => 4,
        0x6A => 4,
        0x6B => 4,
        0x6C => 4,
        0x6D => 4,
        0x6E => 8,
        0x2e => 8,

        // (HL)
        0x77 => 8,
        0x70 => 8,
        0x71 => 8,
        0x72 => 8,
        0x73 => 8,
        0x74 => 8,
        0x75 => 8,
        0x36 => 12,

        0xfa => 16,

        // LD A, (C)
        0xf2 => 8,

        // LD (C), A
        0xe2 => 8,

        // LD SP, HL
        0xf9 => 8,

        // ADD
        0x87 => 4,
        0x80 => 4,
        0x81 => 4,
        0x82 => 4,
        0x83 => 4,
        0x84 => 4,
        0x85 => 4,
        0x86 => 8,
        0xc6 => 8,

        0x09 => 8,
        0x19 => 8,
        0x29 => 8,
        0x39 => 8,

        // ADC
        0x8f => 4,
        0x88 => 4,
        0x89 => 4,
        0x8a => 4,
        0x8b => 4,
        0x8c => 4,
        0x8d => 4,
        0x8e => 8,
        0xce => 8,

        // SUB n
        0x96 => 8,
        0x90..=0x97 => 4,

        0xd6 => 8,

        // SBC
        0x9f => 4,
        0x98 => 4,
        0x99 => 4,
        0x9a => 4,
        0x9b => 4,
        0x9c => 4,
        0x9d => 4,
        0x9e => 8,

        // INC nn
        0x03 => 8,
        0x13 => 8,
        0x23 => 8,
        0x33 => 8,

        // DEC nn
        0x0B => 8,
        0x1B => 8,
        0x2B => 8,
        0x3B => 8,

        // INC n
        0x3c => 4,
        0x04 => 4,
        0x0c => 4,
        0x14 => 4,
        0x1c => 4,
        0x24 => 4,
        0x2c => 4,
        0x34 => 12,

        // DEC
        0x3d => 4,
        0x05 => 4,
        0x0d => 4,
        0x15 => 4,
        0x1d => 4,
        0x25 => 4,
        0x2d => 4,
        0x35 => 12,

        // AND n
        0xa7 => 4,
        0xa0 => 4,
        0xa1 => 4,
        0xa2 => 4,
        0xa3 => 4,
        0xa4 => 4,
        0xa5 => 4,
        0xa6 => 8,
        0xe6 => 8,

        0xe8 => 16,

        // OR n
        0xb7 => 4,
        0xb0 => 4,
        0xb1 => 4,
        0xb2 => 4,
        0xb3 => 4,
        0xb4 => 4,
        0xb5 => 4,
        0xb6 => 8,

        0xf6 => 8,

        // XOR n
        0xaf => 4,
        0xa8 => 4,
        0xa9 => 4,
        0xaa => 4,
        0xab => 4,
        0xac => 4,
        0xad => 4,
        0xae => 8,
        0xee => 8,

        // CP n
        0xbf => 4,
        0xb8 => 4,
        0xb9 => 4,
        0xba => 4,
        0xbb => 4,
        0xbc => 4,
        0xbd => 4,

        0xbe => 8,

        0xfe => 8,

        // Interrupts
        0xf3 => 4,
        0xfb => 4,

        // PUSH
        0xf5 => 16,
        0xc5 => 16,
        0xd5 => 16,
        0xe5 => 16,

        // POP
        0xf1 => 12,
        0xc1 => 12,
        0xd1 => 12,
        0xe1 => 12,

        // CPL
        0x2f => 4,
        // SCF
        0x37 => 4,

        // HALT
        0x76 => 4,

        0xcb => panic!("cb cycles not supported"),

        0xc3 => 16,

        0xc2 => 12, // If cc is true, 16 else 12.
        0xca => 12, // If cc is true, 16 else 12.
        0xd2 => 12, // If cc is true, 16 else 12.
        0xda => 12, // If cc is true, 16 else 12.

        0xe9 => 4,

        0x18 => 12, // 12

        0x20 => 8, // If cc is true, 12 else 8
        0x28 => 8, // If cc is true, 12 else 8
        0x30 => 8, // If cc is true, 12 else 8
        0x38 => 8, // If cc is true, 12 else 8

        0xcd => 24, // 24

        0xc4 => 12, // If cc is true, 24 else 12
        0xcc => 12, // If cc is true, 24 else 12
        0xd4 => 12, // If cc is true, 24 else 12
        0xdc => 12, // If cc is true, 24 else 12

        //
        0xc9 => 16, // RET 16

        0xc0 => 8, // If cc is true, 20 else 8.
        0xc8 => 8, // If cc is true, 20 else 8.
        0xd0 => 8, // If cc is true, 20 else 8.
        0xd8 => 8, // If cc is true, 20 else 8.

        0xd9 => 16, // RETI 16

        0x07 => 4,
        0x0f => 4,
        0x1f => 4,
        0x17 => 4,

        0x3f => 4, // CCF

        // RST n
        0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => 16,

        _ => {
            panic!("missing operator CPU cycles{:#x}", instruction);
        }
    }
}

pub fn get_cb_ticks(cb_instruction: u8) -> u32 {
    match cb_instruction {
        // RLC
        0x06 => 16,
        0x00..=0x07 => 8,
        // RRC
        0x0e => 16,
        0x08..=0x0f => 8,
        // RR
        0x1e => 16,
        0x18..=0x1f => 8,

        // RL
        0x16 => 16,
        0x10..=0x17 => 8,

        // SWAP
        0x36 => 16,
        0x37 => 8,

        // SLA
        0x23 | 0x27 => 8,

        // SRA n
        0x28 | 0x2a => 8,

        // SRL n
        0x3e => 16,
        0x38..=0x3f => 8,

        // RES
        0x86 | 0x96 | 0xa6 | 0xb6 => 16,
        0x8e | 0x9e | 0xae | 0xbe => 16,
        0x80..=0xbf => 8,

        // SET
        0xc6 | 0xd6 | 0xe6 | 0xf6 => 16,
        0xce | 0xde | 0xee | 0xfe => 16,
        0xc0..=0xff => 8,

        // BIT b,r
        0x46 | 0x56 | 0x66 | 0x76 => 12,
        0x4e | 0x5e | 0x6e | 0x7e => 12,
        0x40..=0x7f => 8,

        _ => {
            panic!("missing cb operator CPU cycles{:#x}", cb_instruction);
        }
    }
}

use std::io::{self, Read, Write};
use std::net::TcpStream;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketType {
    Message = 1,
    Command = 2,
}

impl PacketType {
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(PacketType::Message),
            2 => Some(PacketType::Command),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Header {
    pub packet_type: PacketType,
    pub length: u32,
}

impl Header {
    pub const SIZE: usize = 5;

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.packet_type as u8;
        bytes[1..5].copy_from_slice(&self.length.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: [u8; Self::SIZE]) -> io::Result<Self> {
        let packet_type = PacketType::from_u8(bytes[0])
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid Packet Type"))?;

        let length = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);

        Ok(Header { packet_type, length })
    }
}

pub fn send_packet(stream: &mut TcpStream, p_type: PacketType, payload: &[u8]) -> io::Result<()> {
    let header = Header {
        packet_type: p_type,
        length: payload.len() as u32,
    };

    stream.write_all(&header.to_bytes())?;
    stream.write_all(payload)?;
    stream.flush()
}

pub fn receive_packet(stream: &mut TcpStream) -> io::Result<(Header, Vec<u8>)> {
    let mut header_bytes = [0u8; Header::SIZE];
    stream.read_exact(&mut header_bytes)?;

    let header = Header::from_bytes(header_bytes)?;

    let mut payload = vec![0u8; header.length as usize];
    stream.read_exact(&mut payload)?;

    Ok((header, payload))
}

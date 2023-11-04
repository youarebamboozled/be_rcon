use crc::crc32;


pub fn create_login_packet(password: &str) -> Vec<u8> {
    let mut login_packet = vec![0x42, 0x45, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00];
    login_packet.extend_from_slice(password.as_bytes());

    // Calculate CRC32 Checksum of the payload (excluding header)
    let checksum = crc::crc32::checksum_ieee(&login_packet[6..]);
    login_packet[2..6].copy_from_slice(&checksum.to_le_bytes());
    login_packet
}

pub fn create_command_packet(sequence: u8, command: &str) -> Vec<u8> {
    let mut command_packet = vec![0x42, 0x45, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x01, sequence];
    command_packet.extend_from_slice(command.as_bytes());

    // Calculate CRC32 Checksum of the payload (excluding header)
    let checksum = crc32::checksum_ieee(&command_packet[6..]);
    command_packet[2..6].copy_from_slice(&checksum.to_le_bytes());
    command_packet
}

pub fn create_server_message_ack_packet(sequence: u8) -> Vec<u8> {
    let mut packet = vec![0x42, 0x45, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x02, sequence];

    let checksum = crc::crc32::checksum_ieee(&packet[6..]);
    packet[2..6].copy_from_slice(&checksum.to_le_bytes());
    packet
}

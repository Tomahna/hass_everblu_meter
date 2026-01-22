use std::process::exit;

use crc::Crc;
use log::{debug, trace};

// Remove the start- and stop-bits in the bitstream, also decode oversampled bit 0xF0 => 1,0
// 01234567 ###01234 567###01 234567## #0123456 (# -> Start/Stop bit)
// is decoded to:
// 76543210 76543210 76543210 76543210
pub fn decode_4bitpbit_serial(rx_buffer: &[u8], l_total_byte: u16) -> Vec<u8> {
    let mut decoded = [0u8; 200];
    let mut bit_cnt: u8 = 0;
    let mut bit_cnt_flush_s8: i8 = 0;
    let mut bit_pol: u8;
    let mut dest_bit_cnt: u8 = 0;
    let mut dest_byte_cnt: usize = 0;
    let mut current_rx_byte: u8;

    // set 1st bit polarity
    bit_pol = rx_buffer[0] & 0x80; // initialize with 1st bit state

    for i in 0..l_total_byte {
        current_rx_byte = rx_buffer[i as usize];
        trace!("{}", current_rx_byte);

        for _j in 0..8 {
            if (current_rx_byte & 0x80) == bit_pol {
                bit_cnt += 1;
            } else if bit_cnt == 1 {
                // previous bit was a glitch so bit has not really changed
                bit_pol = current_rx_byte & 0x80; // restore correct bit polarity
                bit_cnt = (bit_cnt_flush_s8 + 1) as u8; // hope that previous bit was correctly decoded
            } else {
                // bit polarity has changed
                bit_cnt_flush_s8 = bit_cnt as i8;
                bit_cnt = (bit_cnt + 2) / 4;
                bit_cnt_flush_s8 -= bit_cnt as i8 * 4;

                for _k in 0..bit_cnt {
                    // insert the number of decoded bit
                    if dest_bit_cnt < 8 {
                        // if data byte
                        decoded[dest_byte_cnt] >>= 1;
                        decoded[dest_byte_cnt] |= bit_pol;
                    }
                    dest_bit_cnt += 1;

                    if dest_bit_cnt == 10 && bit_pol == 0 {
                        debug!("stop bit error10");
                        exit(1); // TODO better error handling
                    }
                    if dest_bit_cnt >= 11 && bit_pol == 0 {
                        // start bit
                        dest_bit_cnt = 0;
                        debug!(
                            "dec[{}]={} {}",
                            i, dest_byte_cnt, decoded[dest_byte_cnt]
                        );
                        dest_byte_cnt += 1;
                    }
                }
                bit_pol = current_rx_byte & 0x80;
                bit_cnt = 1;
            }
            current_rx_byte <<= 1;
        } // scan TX_bit
    } // scan TX_byte
    decoded[0..dest_byte_cnt + 1].to_vec() // TODO not sure about that but the rest is 0 so ...
}

/**
 * Reverses the bit order of the input data and adds a start bit before and a stop bit
 * after each byte.
 *
 * @param input_buffer Points to the unencoded data.
 * @param input_buffer Number of bytes of unencoded data.
 * @param output_buffer Points to the encoded data.
 */
pub fn encode2serial_1_3(input: [u8; 19]) -> [u8; 30] {
    // Adds a start and stop bit and reverses the bit order.
    // 76543210 76543210 76543210 76543210
    // is encoded to:
    // #0123456 7###0123 4567###0 1234567# ##012345 6s7# (# -> Start/Stop bit)
    let mut output: [u8; 30] = [0; 30];
    let mut j = 0;

    for i in 0..(input.len() * 8) {
        if i % 8 == 0 {
            if i > 0 {
                // Insert stop bit (3 bits set to 1)
                for _ in 0..3 {
                    let bytepos = j / 8;
                    let bitpos = j % 8;
                    output[bytepos] |= 1 << (7 - bitpos);
                    j += 1;
                }
            }

            // Insert start bit (0)
            let bytepos = j / 8;
            let bitpos = j % 8;
            output[bytepos] &= !(1 << (7 - bitpos));
            j += 1;
        }

        let bytepos = i / 8;
        let bitpos = i % 8;
        let mask = 1 << bitpos;

        if (input[bytepos] & mask) > 0 {
            let out_bytepos = j / 8;
            let out_bitpos = 7 - (j % 8);
            output[out_bytepos] |= 1 << out_bitpos;
        } else {
            let out_bytepos = j / 8;
            let out_bitpos = 7 - (j % 8);
            output[out_bytepos] &= !(1 << out_bitpos);
        }

        j += 1;
    }

    // Insert additional stop bits until end of byte
    while j % 8 > 0 {
        let bytepos = j / 8;
        let bitpos = 7 - (j % 8);
        output[bytepos] |= 1 << bitpos;
        j += 1;
    }

    let final_bytepos = j / 8;
    output[final_bytepos] = 0xFF;

    output
}

pub fn make_radian_master_req(year: u8, serial: u32) -> [u8; 39] {
    let mut to_encode: [u8; 19] = [
        0x13, 0x10, 0x00, 0x45, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x45, 0x20, 0x0A, 0x50, 0x14, 0x00,
        0x0A, 0x40, 0xFF, 0xFF,
    ];

    let synch_pattern: [u8; 9] = [0x50, 0x00, 0x00, 0x00, 0x03, 0xFF, 0xFF, 0xFF, 0xFF];

    // Set year and serial number
    to_encode[4] = year;
    to_encode[5] = ((serial & 0x00FF0000) >> 16) as u8;
    to_encode[6] = ((serial & 0x0000FF00) >> 8) as u8;
    to_encode[7] = (serial & 0x000000FF) as u8;

    // Calculate CRC
    let crc = Crc::<u16>::new(&crc::CRC_16_KERMIT).checksum(&to_encode[0..to_encode.len() - 2]);
    to_encode[to_encode.len() - 1] = ((crc & 0xFF00) >> 8) as u8;
    to_encode[to_encode.len() - 2] = (crc & 0x00FF) as u8;

    // Encode to serial
    let encoded = encode2serial_1_3(to_encode);

    let mut result: [u8; 39] = [0; 39];
    result[..9].copy_from_slice(&synch_pattern);
    result[9..].copy_from_slice(&encoded);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    #[test]
    fn encode_works() {
        let input_buffer: [u8; 19] = [
            0x13, 0x10, 0x00, 0x45, 0x10, 0x10, 0x97, 0x8c, 0x00, 0x45, 0x20, 0x0a, 0x50, 0x14,
            0x00, 0x0a, 0x40, 0xf1, 0xe3,
        ];
        let encoded_buffer_hex = "64704700751704704774f18f00751702728705714700728701747f63ffff";
        let encoded_buffer = encode2serial_1_3(input_buffer);
        assert_eq!(hex::encode(encoded_buffer), encoded_buffer_hex);
    }

    #[test]
    fn decode_works() {
        let encoded_buffer_hex = "00fffff0ff8780078007fc000000003fe1e1e001e1ff000000f00ff8078780007fc00001e3c1fe001e0f000ff0000000007fc3c7c007c3fe01ffe0001ff0ffff0000ff800787807ffc0003ffffffe000000001ff0f0000000ff87fff80007fc000000003fe00001e1e1ff0f0f00fffff8000078007fc000000003fe0000001e0ff00ff800007f87c7c7c003fe3e0000001ff00f0ff000ff87f8000007fc00003c003fe1e1fe0001ff0ff000f00ff807ff87807fc00003fc03fe01e01fe01ff00f00ff00ff800007f807fc3fc03fc03fe00001fe01ff00f0000f0ff8078000787fc003c3fc03fe1e001fe01ff0000000007f8000000003fc01fe00001ff00e00f000ff8007800007fc3c0000003fe1e00001e1ff000f00000ff8000000007fc000000003fe000000001ff000000000ff8000000007fc000000003fe00000001fff00000000fff800000003ffe00000001fff00000000fff800000007ffc00000003ffe00000001fff00000000fff800000007ffc00000003ffe00000001fff00000000fff800000007ffc0003fe3fffe1f1e0001fff0ffff80007f8000000003fe003fe1fffff000fff00fff87fff80007fc000000003fe01e01e001ff0f000ff0fff87fff80007fc000000003fe01ffffe01ff000ff00ffff87fff80007fc000000003fe01fe01e00ff0fff007ffff87fffc0003fe000000001ff0f1fffff0ff8078000007fc00003c003fe000000001ff0f0f000ffff87ffff8007fc00003c003fe000000001ff0f0f000ffff8007800787fc00003e003fe000000001ff0f8ff8ff8ffc07ffc3fc3fc00001e001ff000000000ff87f87ffffffc3c3c0003ffe00001e001ff000000000ff87f8078787fc3c0003c3ffe00001e001ff000000000ff8078787ffffc03c3ffc3ffe00001f001ff0000000007f87c787f807fc3fc03e3fffe00001f000ff8000000007fc03fffc00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let decoded_buffer_hex = "7c110045200a501400450e0f94f800010f0050e51000400615011a03100d232e3032323033304242343100000612040141040000000000008080808080808080808080808080d8850f00ec9c0f0012b10f003ecc0f0026e70f007d021000c51f1000c54410006d6e1000fb85100053a11000eaba100035d31000f0";
        let rx_buffer = hex::decode(encoded_buffer_hex).unwrap();
        let decoded_buffer = decode_4bitpbit_serial(&rx_buffer, 690);
        assert_eq!(hex::encode(decoded_buffer), decoded_buffer_hex);
    }

    #[test]
    fn make_radian_master_req_works() {
        assert_eq!(
            hex::encode(make_radian_master_req(16, 1087372)),
            "5000000003ffffffff64704700751704704774f18f00751702728705714700728701747f63ffff"
        );
    }
}

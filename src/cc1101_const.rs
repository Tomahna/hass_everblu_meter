#![allow(unused)]

pub const TX_LOOP_OUT: u16 = 300;
/*---------------------------[CC1100 - R/W offsets]------------------------------*/
pub const WRITE_SINGLE_BYTE: u8 = 0x00;
pub const WRITE_BURST: u8 = 0x40;
pub const READ_SINGLE_BYTE: u8 = 0x80;
pub const READ_BURST: u8 = 0xC0;

/*-------------------------[CC1100 - config register]----------------------------*/
pub const IOCFG2: u8 = 0x00;                                    // GDO2 output pin configuration
pub const IOCFG1: u8 = 0x01;                                    // GDO1 output pin configuration
pub const IOCFG0: u8 = 0x02;                                    // GDO0 output pin configuration
pub const FIFOTHR: u8 = 0x03;                                    // RX FIFO and TX FIFO thresholds
pub const SYNC1: u8 = 0x04;                                    // Sync word, high byte
pub const SYNC0: u8 = 0x05;                                    // Sync word, low byte
pub const PKTLEN: u8 = 0x06;                                    // Packet length
pub const PKTCTRL1: u8 = 0x07;                                  	// Packet automation control
pub const PKTCTRL0: u8 = 0x08;                                  	// Packet automation control
pub const ADDRR: u8 = 0x09;                                    // Device address
pub const CHANNR: u8 = 0x0A;                                    // Channel number
pub const FSCTRL1: u8 = 0x0B;                                   	// Frequency synthesizer control
pub const FSCTRL0: u8 = 0x0C;                                   	// Frequency synthesizer control
pub const FREQ2: u8 = 0x0D;                                    // Frequency control word, high byte
pub const FREQ1: u8 = 0x0E;                                    // Frequency control word, middle byte
pub const FREQ0: u8 = 0x0F;                                    // Frequency control word, low byte

pub const MDMCFG4: u8 = 0x10;                                   	// Modem configuration
pub const MDMCFG3: u8 = 0x11;                                   	// Modem configuration
pub const MDMCFG2: u8 = 0x12;                                   	// Modem configuration
pub const MDMCFG1: u8 = 0x13;                                   	// Modem configuration
pub const MDMCFG0: u8 = 0x14;                                   	// Modem configuration
pub const DEVIATN: u8 = 0x15;                                   	// Modem deviation setting
pub const MCSM2: u8 = 0x16;                                    // Main Radio Cntrl State Machine config
pub const MCSM1: u8 = 0x17;                                    // Main Radio Cntrl State Machine config
pub const MCSM0: u8 = 0x18;                                    // Main Radio Cntrl State Machine config
pub const FOCCFG: u8 = 0x19;	                                // Frequency Offset Compensation config
pub const BSCFG: u8 = 0x1A;                                    // Bit Synchronization configuration
pub const AGCCTRL2: u8 = 0x1B;                                    // AGC control
pub const AGCCTRL1: u8 = 0x1C;                                    // AGC control
pub const AGCCTRL0: u8 = 0x1D;                                    // AGC control
pub const WOREVT1: u8 = 0x1E;                                   	// High byte Event 0 timeout
pub const WOREVT0: u8 = 0x1F;                                   	// Low byte Event 0 timeout

pub const WORCTRL: u8 = 0x20;                                   	// Wake On Radio control
pub const FREND1: u8 = 0x21;                                    // Front end RX configuration
pub const FREND0: u8 = 0x22;                                    // Front end TX configuration
pub const FSCAL3: u8 = 0x23;                                    // Frequency synthesizer calibration
pub const FSCAL2: u8 = 0x24;                                    // Frequency synthesizer calibration
pub const FSCAL1: u8 = 0x25;                                    // Frequency synthesizer calibration
pub const FSCAL0: u8 = 0x26;                                    // Frequency synthesizer calibration
pub const RCCTRL1: u8 = 0x27;                                   	// RC oscillator configuration
pub const RCCTRL0: u8 = 0x28;                                   	// RC oscillator configuration
pub const FSTEST: u8 = 0x29;                                   	// Frequency synthesizer cal control
pub const PTEST: u8 = 0x2A;                                    // Production test
pub const AGCTEST: u8 = 0x2B;                                   	// AGC test
pub const TEST2: u8 = 0x2C;                                    // Various test settings
pub const TEST1: u8 = 0x2D;                                    // Various test settings
pub const TEST0: u8 = 0x2E;                                    // Various test settings
/*----------------------------[END config register]------------------------------*/
/*-------------------------[CC1100 - status register]----------------------------*/
/* 0x3? is replace by 0xF? because for status register burst bit shall be set */
pub const PARTNUM_ADDR: u8 = 0xF0;				// Part number
pub const VERSION_ADDR: u8 = 0xF1;				// Current version number
pub const FREQEST_ADDR: u8 = 0xF2;				// Frequency offset estimate
pub const LQI_ADDR: u8 = 0xF3;				// Demodulator estimate for link quality
pub const RSSI_ADDR: u8 = 0xF4;				// Received signal strength indication
pub const MARCSTATE_ADDR: u8 = 0xF5;				// Control state machine state
pub const WORTIME1_ADDR: u8 = 0xF6;				// High byte of WOR timer
pub const WORTIME0_ADDR: u8 = 0xF7;				// Low byte of WOR timer
pub const PKTSTATUS_ADDR: u8 = 0xF8;				// Current GDOx status and packet status
pub const VCO_VC_DAC_ADDR: u8 = 0xF9;				// Current setting from PLL cal module
pub const TXBYTES_ADDR: u8 = 0xFA;				// Underflow and # of bytes in TXFIFO
pub const RXBYTES_ADDR: u8 = 0xFB;				// Overflow and # of bytes in RXFIFO
//----------------------------[END status register]-------------------------------
pub const RXBYTES_MASK: u8 = 0x7F;        // Mask "# of bytes" field in _RXBYTES

/*---------------------------[CC1100-command strobes]----------------------------*/
pub const SRES: u8 = 0x30;                                    // Reset chip
pub const SFSTXON: u8 = 0x31;                                    // Enable/calibrate freq synthesizer
pub const SXOFF: u8 = 0x32;                                    // Turn off crystal oscillator.
pub const SCAL: u8 = 0x33;                                    // Calibrate freq synthesizer & disable
pub const SRX: u8 = 0x34;                                    // Enable RX.
pub const STX: u8 = 0x35;                                    // Enable TX.
pub const SIDLE: u8 = 0x36;                                    // Exit RX / TX
pub const SAFC: u8 = 0x37;                                    // AFC adjustment of freq synthesizer
pub const SWOR: u8 = 0x38;                                    // Start automatic RX polling sequence
pub const SPWD: u8 = 0x39;                                    // Enter pwr down mode when CSn goes hi
pub const SFRX: u8 = 0x3A;                                    // Flush the RX FIFO buffer.
pub const SFTX: u8 = 0x3B;                                    // Flush the TX FIFO buffer.
pub const SWORRST: u8 = 0x3C;                                    // Reset real time clock.
pub const SNOP: u8 = 0x3D;                                    // No operation.
/*----------------------------[END command strobes]------------------------------*/

pub const PATABLE_ADDR: u8 = 0x3E;                                    // Pa Table Adress
pub const TX_FIFO_ADDR: u8 = 0x3F;
pub const RX_FIFO_ADDR: u8 = 0xBF;

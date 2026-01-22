use std::sync::atomic::{AtomicU8, Ordering};
use std::thread::sleep;
use std::time::Duration;
use rppal::{gpio, spi};
use serde::Serialize;
use crate::delay::Timeout;
use crate::radian::{make_radian_master_req, decode_4bitpbit_serial};
use crate::cc1101_const::*;
use log::debug;

static PA: [u8; 8] = [0x60, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

static CC1101_STATUS_STATE: AtomicU8 = AtomicU8::new(0); // TODO we can probably do better than having a shared mutable state
static CC1101_STATUS_FIFO_FREEBYTE: AtomicU8 = AtomicU8::new(0);
static CC1101_STATUS_FIFO_READBYTE: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Serialize)]
pub struct MeterData {
    pub liters: i32,
    pub reads_counter: i32, // how many times the meter has been readed
    pub battery_left: i32, //in months
    pub time_start: i32, // like 8am
    pub time_end: i32 // like 4pm
} 

pub struct CC1101 {
   gdo0: gpio::InputPin,
   // gdo2: gpio::InputPin,
   spi: spi::Spi 
}

impl CC1101 {
    pub fn new() -> Self {
        // Initialize GPIO
        debug!("Initializing GPIO");
        let gpio = gpio::Gpio::new().expect("Failed to initialize GPIO");
        let gdo0 = gpio.get(17).unwrap().into_input(); //pin 0 (GDO0) = BCM GPIO 17 (physical pin 11)
        let _gdo2 = gpio.get(27).unwrap().into_input(); //pin 2 (GDO2) = BCM GPIO 27 (physical pin 13) TODO never read (is it used ?)

        // to use SPI pi@MinePi ~ $ gpio unload spi  then gpio load spi   
        // sinon pas de MOSI ni pas de CSn , buffer de 4kB
        let speed = 100000u32;
        let spi = spi::Spi::new(spi::Bus::Spi0, spi::SlaveSelect::Ss0, speed, spi::Mode::Mode0).unwrap();

        let cc1101 = CC1101{
            gdo0,
            // gdo2,
            spi
        };
        cc1101.reset();
        sleep(Duration::from_millis(1));
        cc1101.cc1101_configure_rf_0();

        debug!("{}", cc1101.version());
        debug!("{}", cc1101.registers_settings());

        cc1101 
    }

    //------------------[write register]--------------------------------
    fn hal_rf_write_reg(&self, reg_addr: u8, value: u8) -> u8
    {
        let mut tbuf = [reg_addr | WRITE_SINGLE_BYTE, value];
        self.data_rw(&mut tbuf);
        CC1101_STATUS_FIFO_FREEBYTE.store(tbuf[1]&0x0F, Ordering::Relaxed);
        CC1101_STATUS_STATE.store((tbuf[0]>>4)&0x0F, Ordering::Relaxed);

        return 0;
    }

    fn hal_rf_read_reg(&self, spi_instr: u8) -> u8
    {
        let mut rbuf = [spi_instr | READ_SINGLE_BYTE, 0];
        //errata Section 3. You have to make sure that you read the same value of the register twice in a row before you evaluate it otherwise you might read a value that is a mix of 2 state values.
        self.data_rw(&mut rbuf) ;
        CC1101_STATUS_FIFO_READBYTE.store(rbuf[0]&0x0F, Ordering::Relaxed);
        CC1101_STATUS_STATE.store((rbuf[0]>>4)&0x0F, Ordering::Relaxed);
        return rbuf[1];
    }


    fn spi_read_burst_reg(&self, spi_instr: u8, buffer: &mut [u8]) {
        let len = buffer.len();
        let mut rbuf = vec![0u8; len + 1];
        rbuf[0] = spi_instr | READ_BURST;
        self.data_rw(&mut rbuf);
        buffer.copy_from_slice(&rbuf[1..]);
        CC1101_STATUS_FIFO_READBYTE.store(rbuf[0] & 0x0F, Ordering::Relaxed);
        CC1101_STATUS_STATE.store((rbuf[0] >> 4) & 0x0F, Ordering::Relaxed);
    }

    fn spi_write_burst_reg(&self, spi_instr: u8, p_arr: &[u8], len: u8) {
        let mut tbuf = vec![0u8; (len + 1) as usize];
        tbuf[0] = spi_instr | WRITE_BURST;
        tbuf[1..].copy_from_slice(&p_arr);
        self.data_rw(&mut tbuf);
        CC1101_STATUS_FIFO_FREEBYTE.store(tbuf[len as usize] & 0x0F, Ordering::Relaxed);
        CC1101_STATUS_STATE.store((tbuf[len as usize] >> 4) & 0x0F, Ordering::Relaxed);
    }

    fn cmd(&self, spi_instr: u8) {
        let mut tbuf: [u8; 1] = [0];
        tbuf[0] = spi_instr | WRITE_SINGLE_BYTE;
        self.data_rw(&mut tbuf);
        CC1101_STATUS_STATE.store((tbuf[0] >> 4) & 0x0F, Ordering::Relaxed);
    }

    //---------------[CC1100 reset functions "200us"]-----------------------
    fn reset(&self) {			// reset defined in cc1100 datasheet §19.1
        // CS should be high from gpio load spi command
        /* commented car ne fonctionne pas avec wiringPi a voir avec BCM2835 ..
        digitalWrite(cc1101_CSn, 0);     		// CS low
        pinMode (cc1101_CSn, OUTPUT); 
        delayMicroseconds(30);
        digitalWrite(cc1101_CSn, 1);      	// CS high
        delayMicroseconds(100);	 // min 40us
        //Pull CSn low and wait for SO to go low
        digitalWrite(cc1101_CSn, 0);     		// CS low
        delayMicroseconds(30);
        */

        self.cmd(SRES);	//GDO0 pin should output a clock signal with a frequency of CLK_XOSC/192.
        //periode 1/7.417us= 134.8254k  * 192 --> 25.886477M
        //10 periode 73.83 = 135.4463k *192 --> 26Mhz
        sleep(Duration::from_millis(1)); //1ms for getting chip to reset properly

        self.cmd(SFTX);   //flush the TX_fifo content -> a must for interrupt handling
        self.cmd(SFRX);	//flush the RX_fifo content -> a must for interrupt handling	
    }

    fn cc1101_configure_rf_0(&self) {
        //
        // Rf settings for CC1101
        //
        self.hal_rf_write_reg(IOCFG2,0x0D);  //GDO2 Output Pin Configuration : Serial Data Output
        self.hal_rf_write_reg(IOCFG0,0x06);  //GDO0 Output Pin Configuration : Asserts when sync word has been sent / received, and de-asserts at the end of the packet.
        self.hal_rf_write_reg(FIFOTHR,0x47); //0x4? adc with bandwith< 325khz
        self.hal_rf_write_reg(SYNC1,0x55);   //01010101
        self.hal_rf_write_reg(SYNC0,0x00);   //00000000 

        //self.hal_rf_write_reg(PKTCTRL1,0x80);//Preamble quality estimator threshold=16  ; APPEND_STATUS=0; no addr check
        self.hal_rf_write_reg(PKTCTRL1,0x00);//Preamble quality estimator threshold=0   ; APPEND_STATUS=0; no addr check
        self.hal_rf_write_reg(PKTCTRL0,0x00);//fix length , no CRC
        self.hal_rf_write_reg(FSCTRL1,0x08); //Frequency Synthesizer Control

        self.hal_rf_write_reg(FREQ2,0x10);   //Frequency Control Word, High Byte  Base frequency = 433.82
        self.hal_rf_write_reg(FREQ1,0xAF);   //Frequency Control Word, Middle Byte
        self.hal_rf_write_reg(FREQ0,0x75); //Frequency Control Word, Low Byte la fréquence reel etait 433.790 (centre)
        //self.hal_rf_write_reg(FREQ0,0xC1); //Frequency Control Word, Low Byte rasmobo 814 824 (KO) ; minepi 810 820 (OK)
        //self.hal_rf_write_reg(FREQ0,0x9B); //rasmobo 808.5  -16  pour -38
        //self.hal_rf_write_reg(FREQ0,0xB7);   //rasmobo 810 819.5 OK
        //mon compteur F1 : 433809500  F2 : 433820000   deviation +-5.25khz depuis 433.81475M

        self.hal_rf_write_reg(MDMCFG4,0xF6); //Modem Configuration   RX filter BW = 58Khz
        self.hal_rf_write_reg(MDMCFG3,0x83); //Modem Configuration   26M*((256+83h)*2^6)/2^28 = 2.4kbps 
        self.hal_rf_write_reg(MDMCFG2,0x02); //Modem Configuration   2-FSK;  no Manchester ; 16/16 sync word bits detected
        self.hal_rf_write_reg(MDMCFG1,0x00); //Modem Configuration num preamble 2=>0 , Channel spacing_exp
        self.hal_rf_write_reg(MDMCFG0,0x00); /*# MDMCFG0 Channel spacing = 25Khz*/
        self.hal_rf_write_reg(DEVIATN,0x15);  //5.157471khz 
        //self.hal_rf_write_reg(MCSM1,0x0F);   //CCA always ; default mode RX
        self.hal_rf_write_reg(MCSM1,0x00);   //CCA always ; default mode IDLE
        self.hal_rf_write_reg(MCSM0,0x18);   //Main Radio Control State Machine Configuration
        self.hal_rf_write_reg(FOCCFG,0x1D);  //Frequency Offset Compensation Configuration
        self.hal_rf_write_reg(BSCFG,0x1C);   //Bit Synchronization Configuration
        self.hal_rf_write_reg(AGCCTRL2,0xC7);//AGC Control
        self.hal_rf_write_reg(AGCCTRL1,0x00);//AGC Control
        self.hal_rf_write_reg(AGCCTRL0,0xB2);//AGC Control
        self.hal_rf_write_reg(WORCTRL,0xFB); //Wake On Radio Control
        self.hal_rf_write_reg(FREND1,0xB6);  //Front End RX Configuration
        self.hal_rf_write_reg(FSCAL3,0xE9);  //Frequency Synthesizer Calibration
        self.hal_rf_write_reg(FSCAL2,0x2A);  //Frequency Synthesizer Calibration
        self.hal_rf_write_reg(FSCAL1,0x00);  //Frequency Synthesizer Calibration
        self.hal_rf_write_reg(FSCAL0,0x1F);  //Frequency Synthesizer Calibration
        self.hal_rf_write_reg(TEST2,0x81);   //Various Test Settings link to adc retention
        self.hal_rf_write_reg(TEST1,0x35);   //Various Test Settings link to adc retention
        self.hal_rf_write_reg(TEST0,0x09);   //Various Test Settings link to adc retention

        self.spi_write_burst_reg(PATABLE_ADDR, &PA, 8);
    }

    fn read_gdo0(&self) -> gpio::Level{
        self.gdo0.read() 
    }

    // fn read_gdo2() -> gpio::Level{
    //    GDO2.get().unwrap().lock().unwrap().read() 
    // }

    fn data_rw(&self, data: &mut [u8]) -> i32 {
        // copy data to be written
        let write = data.to_vec();

        match self.spi.transfer(data, &write) {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("SPI transfer failed: {}", e);
                -1
            }
        }
    }

    fn rssi_convert2dbm(rssi_dec: u8) -> i8 {
        if rssi_dec >= 128 {
            ((rssi_dec as i16 - 256) / 2 - 74) as i8  // rssi_offset via datasheet
        } else {
            ((rssi_dec as i16) / 2 - 74) as i8
        }
    }

    // Configure cc1101 in receive mode
    fn cc1101_rec_mode(&self) {
        let mut marcstate: u8;
        self.cmd(SIDLE);  // sets to idle first. must be in
        self.cmd(SRX);    // writes receive strobe (receive mode)
        marcstate = 0xFF;   // set unknown/dummy state value
        while marcstate != 0x0D && marcstate != 0x0E && marcstate != 0x0F {  // 0x0D = RX
            marcstate = self.hal_rf_read_reg(MARCSTATE_ADDR);  // read out state of cc1100 to be sure in RX
        }
    }

    fn version(&self) -> String{
        format!(
            r#"
                CC1101 Partnumber: 0x{:02X}"
                CC1101 Version != 00 or 0xFF  : 0x{:02X}
            "#,
            self.hal_rf_read_reg(PARTNUM_ADDR),
            self.hal_rf_read_reg(VERSION_ADDR)
        )
    }

    fn registers_settings(&self) -> String {
        let mut config_reg_verify: [u8; 47] = [0; 47]; //47 registers
        let mut patable_verify: [u8; 8] = [0; 8];

        self.spi_read_burst_reg(0, &mut config_reg_verify);			//reads all 47 config register from cc1100	"359.63us"
        self.spi_read_burst_reg(PATABLE_ADDR, &mut patable_verify);				//reads output power settings from cc1100	"104us"

        format!(
            r#"
                Config Register in hex:
                [ 0   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F]
                {:02X?}
                {:02X?}
                {:02X?}

                PaTable:
                {:02X?}
            "#,
            &config_reg_verify[0..16],
            &config_reg_verify[16..32],
            &config_reg_verify[32..47],
            &patable_verify
        )
    }


    fn _is_look_like_radian_frame(buffer: &[u8]) -> bool {
        for &byte in buffer {
            if byte == 0xFF {
                return true;
            }
        }
        false
    }

    fn parse_meter_report(decoded_buffer: &[u8]) -> Result<MeterData, String> {
        let size = decoded_buffer.len();
        let mut data = MeterData {
            liters: 0,
            reads_counter: 0,
            battery_left: 0,
            time_start: 0,
            time_end: 0,
        };

        if size < 30 {
            return Err(format!("Failed to read meter data, decode_buffer size {} is < 30", size))
        } 
        if size >= 30 {
            let b18 = decoded_buffer[18] as i32;
            let b19 = decoded_buffer[19] as i32;
            let b20 = decoded_buffer[20] as i32;
            let b21 = decoded_buffer[21] as i32;
            data.liters = b18 + b19 * 256 + b20 * 65536 + b21 * 16777216;
        }
        if size >= 48 {
            data.reads_counter = decoded_buffer[48] as i32;
            data.battery_left = decoded_buffer[31] as i32;
            data.time_start = decoded_buffer[44] as i32;
            data.time_end = decoded_buffer[45] as i32;
        }

        Ok(data)
    }

    // Check if Packet is received
    fn _check_packet_received(&self) -> bool {
        let mut rx_buffer: [u8; 100] = [0; 100];
        let mut l_nb_byte: u8;
        let mut pkt_len: u8 = 0;

        if self.read_gdo0() == gpio::Level::High {
            // get RF info at beginning of the frame
            while self.read_gdo0() == gpio::Level::High {
                sleep(Duration::from_millis(5));  // wait for some byte received
                l_nb_byte = self.hal_rf_read_reg(RXBYTES_ADDR) & RXBYTES_MASK;
                if l_nb_byte != 0 && (pkt_len + l_nb_byte) < 100 {
                    self.spi_read_burst_reg(RX_FIFO_ADDR, &mut rx_buffer[pkt_len as usize..(pkt_len + l_nb_byte) as usize]);
                    pkt_len += l_nb_byte;
                }
            }

            if Self::_is_look_like_radian_frame(&rx_buffer[..pkt_len as usize]) {
                debug!(
                    "bytes={} rssi={} lqi={} F_est={}",
                    pkt_len,
                    Self::rssi_convert2dbm(self.hal_rf_read_reg(RSSI_ADDR)),
                    self.hal_rf_read_reg(LQI_ADDR),
                    self.hal_rf_read_reg(FREQEST_ADDR)
                );
                debug!("{:02X?}", rx_buffer);
            } else {
                debug!(".")
            }

            return true;
        }
        false
    }

    fn _wait_for_packet(&self, milliseconds: i32) -> bool {
        for _ in 0..milliseconds {
            sleep(Duration::from_millis(1));  // in ms
            if self._check_packet_received() {
                return true;
            }
        }
        false
    }

    /*
    search for 0101010101010000b sync pattern then change data rate in order to get 4bit per bit
    search for end of sync pattern with start bit 1111111111110000b
    */
    fn receive_radian_frame(
        &self,
        size_byte: u16,
        rx_tmo_ms: u64,
        rx_buffer: &mut [u8]
    ) -> u16 {
        let mut timeout = Timeout::new(rx_tmo_ms);
        let mut l_byte_in_rx: u8 = 0;
        let mut l_total_byte: u16 = 0;
        let l_radian_frame_size_byte: u16 = (size_byte * (8 + 3)) / 8 + 1;

        if (l_radian_frame_size_byte * 4) as i32 > rx_buffer.len() as i32 {
            debug!("buffer too small");
            return 0;
        }

        self.cmd(SFRX);
        self.hal_rf_write_reg(MCSM1, 0x0F);    // CCA always ; default mode RX
        self.hal_rf_write_reg(MDMCFG2, 0x02);  // Modem Configuration   2-FSK;  no Manchester ; 16/16 sync word bits detected
        // configure to receive beginning of sync pattern
        self.hal_rf_write_reg(SYNC1, 0x55);    // 01010101
        self.hal_rf_write_reg(SYNC0, 0x50);    // 01010000
        self.hal_rf_write_reg(MDMCFG4, 0xF6);  // Modem Configuration   RX filter BW = 58Khz
        self.hal_rf_write_reg(MDMCFG3, 0x83);  // Modem Configuration   26M*((256+83h)*2^6)/2^28 = 2.4kbps
        self.hal_rf_write_reg(PKTLEN, 1);      // just one byte of synch pattern
        self.cc1101_rec_mode();

        while self.read_gdo0() == gpio::Level::Low && !timeout.has_timed_out(){
            timeout.delay(1);
        }
        if !timeout.has_timed_out() {
            debug!("GDO0!");
        } else {
            return 0;
        }

        while l_byte_in_rx == 0 && !timeout.has_timed_out() {
            timeout.delay(5);
            l_byte_in_rx = self.hal_rf_read_reg(RXBYTES_ADDR) & RXBYTES_MASK;
            if l_byte_in_rx != 0 {
                self.spi_read_burst_reg(RX_FIFO_ADDR, &mut rx_buffer[..l_byte_in_rx as usize]);  // Pull data
                debug!("{:02X?}", rx_buffer)
            }
        }
        if !timeout.has_timed_out() {
            debug!("1st synch received")
        } else {
            return 0;
        }

        debug!(
            "rssi={} lqi={} F_est={}",
            Self::rssi_convert2dbm(self.hal_rf_read_reg(RSSI_ADDR)), 
            self.hal_rf_read_reg(LQI_ADDR),
            self.hal_rf_read_reg(FREQEST_ADDR)
        );

        self.hal_rf_write_reg(SYNC1, 0xFF);    // 11111111
        self.hal_rf_write_reg(SYNC0, 0xF0);    // 11110000 la fin du synch pattern et le bit de start
        self.hal_rf_write_reg(MDMCFG4, 0xF8);  // Modem Configuration   RX filter BW = 58Khz
        self.hal_rf_write_reg(MDMCFG3, 0x83);  // Modem Configuration   26M*((256+83h)*2^8)/2^28 = 9.59kbps
        self.hal_rf_write_reg(PKTCTRL0, 0x02); // infinite packet len
        self.cmd(SFRX);
        self.cc1101_rec_mode();

        l_byte_in_rx = 1;
        while self.read_gdo0() == gpio::Level::Low && !timeout.has_timed_out() {
            timeout.delay(1);
        }
        if !timeout.has_timed_out() {
            debug!("GDO0!");
        } else {
            return 0;
        }

        while l_byte_in_rx > 0 && l_total_byte < (l_radian_frame_size_byte * 4) && !timeout.has_timed_out() {
            timeout.delay(5);
            l_byte_in_rx = self.hal_rf_read_reg(RXBYTES_ADDR) & RXBYTES_MASK;
            if l_byte_in_rx != 0 {
                let start = l_total_byte as usize;
                let end = start + l_byte_in_rx as usize;
                self.spi_read_burst_reg(RX_FIFO_ADDR, &mut rx_buffer[start..end]);  // Pull data
                l_total_byte += l_byte_in_rx as u16;
            }
        }
        if !timeout.has_timed_out() {
            debug!("frame received");
        } else {
            return 0;
        }

        // stop reception
        self.cmd(SFRX);
        self.cmd(SIDLE);

        // restore default reg
        self.hal_rf_write_reg(MDMCFG4, 0xF6);  // Modem Configuration   RX filter BW = 58Khz
        self.hal_rf_write_reg(MDMCFG3, 0x83);  // Modem Configuration   26M*((256+83h)*2^6)/2^28 = 2.4kbps
        self.hal_rf_write_reg(PKTCTRL0, 0x00); // fix packet len
        self.hal_rf_write_reg(PKTLEN, 38);
        self.hal_rf_write_reg(SYNC1, 0x55);    // 01010101
        self.hal_rf_write_reg(SYNC0, 0x00);    // 00000000

        l_total_byte
    }

    /*
    scenario_releve
    2s de WUP
    130ms : trame interrogation de l'outils de reléve   ______------|...............-----
    43ms de bruit
    34ms 0101...01
    14.25ms 000...000
    14ms 1111...11111
    83.5ms de data acquitement
    50ms de 111111
    34ms 0101...01
    14.25ms 000...000
    14ms 1111...11111
    582ms de data avec l'index

    l'outils de reléve doit normalement acquité
    */
    pub fn get_meter_data(&self, year: u8, serial: u32) -> Result<MeterData, String> {

        // let mut marcstate: u8 = 0xFF;
        let wupbuffer: [u8; 8] = [0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55];
        let mut wup2send: u8 = 77;
        let mut timeout = Timeout::new(TX_LOOP_OUT as u64); // TODO move TX_LOOP_OUT somewhere closer
        let mut rx_buffer: [u8; 1000] = [0; 1000];

        // let mut txbuffer: [u8; 100] = [0; 100];
        let txbuffer = make_radian_master_req(year, serial);

        self.hal_rf_write_reg(MDMCFG2, 0x00);  // clear MDMCFG2 to do not send preamble and sync
        self.hal_rf_write_reg(PKTCTRL0, 0x02); // infinite packet len
        self.spi_write_burst_reg(TX_FIFO_ADDR, &wupbuffer, 8);
        wup2send -= 1;
        self.cmd(STX);  // sends the data store into transmit buffer over the air
        timeout.delay(10);  // to give time for calibration
        let marcstate = self.hal_rf_read_reg(MARCSTATE_ADDR);  // to update CC1101_status_state
        debug!("MARCSTATE : raw:0x{}  0x{} free_byte:0x{} sts:0x{} sending 2s WUP...", marcstate, (marcstate & 0x1F), CC1101_STATUS_FIFO_FREEBYTE.load(Ordering::Relaxed), CC1101_STATUS_STATE.load(Ordering::Relaxed));

        while CC1101_STATUS_STATE.load(Ordering::Relaxed) == 0x02 && !timeout.has_timed_out() {  // in TX
            if wup2send != 0 {
                if wup2send < 0xFF {
                    if CC1101_STATUS_FIFO_FREEBYTE.load(Ordering::Relaxed) <= 10 {
                        // this give 10+20ms from previous frame : 8*8/2.4k=26.6ms  temps pour envoyer un wupbuffer
                        timeout.delay(20);
                    }
                    self.spi_write_burst_reg(TX_FIFO_ADDR, &wupbuffer, 8);
                    wup2send -= 1;
                }
            } else {
                sleep(Duration::from_millis(130));  // 130ms time to free 39bytes FIFO space
                self.spi_write_burst_reg(TX_FIFO_ADDR, &txbuffer, 39);
                debug!("{:02X?}", txbuffer);
                wup2send = 0xFF;
            }
            timeout.delay(10);
            self.hal_rf_read_reg(MARCSTATE_ADDR);  // read out state of cc1100 to be sure in IDLE and TX is finished this update also CC1101_status_state
        }

        debug!("{}ifree_byte:{} sts:{}", timeout.time, CC1101_STATUS_FIFO_FREEBYTE.load(Ordering::Relaxed), CC1101_STATUS_STATE.load(Ordering::Relaxed));
        self.cmd(SFTX);  // flush the Tx_fifo content this clear the status state and put sate machin in IDLE

        // end of transition restore default register
        self.hal_rf_write_reg(MDMCFG2, 0x02);  // Modem Configuration   2-FSK;  no Manchester ; 16/16 sync word bits detected
        self.hal_rf_write_reg(PKTCTRL0, 0x00); // fix packet len

        sleep(Duration::from_millis(30));  // 43ms de bruit
        // 34ms 0101...01  14.25ms 000...000  14ms 1111...11111  83.5ms de data acquitement
        if self.receive_radian_frame(0x12, 150, &mut rx_buffer) == 0 {
                    debug!("TMO on REC");
	}
	sleep(Duration::from_millis(30));  // 50ms de 111111  , mais on a 7+3ms de printf et xxms calculs
	// 34ms 0101...01  14.25ms 000...000  14ms 1111...11111  582ms de data avec l'index
	let rx_buffer_size = self.receive_radian_frame(0x7C, 700, &mut rx_buffer);
	if rx_buffer_size != 0 {
        debug!("{:02X?}", rx_buffer);

		let meter_data = decode_4bitpbit_serial(&rx_buffer, rx_buffer_size);
		Self::parse_meter_report(&meter_data)
	} else {
        Err("TMO on REC".to_string())
	}
}
}

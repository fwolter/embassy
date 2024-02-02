#![no_std]
#![no_main]
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::peripherals::*;
use embassy_stm32::{bind_interrupts, can, Config};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    FDCAN1_IT0 => can::IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => can::IT1InterruptHandler<FDCAN1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let config = Config::default();

    let peripherals = embassy_stm32::init(config);

    let mut can = can::Fdcan::new(peripherals.FDCAN1, peripherals.PA11, peripherals.PA12, Irqs);

    can.set_extended_filter(
        can::fd::filter::ExtendedFilterSlot::_0,
        can::fd::filter::ExtendedFilter::accept_all_into_fifo1(),
    );

    // 250k bps
    can.set_bitrate(250_000);

    // 1M bps
    can.set_fd_data_bitrate(1_000_000, false);

    info!("Configured");

    //let mut can = can.into_normal_mode();
    let mut can = can.into_internal_loopback_mode();

    let mut i = 0;
    let mut last_read_ts = embassy_time::Instant::now();

    loop {
        let frame = can::frame::ClassicFrame::new_extended(0x123456F, &[i; 8]).unwrap();
        info!("Writing frame");

        _ = can.write(&frame).await;

        match can.read().await {
            Ok((rx_frame, ts)) => {
                let delta = (ts - last_read_ts).as_millis();
                last_read_ts = ts;
                info!(
                    "Rx: {} {:02x} --- {}ms",
                    rx_frame.header().len(),
                    rx_frame.data()[0..rx_frame.header().len() as usize],
                    delta,
                )
            }
            Err(_err) => error!("Error in frame"),
        }

        Timer::after_millis(250).await;

        i += 1;
        if i > 2 {
            break;
        }
    }

    // Use the FD API's even if we don't get FD packets.
    loop {
        let frame = can::frame::FdFrame::new_extended(0x123456F, &[i; 16]).unwrap();
        info!("Writing frame using FD API");

        _ = can.write_fd(&frame).await;

        match can.read_fd().await {
            Ok((rx_frame, ts)) => {
                let delta = (ts - last_read_ts).as_millis();
                last_read_ts = ts;
                info!(
                    "Rx: {} {:02x} --- using FD API {}ms",
                    rx_frame.header().len(),
                    rx_frame.data()[0..rx_frame.header().len() as usize],
                    delta,
                )
            }
            Err(_err) => error!("Error in frame"),
        }

        Timer::after_millis(250).await;

        i += 1;
        if i > 4 {
            break;
        }
    }

    let (mut tx, mut rx) = can.split();
    // With split
    loop {
        let frame = can::frame::ClassicFrame::new_extended(0x123456F, &[i; 8]).unwrap();
        info!("Writing frame");
        _ = tx.write(&frame).await;

        match rx.read().await {
            Ok((rx_frame, ts)) => {
                let delta = (ts - last_read_ts).as_millis();
                last_read_ts = ts;
                info!(
                    "Rx: {} {:02x} --- {}ms",
                    rx_frame.header().len(),
                    rx_frame.data()[0..rx_frame.header().len() as usize],
                    delta,
                )
            }
            Err(_err) => error!("Error in frame"),
        }

        Timer::after_millis(250).await;

        i += 1;
    }
}

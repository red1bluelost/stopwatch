#![no_main]
#![no_std]

use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use stm32f4xx_hal::gpio::{ErasedPin, Output};

use core::fmt::Write as _;

use panic_probe as _;

type Lcd = stopwatch::LcdDriver<
    hd44780_driver::bus::FourBitBus<
        ErasedPin<Output>,
        ErasedPin<Output>,
        ErasedPin<Output>,
        ErasedPin<Output>,
        ErasedPin<Output>,
        ErasedPin<Output>,
    >,
>;

#[allow(unused)]
fn debug_print_time(current_time: time::PrimitiveDateTime) {
    rtt_target::rprintln!(
        "CURRENT TIME {:?} {:?} {:?} {:?}",
        current_time.year(),
        current_time.month() as u8,
        current_time.day(),
        current_time.as_hms(),
    );
}

fn lcd_print_time(
    lcd: &mut Lcd,
    delay: &mut (impl DelayUs<u16> + DelayMs<u8>),
    dt: time::PrimitiveDateTime,
) {
    write!(
        lcd.writer(delay).expect("failed to make writer"),
        "{}",
        stopwatch::RelTime::from_raw(dt),
    )
    .expect("failed to write datetime");
    // debug_print_time(dt);
}

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use super::{lcd_print_time, Lcd};

    use stopwatch::{RelTime, REL_TIME_ZERO};

    use stm32f4xx_hal::{
        gpio::{self, Edge, Input},
        pac::TIM3,
        prelude::*,
        rtc::{Event, Rtc},
        timer::DelayMs,
    };

    // Resources shared between tasks
    #[shared]
    struct Shared {
        stoptime: Option<RelTime>,
        rtc: Rtc,
        lcd: Lcd,
        delay: DelayMs<TIM3>,
    }

    // Local resources to specific tasks (cannot be shared)
    #[local]
    struct Local {
        board_button: gpio::ErasedPin<Input>,
        startstop_button: gpio::ErasedPin<Input>,
        // inc_button: gpio::ErasedPin<Input>,
        // dec_button: gpio::ErasedPin<Input>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        rtt_target::rtt_init_print!();

        let mut dp = ctx.device;

        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(8.MHz()).freeze();

        let mut rtc = Rtc::new(dp.RTC, &mut dp.PWR);
        rtc.set_datetime(&REL_TIME_ZERO.raw())
            .expect("bad set datetime");
        rtc.enable_wakeup(1.secs().into());
        rtc.listen(&mut dp.EXTI, Event::Wakeup);
        rtt_target::rprintln!("SETUP TIME!!!");

        // let delay1 = dp.TIM1.delay_ms(&clocks);

        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();
        let mut delay = dp.TIM3.delay_ms(&clocks);
        let lcd = hd44780_driver::HD44780::new_4bit(
            gpiob.pb8.into_push_pull_output().erase(),
            gpiob.pb5.into_push_pull_output().erase(),
            gpioa.pa6.into_push_pull_output().erase(),
            gpioa.pa7.into_push_pull_output().erase(),
            gpioa.pa8.into_push_pull_output().erase(),
            gpioa.pa9.into_push_pull_output().erase(),
            &mut delay,
        )
        .unwrap();
        let lcd = Lcd::new(lcd, 16, 2, &mut delay).unwrap();

        let mut syscfg = dp.SYSCFG.constrain();
        let gpioc = dp.GPIOC.split();
        let mut board_button = gpioc.pc13.into_pull_up_input().erase();
        board_button.make_interrupt_source(&mut syscfg);
        board_button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
        board_button.enable_interrupt(&mut dp.EXTI);

        let mut startstop_button = gpioa.pa10.into_pull_up_input().erase();
        startstop_button.make_interrupt_source(&mut syscfg);
        startstop_button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
        startstop_button.enable_interrupt(&mut dp.EXTI);

        rtt_target::rprintln!("DONE SETUP!!!");
        (
            Shared {
                stoptime: None,
                rtc,
                lcd,
                delay,
            },
            Local {
                board_button,
                startstop_button,
            },
        )
    }

    // #[idle(local = [led, delay], shared = [delayval])]
    // fn idle(mut ctx: idle::Context) -> ! {
    //     let led = ctx.local.led;
    //     let delay = ctx.local.delay;
    //     loop {
    //         led.set_high();
    //         delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
    //         led.set_low();
    //         delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
    //     }
    // }

    #[task(
        binds = EXTI15_10,
        local = [board_button, startstop_button],
        shared = [stoptime, rtc],
    )]
    fn board_button_handler(ctx: board_button_handler::Context) {
        let board_button_handler::LocalResources {
            board_button,
            startstop_button,
            ..
        } = ctx.local;
        let board_button_handler::SharedResources { stoptime, rtc, .. } =
            ctx.shared;

        if board_button.check_interrupt() {
            board_button.clear_interrupt_pending_bit();
            rtt_target::rprintln!("BOARD BUTTON!!");
        }
        if startstop_button.check_interrupt() {
            startstop_button.clear_interrupt_pending_bit();
            rtt_target::rprintln!("START/STOP BUTTON!!");
            (stoptime, rtc).lock(|stoptime, rtc| {
                if let Some(st) = stoptime.take() {
                    rtc.set_datetime(&st.raw()).expect("failed to set time");
                    rtc.enable_wakeup(1.secs().into())
                } else {
                    rtc.disable_wakeup();
                    let dt = rtc.get_datetime();
                    stoptime.replace(RelTime::from_raw(dt));
                }
            })
        }
    }

    #[task(binds = RTC_WKUP, shared = [stoptime, rtc, lcd, delay])]
    fn rtc_wakeup(mut ctx: rtc_wakeup::Context) {
        rtt_target::rprintln!("RTC INTERRUPT!!!!");
        let rtc_wakeup::SharedResources {
            stoptime,
            rtc,
            lcd,
            delay,
            ..
        } = &mut ctx.shared;

        let current_time = stoptime
            .lock(|st| st.map(|st| st.raw()))
            .unwrap_or_else(|| {
                rtc.lock(|rtc| {
                    let current_time = rtc.get_datetime();
                    rtc.clear_interrupt(Event::Wakeup);
                    current_time
                })
            });

        (lcd, delay)
            .lock(|lcd, delay| lcd_print_time(lcd, delay, current_time));
        // Your RTC wakeup interrupt handling code here
    }
}

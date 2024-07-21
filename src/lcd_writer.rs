use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use hd44780_driver::{bus::DataBus, HD44780};

use core::fmt;

pub struct LcdDriver<DB: DataBus> {
    lcd: HD44780<DB>,
}

impl<DB: DataBus> LcdDriver<DB> {
    pub fn new(
        mut lcd: HD44780<DB>,
        _rows: u8,
        _cols: u8,
        delay: &mut (impl DelayUs<u16> + DelayMs<u8>),
    ) -> hd44780_driver::error::Result<Self> {
        lcd.reset(delay)?;
        lcd.clear(delay)?;
        lcd.set_cursor_visibility(hd44780_driver::Cursor::Invisible, delay)?;
        lcd.set_cursor_blink(hd44780_driver::CursorBlink::Off, delay)?;
        Ok(Self { lcd })
    }

    pub fn writer<'lw, DL: DelayUs<u16> + DelayMs<u8>>(
        &'lw mut self,
        delay: &'lw mut DL,
    ) -> hd44780_driver::error::Result<LcdWriter<'lw, DB, DL>> {
        self.lcd.reset(delay)?;
        self.lcd.clear(delay)?;
        Ok(LcdWriter { lcd: self, delay })
    }
}

pub struct LcdWriter<'lw, DB: DataBus, DL> {
    pub lcd: &'lw mut LcdDriver<DB>,
    pub delay: &'lw mut DL,
}

impl<'lw, DB: DataBus, DL: DelayUs<u16> + DelayMs<u8>> fmt::Write
    for LcdWriter<'lw, DB, DL>
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let LcdDriver { ref mut lcd, .. } = *self.lcd;
        let nlp = s.find('\n');
        if let Some(nlp) = nlp {
            let (before, after) = s.split_at(nlp);
            lcd.write_str(before, self.delay).map_err(|_| fmt::Error)?;
            lcd.set_cursor_pos(40, self.delay).map_err(|_| fmt::Error)?;
            lcd.write_str(
                after.strip_prefix('\n').expect("cannot strip"),
                self.delay,
            )
            .map_err(|_| fmt::Error)?;
        } else {
            lcd.write_str(s, self.delay).map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}

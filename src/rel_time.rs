use num_integer::Integer as _;

#[derive(Copy, Clone)]
pub struct RelTime(time::PrimitiveDateTime);

lazy_static::lazy_static! {
    pub static ref REL_TIME_ZERO: RelTime = RelTime::zero();
}

const SECONDS_IN_MINUTE: u32 = 60;
const SECONDS_IN_HOUR: u32 = 60 * SECONDS_IN_MINUTE;
const SECONDS_IN_DAY: u32 = 24 * SECONDS_IN_HOUR;
const SECONDS_IN_YEAR: u32 = 365 * SECONDS_IN_DAY; // in 1970
const MINUTES_IN_HOUR: u32 = 60;

impl RelTime {
    pub fn zero() -> Self {
        Self::from_raw(time::PrimitiveDateTime::new(
            time::Date::from_ordinal_date(1970, 1).expect("bad calendar date"),
            time::Time::MIDNIGHT,
        ))
    }

    pub fn from_raw(dt: time::PrimitiveDateTime) -> Self {
        assert_eq!(dt.year(), 1970);
        Self(dt)
    }

    pub fn raw(self) -> time::PrimitiveDateTime {
        self.0
    }

    #[allow(clippy::let_and_return)]
    pub fn seconds(self) -> u32 {
        let (lh, lm, ls) = self.raw().as_hms();
        let [lh, lm, ls] = [lh, lm, ls].map(u32::from);
        let ls = lh * SECONDS_IN_HOUR + lm * SECONDS_IN_MINUTE + ls;

        let (eh, em, es) = REL_TIME_ZERO.raw().as_hms();
        let [eh, em, es] = [eh, em, es].map(u32::from);
        let es = eh * SECONDS_IN_HOUR + em * SECONDS_IN_MINUTE + es;

        let (ly, ld) = self.raw().to_ordinal_date();
        let (ey, ed) = REL_TIME_ZERO.raw().to_ordinal_date();
        assert_eq!(ly, 1970);
        assert_eq!(ey, 1970);
        let sec = u32::try_from(ly - ey).unwrap() * SECONDS_IN_YEAR
            + u32::from(ld - ed) * SECONDS_IN_DAY;

        let sec = sec + ls;
        let sec = sec - es;

        sec
    }

    pub fn add_hour(&mut self) {
        self.0 = self.raw() + time::Duration::hours(1)
    }

    pub fn add_minute(&mut self) {
        self.0 = self.raw() + time::Duration::minutes(1)
    }

    pub fn add_second(&mut self) {
        self.0 = self.raw() + time::Duration::seconds(1)
    }

    pub fn sub_hour(&mut self) {
        self.0 = self.raw() - time::Duration::hours(1)
    }

    pub fn sub_minute(&mut self) {
        self.0 = self.raw() - time::Duration::minutes(1)
    }

    pub fn sub_second(&mut self) {
        self.0 = self.raw() - time::Duration::seconds(1)
    }
}

impl core::fmt::Display for RelTime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let sec = self.seconds();
        let (min, sec) = sec.div_rem(&SECONDS_IN_MINUTE);
        let (hour, min) = min.div_rem(&MINUTES_IN_HOUR);
        write!(f, "{:02}:{:02}:{:02}", hour, min, sec)
    }
}

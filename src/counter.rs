use lazy_static::lazy_static;
use parking_lot::Mutex;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Weak;

#[doc(hidden)]
#[macro_export]
macro_rules! declare_counter_i64_impl {
    ($name:expr, $mode:ty, $reset:expr, $extra:expr) => {
        $crate::counter::AtomicCounter::<$mode> {
            __get_counter: || {
                thread_local! {
                    static COUNTER: std::sync::Arc<std::sync::atomic::AtomicI64> = {
                        let arc = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(0));
                        let mut list = $crate::counter::__COUNTERS_LIST.lock();
                        let cvec = list.entry($name.to_string()).or_insert((Vec::new(), 0, <$crate::counter::AtomicCounter<$mode> as $crate::counter::__CounterType>::MODE, $reset));
                        cvec.0.push(std::sync::Arc::downgrade(&arc));
                        arc
                    }
                }
                use std::ops::Deref;
                COUNTER.with(|c| {
                    unsafe {
                        &*(c.deref() as *const std::sync::atomic::AtomicI64)
                    }
                })
            },
            __extra: $extra,
            _phantom: std::marker::PhantomData
        }
    }
}

#[macro_export]
macro_rules! declare_counter_i64 {
    ($name:literal, $mode:ty, $reset:expr) => {
        $crate::declare_counter_i64_impl!($name, $mode, $reset, ())
    };
}

pub(crate) static COUNTER_SUFFIX: &str = "$COUNTER_83uRij";

#[macro_export]
macro_rules! declare_avg_counter_i64 {
    ($name:literal, $reset:expr) => {
        $crate::declare_counter_i64_impl!(
            $name,
            $crate::counter::AvgMode,
            $reset,
            $crate::declare_counter_i64_impl!(
                concat!($name, "$COUNTER_83uRij"),
                $crate::counter::SumMode,
                $reset,
                ()
            )
        )
    };
}

lazy_static! {
    #[doc(hidden)]
    pub static ref __COUNTERS_LIST: Mutex<HashMap<String, (Vec<Weak<AtomicI64>>, i64, __AcMode, bool)>> = Mutex::new(HashMap::new());
}

#[doc(hidden)]
pub trait AtomicCounterMode {
    type Extra;
}

pub struct SumMode {}
impl AtomicCounterMode for SumMode {
    type Extra = ();
}
pub struct MaxMode {}
impl AtomicCounterMode for MaxMode {
    type Extra = ();
}
pub struct MinMode {}
impl AtomicCounterMode for MinMode {
    type Extra = ();
}
pub struct AvgMode {}
impl AtomicCounterMode for AvgMode {
    type Extra = AtomicCounter<SumMode>;
}

pub struct AtomicCounter<MODE: AtomicCounterMode> {
    #[doc(hidden)]
    pub __get_counter: fn() -> &'static AtomicI64,
    #[doc(hidden)]
    pub __extra: MODE::Extra,
    #[doc(hidden)]
    pub _phantom: PhantomData<MODE>,
}

#[doc(hidden)]
#[derive(Eq, PartialEq)]
pub enum __AcMode {
    SUM,
    MAX,
    MIN,
    AVG,
}

#[doc(hidden)]
pub trait __CounterType {
    const MODE: __AcMode;
}

impl AtomicCounter<SumMode> {
    #[inline(always)]
    pub fn inc(&self) {
        self.inc_by(1);
    }

    #[inline(always)]
    pub fn inc_by(&self, value: i64) {
        (self.__get_counter)().fetch_add(value, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn sub(&self, value: i64) {
        (self.__get_counter)().fetch_sub(value, Ordering::Relaxed);
    }
}

impl AtomicCounter<AvgMode> {
    #[inline(always)]
    pub fn add_value(&self, value: i64) {
        (self.__get_counter)().fetch_add(value, Ordering::Relaxed);
        self.__extra.inc();
    }
}

impl __CounterType for AtomicCounter<SumMode> {
    const MODE: __AcMode = __AcMode::SUM;
}

impl AtomicCounter<MaxMode> {
    #[inline(always)]
    pub fn max(&self, val: i64) {
        (self.__get_counter)().fetch_max(val, Ordering::Relaxed);
    }
}

impl __CounterType for AtomicCounter<MaxMode> {
    const MODE: __AcMode = __AcMode::MAX;
}

impl AtomicCounter<MinMode> {
    #[inline(always)]
    pub fn min(&self, val: i64) {
        (self.__get_counter)().fetch_min(val, Ordering::Relaxed);
    }
}

impl __CounterType for AtomicCounter<MinMode> {
    const MODE: __AcMode = __AcMode::MIN;
}

impl __CounterType for AtomicCounter<AvgMode> {
    const MODE: __AcMode = __AcMode::AVG;
}

pub struct AtomicCounterGuardSum<'a> {
    value: i64,
    counter: &'a AtomicCounter<SumMode>,
}

impl<'a> AtomicCounterGuardSum<'a> {
    pub fn new(counter: &'a AtomicCounter<SumMode>, value: i64) -> Self {
        counter.inc_by(value);
        Self { value, counter }
    }
}

impl<'a> Drop for AtomicCounterGuardSum<'a> {
    fn drop(&mut self) {
        self.counter.sub(self.value);
    }
}

pub fn get_counter_value(name: &str) -> (i64, i64) {
    let mut counters = __COUNTERS_LIST.lock();

    let (ref mut vec, part_value, mode, reset) = if let Some(val) = counters.get_mut(name) {
        val
    } else {
        return (0, 0);
    };

    let reset_value = match mode {
        __AcMode::SUM => 0,
        __AcMode::MAX => 0,
        __AcMode::MIN => i64::MAX,
        __AcMode::AVG => 0,
    };

    if *reset {
        *part_value = reset_value;
    }

    let mut result = *part_value;

    vec.retain(|val| {
        if val.strong_count() > 0 {
            if let Some(value) = val.upgrade() {
                let value = value.swap(reset_value, Ordering::Relaxed);

                match mode {
                    __AcMode::SUM => {
                        result += value;
                    }
                    __AcMode::MAX => {
                        result = max(result, value);
                    }
                    __AcMode::MIN => {
                        result = min(result, value);
                    }
                    __AcMode::AVG => {
                        result += value;
                    }
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    });

    *part_value = result;

    let is_average = __AcMode::AVG == *mode;
    drop(counters);

    let counter = if is_average {
        get_counter_value(&(name.to_string() + COUNTER_SUFFIX)).0
    } else {
        0
    };

    (result, counter)
}

#[cfg(test)]
mod tests {
    use super::SumMode;
    #[test]
    fn alloc_test() {
        let _sum_counter = declare_counter_i64!("test_counter", SumMode, false);
    }
}

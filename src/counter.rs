use std::cmp::{max, min};
use std::collections::HashMap;
use std::marker::PhantomData;
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::thread_local;
use parking_lot::lock_api::RawMutex;
use lazy_static::lazy_static;

lazy_static! {
    #[doc(hidden)]
    pub static ref __COUNTERS_LIST: Mutex<HashMap<String, (Vec<Weak<AtomicU64>>, __AcMode)>> = Mutex::new(HashMap::new());
}


pub enum SumMode {}

pub enum MaxMode {}

pub enum MinMode {}

pub struct AtomicCounter<MODE> {
    #[doc(hidden)]
    pub __get_counter: fn() -> &'static AtomicU64,
    #[doc(hidden)]
    pub _phantom: PhantomData<MODE>,
}

#[doc(hidden)]
pub enum __AcMode {
    SUM,
    MAX,
    MIN,
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
    pub fn inc_by(&self, value: u64) {
        (self.__get_counter)().fetch_add(value, Ordering::Relaxed);
    }

    fn sub(&self, value: u64) {
        (self.__get_counter)().fetch_sub(value, Ordering::Relaxed);
    }
}

impl __CounterType for AtomicCounter<SumMode> { const MODE: __AcMode = __AcMode::SUM; }

impl AtomicCounter<MaxMode> {
    #[inline(always)]
    pub fn max(&self, val: u64) {
        (self.__get_counter)().fetch_max(val, Ordering::Relaxed);
    }
}

impl __CounterType for AtomicCounter<MaxMode> { const MODE: __AcMode = __AcMode::MAX; }

impl AtomicCounter<MinMode> {
    #[inline(always)]
    pub fn min(&self, val: u64) {
        (self.__get_counter)().fetch_min(val, Ordering::Relaxed);
    }
}

impl __CounterType for AtomicCounter<MinMode> { const MODE: __AcMode = __AcMode::MIN; }

pub struct AtomicCounterGuardSum<'a> {
    value: u64,
    counter: &'a AtomicCounter<SumMode>,
}

impl<'a> AtomicCounterGuardSum<'a> {
    pub fn new(counter: &'a AtomicCounter<SumMode>, value: u64) -> Self {
        counter.inc_by(value);
        Self {
            value,
            counter,
        }
    }
}

impl<'a> Drop for AtomicCounterGuardSum<'a> {
    fn drop(&mut self) {
        self.counter.sub(self.value);
    }
}

#[macro_export]
macro_rules! declare_counter_u64 {
    ($name:expr, $mode:ty) => {
        AtomicCounter::<$mode> {
            __get_counter: || {
                thread_local! {
                    static COUNTER: Arc<AtomicU64> = {
                        let arc = Arc::new(AtomicU64::new(0));
                        let mut list = $crate::counter::__COUNTERS_LIST.lock();
                        let mut cvec = list.entry($name.to_string()).or_insert((Vec::new(), <AtomicCounter<$mode> as $crate::counter::__CounterType>::MODE));
                        cvec.0.push(Arc::downgrade(&arc));
                        arc
                    }
                }
                use std::ops::Deref;
                COUNTER.with(|c| {
                    unsafe {
                        &*(c.deref() as *const AtomicU64)
                    }
                })
            },
            _phantom: std::marker::PhantomData
        }
    }
}

pub fn get_counter_value(name: &str) -> u64 {
    let mut counters = __COUNTERS_LIST.lock();

    let (ref mut vec, mode) = if let Some(val) = counters.get_mut(name) {
        val
    } else {
        return 0;
    };

    let mut result = match mode {
        __AcMode::SUM => { 0 },
        __AcMode::MAX => { 0 },
        __AcMode::MIN => { u64::MAX },
    };

    vec.retain(|val| {
        if val.strong_count() > 0 {
            if let Some(value) = val.upgrade() {
                match mode {
                    __AcMode::SUM => {
                        result += value.load(Ordering::Relaxed);
                    }
                    __AcMode::MAX => {
                        result = max(result, value.load(Ordering::Relaxed));
                    }
                    __AcMode::MIN => {
                        result = min(result, value.load(Ordering::Relaxed));
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


    result
}

use crate::counter::{get_counter_value, COUNTER_SUFFIX, __COUNTERS_LIST};
use bytesize::ByteSize;
use json::{object, JsonValue};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub fn enable_counters_logging(file: impl AsRef<Path>, interval: Duration) {
    let mut file = File::create(file).unwrap();
    let time = Instant::now();

    std::thread::spawn(move || {
        let mut keys = Vec::new();
        loop {
            sleep(interval);
            {
                keys.clear();
                let list = __COUNTERS_LIST.lock();
                keys.extend(list.keys().cloned());
            }

            let mut json_values = object! {};

            json_values["_time"] = time.elapsed().as_secs_f64().into();

            for name in &keys {
                // Skip average counters
                if name.ends_with(COUNTER_SUFFIX) {
                    continue;
                }

                let (value, avg_counter) = get_counter_value(&name);

                // Average, use floating point
                if avg_counter > 0 {
                    let avg_value = (value as f64) / (avg_counter as f64);
                    let values: [JsonValue; 2] = [
                        avg_value.into(),
                        format!("{}", ByteSize(avg_value as u64)).into(),
                    ];
                    json_values[name] = values.as_ref().into();
                } else {
                    let values: [JsonValue; 2] =
                        [value.into(), format!("{}", ByteSize(value)).into()];
                    json_values[name] = values.as_ref().into();
                }
            }

            writeln!(file, "{}", json_values);
        }
    });
}

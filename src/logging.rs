use std::fs::File;
use std::path::{Path};
use std::thread::sleep;
use std::time::{Duration, Instant};
use json::{JsonValue, object};
use crate::counter::{__COUNTERS_LIST, get_counter_value};
use std::io::Write;
use bytesize::ByteSize;

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
                let value = get_counter_value(&name);

                let values: [JsonValue; 2] = [value.into(), format!("{}", ByteSize(value)).into()];

                json_values[name] = values.as_ref().into();
            }

            writeln!(file, "{}", json_values);
        }
    });
}
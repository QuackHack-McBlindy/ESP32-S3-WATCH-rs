// APPLICATIONS/TINYWEATHER
// FETCHES CURRENT WEATHER + 3-DAY FORECAST FROM WTTR.IN (JSON API)
// STORES RESULT IN GLOBAL WEATHER STATE FOR THE GUI

extern crate alloc;

// ───────────────────────────────────────────────────────────────────────
// GLOBAL WEATHER STATE

#[derive(Clone)]
pub struct WeatherData {
    pub current_temp: i32,
    pub current_code: heapless::String<4>,
    pub current_desc: heapless::String<64>,
    pub days: [core::option::Option<DayWeather>; 3],
}

#[derive(Clone)]
pub struct DayWeather {
    pub date: heapless::String<16>,
    pub mintemp: i32,
    pub maxtemp: i32,
    pub code: heapless::String<4>,
    pub desc: heapless::String<64>,
}

pub static WEATHER: embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    core::option::Option<WeatherData>,
> = embassy_sync::mutex::Mutex::new(core::option::Option::None);

pub static WEATHER_DAY: embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    usize,
> = embassy_sync::mutex::Mutex::new(0);

// ───────────────────────────────────────────────────────────────────────
// DESCRIBE THIS APPLICATION
pub const APP_DESCRIPTOR: crate::applications::AppDescriptor = crate::applications::AppDescriptor {
    name: "tinyWeather",
    description: "Fetch and display current weather + 3-day forcast. No key/location required.",
    launch: open_app,
    icon: crate::base::assets::SUNNY_PNG,
};

pub fn open_app() {
    defmt::info!("Opening tinyWeather app");
    crate::store!(crate::gui::pages::CURRENT_PAGE, 3);
}

// ───────────────────────────────────────────────────────────────────────
// FETCH AND STORE WEATHER (CURRENT + 3‑DAY FORECAST)
pub async fn get_current_weather(
    stack: embassy_net::Stack<'_>,
) -> core::option::Option<()> {
    defmt::info!("Fetching weather...");

    let mut buf = alloc::vec![0u8; 40960];

    let tinyapi::HttpResponse { status, body } = match tinyapi::http_get(
        stack,
        "http://5.9.243.187/?format=j1",
        &mut buf,
    )
    .await
    {
        core::result::Result::Ok(resp) => {
            defmt::debug!("HTTP GET succeeded, status = {}", resp.status);
            defmt::debug!("Response body length = {} bytes", resp.body.len());
            resp
        }
        core::result::Result::Err(_) => {
            defmt::error!("HTTP GET request failed");
            return core::option::Option::None;
        }
    };

    if status != 200 {
        defmt::error!("HTTP status not 200 (got {}), aborting", status);
        return core::option::Option::None;
    }

    let weather_data = parse_wttr_json(body)?;
    defmt::debug!("Weather parsed successfully, storing...");

    *WEATHER.lock().await = core::option::Option::Some(weather_data);
    core::option::Option::Some(())
}

// ───────────────────────────────────────────────────────────────────────
// JSON PARSER – RETURNS COMPLETE WeatherData
fn parse_wttr_json(json_bytes: &[u8]) -> core::option::Option<WeatherData> {
    // CURRENT CONDITION
    let temp_c = extract_json_string_value(json_bytes, "temp_C")?;
    let temperature: i32 = temp_c.parse().ok()?;

    let weather_code = extract_json_string_value(json_bytes, "weatherCode")?;
    let emoji = weather_code_to_emoji(weather_code);
    let cond = extract_first_weather_desc_value(json_bytes)?;

    defmt::info!(
        "Current weather: {} {} {}°C",
        emoji,
        cond.as_str(),
        temperature
    );

    // 3‑DAY FORECAST
    let weather_array = find_json_array(json_bytes, "weather")?;
    let mut remaining = &weather_array[1..]; // SKIP '['

    let mut days: [core::option::Option<DayWeather>; 3] =
        [core::option::Option::None, core::option::Option::None, core::option::Option::None];

    for day_idx in 0..3 {
        // SKIP WHITESPACE / COMMAS
        loop {
            if remaining.is_empty() {
                break;
            }
            let ch = remaining[0];
            if ch.is_ascii_whitespace() || ch == b',' {
                remaining = &remaining[1..];
            } else { break; }
        }

        if remaining.is_empty() || remaining[0] == b']' {
            defmt::info!("Day {}: no more objects (array ended)", day_idx);
            break;
        }

        if let core::option::Option::Some(day_obj) = extract_json_object(remaining) {
            let date_str = get_string_from_obj(day_obj, "date")?;
            let mut date: heapless::String<16> = heapless::String::new();
            date.push_str(date_str).ok()?;

            let mintemp = get_int_from_obj(day_obj, "mintempC").unwrap_or(i32::MIN);
            let maxtemp = get_int_from_obj(day_obj, "maxtempC").unwrap_or(i32::MIN);

            let hour = find_array_object_with_value(day_obj, "hourly", "time", "1200")
                .or_else(|| find_first_array_object(day_obj, "hourly"));

            if let core::option::Option::Some(hour) = hour {
                let code_str = get_string_from_obj(hour, "weatherCode").unwrap_or("?");
                let desc = get_first_weather_desc_value_from_obj(hour)
                    .unwrap_or_else(|| {
                        let mut s = heapless::String::<64>::new();
                        s.push_str("?").ok();
                        s
                    });
                let day_emoji = weather_code_to_emoji(code_str);

                let mut code: heapless::String<4> = heapless::String::new();
                code.push_str(code_str).ok()?;
                let mut desc_str: heapless::String<64> = heapless::String::new();
                desc_str.push_str(&desc).ok()?;

                defmt::info!(
                    "Day {}: {} {}-{}°C {} {}",
                    day_idx,
                    date,
                    mintemp,
                    maxtemp,
                    day_emoji,
                    desc.as_str(),
                );

                days[day_idx] = core::option::Option::Some(DayWeather {
                    date,
                    mintemp,
                    maxtemp,
                    code,
                    desc: desc_str,
                });
            } else { defmt::info!("Day {}: no noon hour data", day_idx); }

            remaining = &remaining[day_obj.len()..];
        } else {
            defmt::info!("Day {}: failed to extract JSON object", day_idx);
            break;
        }
    }

    let mut current_code: heapless::String<4> = heapless::String::new();
    current_code.push_str(weather_code).ok()?;
    let mut current_desc: heapless::String<64> = heapless::String::new();
    current_desc.push_str(&cond).ok()?;

    core::option::Option::Some(WeatherData {
        current_temp: temperature,
        current_code,
        current_desc,
        days,
    })
}

// ───────────────────────────────────────────────────────────────────────
// JSON SCANNING HELPERS
fn find_json_key_start(json: &[u8], key: &str) -> core::option::Option<usize> {
    let key_bytes = key.as_bytes();
    let len = json.len();
    let mut i = 0;
    while i < len {
        if json[i] == b'"' {
            i += 1;
            if i + key_bytes.len() <= len && &json[i..i + key_bytes.len()] == key_bytes {
                let after = i + key_bytes.len();
                if after < len && json[after] == b'"' {
                    i = after + 1;
                    while i < len && json[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if i < len && json[i] == b':' {
                        return core::option::Option::Some(i + 1);
                    }
                }
            }
        }
        i += 1;
    }
    core::option::Option::None
}

fn extract_json_string_value<'a>(
    json: &'a [u8],
    key: &str,
) -> core::option::Option<&'a str> {
    let after_colon = find_json_key_start(json, key)?;
    let mut pos = after_colon;
    while pos < json.len() && json[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= json.len() || json[pos] != b'"' {
        return core::option::Option::None;
    }
    pos += 1;
    let start = pos;
    while pos < json.len() && json[pos] != b'"' {
        pos += 1;
    }
    if pos >= json.len() {
        return core::option::Option::None;
    }
    core::str::from_utf8(&json[start..pos]).ok()
}

fn extract_first_weather_desc_value(
    json: &[u8],
) -> core::option::Option<heapless::String<64>> {
    let after_wd = find_json_key_start(json, "weatherDesc")?;
    let after_value = find_json_key_start(&json[after_wd..], "value")?;
    let after_value = after_wd + after_value;
    let mut pos = after_value;
    while pos < json.len() && json[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= json.len() || json[pos] != b'"' {
        return core::option::Option::None;
    }
    pos += 1;
    let start = pos;
    while pos < json.len() && json[pos] != b'"' {
        pos += 1;
    }
    if pos >= json.len() {
        return core::option::Option::None;
    }
    let desc_str = core::str::from_utf8(&json[start..pos]).ok()?;
    let mut desc = heapless::String::<64>::new();
    desc.push_str(desc_str).ok()?;
    core::option::Option::Some(desc)
}

fn find_json_array<'a>(
    json: &'a [u8],
    key: &str,
) -> core::option::Option<&'a [u8]> {
    let after_key = find_json_key_start(json, key)?;
    let mut pos = after_key;
    while pos < json.len() && json[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= json.len() || json[pos] != b'[' {
        return core::option::Option::None;
    }
    core::option::Option::Some(&json[pos..])
}

fn extract_json_object(slice: &[u8]) -> core::option::Option<&[u8]> {
    let mut pos = 0;
    while pos < slice.len() && slice[pos].is_ascii_whitespace() {
        pos += 1;
    }
    if pos >= slice.len() || slice[pos] != b'{' {
        return core::option::Option::None;
    }
    let start = pos;
    pos += 1;
    let mut depth = 1;
    while pos < slice.len() && depth > 0 {
        match slice[pos] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        pos += 1;
    }
    if depth == 0 { core::option::Option::Some(&slice[start..pos]) }
    else { core::option::Option::None }
}

fn get_int_from_obj(obj: &[u8], key: &str) -> core::option::Option<i32> {
    let s = extract_json_string_value(obj, key)?;
    s.parse().ok()
}

fn get_string_from_obj<'a>(
    obj: &'a [u8],
    key: &str,
) -> core::option::Option<&'a str> {
    extract_json_string_value(obj, key)
}

fn find_array_object_with_value<'a>(
    obj: &'a [u8],
    array_key: &str,
    match_key: &str,
    match_val: &str,
) -> core::option::Option<&'a [u8]> {
    let array_slice = find_json_array(obj, array_key)?;
    let mut remaining = &array_slice[1..]; // SKIP '['
    loop {
        while !remaining.is_empty()
            && (remaining[0].is_ascii_whitespace() || remaining[0] == b',')
        {
            remaining = &remaining[1..];
        }
        if remaining.is_empty() || remaining[0] == b']' {
            return core::option::Option::None;
        }
        if let core::option::Option::Some(elem) = extract_json_object(remaining) {
            let found = get_string_from_obj(elem, match_key)
                .map(|v| v == match_val)
                .unwrap_or(false);
            if found { return core::option::Option::Some(elem); }
            remaining = &remaining[elem.len()..];
        } else { break; }
    }
    core::option::Option::None
}

fn find_first_array_object<'a>(
    obj: &'a [u8],
    array_key: &str,
) -> core::option::Option<&'a [u8]> {
    let array_slice = find_json_array(obj, array_key)?;
    let after_bracket = &array_slice[1..]; // SKIP '['
    let mut pos = 0;
    while pos < after_bracket.len() && after_bracket[pos].is_ascii_whitespace() {
        pos += 1;
    }
    extract_json_object(&after_bracket[pos..])
}

fn get_first_weather_desc_value_from_obj(
    obj: &[u8],
) -> core::option::Option<heapless::String<64>> {
    let wd_array = find_json_array(obj, "weatherDesc")?;
    let first_obj = extract_json_object(&wd_array[1..])?; // SKIP '['
    let val = extract_json_string_value(first_obj, "value")?;
    let mut desc = heapless::String::<64>::new();
    desc.push_str(val).ok()?;
    core::option::Option::Some(desc)
}

// ───────────────────────────────────────────────────────────────────────
// WEATHER CODE TO EMOJI MAPPING
fn weather_code_to_emoji(code: &str) -> &str {
    match code {
        "113" => "☀️",
        "116" => "⛅",
        "119" => "☁️",
        "122" => "☁️",
        "143" => "☁️",
        "176" => "🌧️",
        "179" => "🌧️",
        "182" => "🌧️",
        "185" => "🌧️",
        "200" => "⛈️",
        "227" => "🌨️",
        "230" => "🌨️",
        "248" => "☁️",
        "260" => "☁️",
        "263" => "🌧️",
        "266" => "🌧️",
        "281" => "🌧️",
        "284" => "🌧️",
        "293" => "🌧️",
        "296" => "🌧️",
        "299" => "🌧️",
        "302" => "🌧️",
        "305" => "🌧️",
        "308" => "🌧️",
        "311" => "🌧️",
        "314" => "🌧️",
        "317" => "🌧️",
        "320" => "🌨️",
        "323" => "🌨️",
        "326" => "🌨️",
        "329" => "❄️",
        "332" => "❄️",
        "335" => "❄️",
        "338" => "❄️",
        "350" => "🌧️",
        "353" => "🌧️",
        "356" => "🌧️",
        "359" => "🌧️",
        "362" => "🌧️",
        "365" => "🌧️",
        "368" => "🌧️",
        "371" => "❄️",
        "374" => "🌨️",
        "377" => "🌨️",
        "386" => "🌨️",
        "389" => "🌨️",
        "392" => "🌧️",
        "395" => "❄️",
        _ => "❓",
    }
}

// ───────────────────────────────────────────────────────────────────────
// WEATHER CODE TO PNG MAP
pub fn weather_png(code: &str) -> core::option::Option<&'static [u8]> {
    match code {
        // SUNNY
        "113" => core::option::Option::Some(crate::base::assets::SUNNY_PNG),
        // PARTLY CLOUDY
        "116" => core::option::Option::Some(crate::base::assets::PARTLY_CLOUDY_PNG),
        // CLOUDY
        "119" | "122" | "143" | "248" | "260" => {
            core::option::Option::Some(crate::base::assets::CLOUDY_PNG)
        }
        // RAIN
        "176" | "179" | "182" | "185" | "263" | "266" | "281" | "284" | "293"
        | "296" | "299" | "302" | "305" | "308" | "311" | "314" | "317" | "350"
        | "353" | "356" | "359" | "362" | "365" | "368" | "392" => {
            core::option::Option::Some(crate::base::assets::RAIN_PNG)
        }
        // THUNDERSTORM
        "200" => core::option::Option::Some(crate::base::assets::THUNDERSTORM_PNG),
        // SLEET
        "227" | "230" | "320" | "323" | "326" | "374" | "377" | "386" | "389" => {
            core::option::Option::Some(crate::base::assets::SLEET_PNG)
        }
        // SNOW
        "329" | "332" | "335" | "338" | "371" | "395" => {
            core::option::Option::Some(crate::base::assets::SNOW_PNG)
        }
        // FALLBACK
        _ => core::option::Option::Some(crate::base::assets::UNKNOWN_WEATHER_PNG),
    }
}


// ───────────────────────────────────────────────────────────────────────
// WEATHER TASK THAT FETCH NEW WEATHER DATA ON REQUEST
use core::sync::atomic::{AtomicBool, Ordering};
pub static WEATHER_CMD: embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, (), 1> = embassy_sync::channel::Channel::new();

static FETCHING: AtomicBool = AtomicBool::new(false);

#[embassy_executor::task]
pub async fn weather_task(stack: embassy_net::Stack<'static>) {
    loop {
        WEATHER_CMD.receive().await;
        get_current_weather(stack).await;
        crate::delay_s!(3);
        crate::dirty!();
        crate::delay_s!(45);
        FETCHING.store(false, Ordering::Release);
    }
}

// ───────────────────────────────────────────────────────────────────────
// PUBLIC ZERO ARGS REFRESH FUNCTION
// ASYNC
pub async fn update() {
    WEATHER_CMD.send(()).await;
}
// NOT ASYNC!
pub fn update_now() {
    if FETCHING.swap(true, Ordering::AcqRel) == false {
        if WEATHER_CMD.try_send(()).is_err() { FETCHING.store(false, Ordering::Release); }
    }
}


// BASE/ASSETS
// FILE DEFINITIONS GOT IT'S OWN MODULE
// EMBEDDED **ONCE** - BYTES STORED AS STATIC CONST [u8]
// SHARED ACROSS MODULES - TO KEEP FLASH FOOTPRINT MINIMAL


// ───────────────────────────────────────────────────────────────────────
// FONTS
pub const ROBOTO_BOLD: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Roboto-Bold.ttf"
));

pub const ROBOTO_REGULAR: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/Roboto-Regular.ttf"
));


// ───────────────────────────────────────────────────────────────────────
// PNG IMAGES

// WEATHER ICONS
pub const SUNNY_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));

pub const PARTLY_CLOUDY_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));

pub const CLOUDY_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));

pub const RAIN_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));

pub const THUNDERSTORM_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));

pub const SLEET_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));

pub const FOG_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));

pub const SNOW_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/snow.png"
));



// SETTINGS ICONS
pub const SETTINGS_ALERT_TRIANGLE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/alert-triangle.png"
));

pub const SETTINGS_AMP_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/amp.png"
));

pub const SETTINGS_BATTERY_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/battery.png"
));

pub const SETTINGS_BATTERY_CHARGING_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/battery-charging.png"
));

pub const SETTINGS_BAR_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/bar-chart.png"
));

pub const SETTINGS_GITHUB_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/github.png"
));

pub const SETTINGS_ARROW_DOWN_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/arrow-down.png"
));

pub const SETTINGS_BLUETOOTH_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/bluetooth.png"
));

pub const SETTINGS_CAST_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/cast.png"
));

pub const SETTINGS_CPU_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/cpu.png"
));

pub const SETTINGS_DISPLAY_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/display.png"
));

pub const SETTINGS_WAKE_WORD_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/wakeword.png"
));

pub const SETTINGS_INFO_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/info.png"
));

pub const SETTINGS_DELETE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/delete.png"
));

pub const SETTINGS_API_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/api.png"
));

pub const SETTINGS_MIC_ON_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/mic-on.png"
));

pub const SETTINGS_MIC_OFF_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/mic-off.png"
));


pub const SETTINGS_SETTINGS_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/settings.png"
));

pub const SETTINGS_SLASH_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/slash.png"
));

pub const SETTINGS_VOLUME_0_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/volume-0.png"
));

pub const SETTINGS_VOLUME_1_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/volume-1.png"
));

pub const SETTINGS_VOLUME_2_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/volume-2.png"
));

pub const SETTINGS_SPEAKER_OFF_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/speaker-off.png"
));

pub const SETTINGS_BRIGHTNESS_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/brightness.png"
));

pub const SETTINGS_TERMINAL_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/terminal.png"
));

pub const SETTINGS_TOOL_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/tool.png"
));


pub const SETTINGS_WATCH_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/watch.png"
));

pub const SETTINGS_WIFI_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/wifi-on.png"
));

pub const SETTINGS_WIFI_OFF_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/wifi-off.png"
));


// TASKS ICONS



pub const BOLT_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/bolt.png"
));

// APPLICATION ICONS
pub const SETTINGS_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings.png"
));

pub const DUCKCLOUD_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/duckcloud.png"
));

pub const DUCK_TV_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/duck-tv.png"
));

pub const HOUSE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/house.png"
));

pub const QWACKIFY_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/qwackify.png"
));

// IN-APP PNG IMAGES
pub const SETTINGS_WIFI_ON_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/settings/wifi-on.png"
));

pub const HOUSE_LIGHTS_ON_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/house/lights_on.png"
));

pub const MEDIA_NEXT_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/media_player/next.png"
));

pub const MEDIA_PAUSE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/media_player/pause.png"
));

pub const MEDIA_PLAY_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/media_player/play.png"
));

pub const MEDIA_PREVIOUS_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/media_player/previous.png"
));

pub const MEDIA_HEART_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/media_player/heart.png"
));

pub const MEDIA_CLEAR_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/media_player/clear.png"
));

pub const CALL_ACCEPT_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/call/call_accept.png"
));

pub const CALL_DECLINE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/call/call_decline.png"
));

pub const WEATHER_DRIZZLE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/tinyweather/cloud-drizzle.png"
));


// ───────────────────────────────────────────────────────────────────────
// SOUNDS
pub const DING_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/sound/ding.wav"
));

pub const DONE_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/sound/done.wav"
));

pub const FAIL_WAV: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/sound/fail.wav"
));

//pub const BOOT_WAV: &[u8] = include_bytes!(concat!(
//    env!("CARGO_MANIFEST_DIR"),
//    "/assets/sound/boot.wav"
//));

//pub const NOTIFICATION_WAV: &[u8] = include_bytes!(concat!(
//    env!("CARGO_MANIFEST_DIR"),
//    "/assets/sound/notification.wav"
//));

//pub const WARNING_WAV: &[u8] = include_bytes!(concat!(
//    env!("CARGO_MANIFEST_DIR"),
//    "/assets/sound/warning.wav"
//));

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

// SETTINGS ICONS
pub const SETTINGS_ALERT_TRIANGLE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/alert-triangle.png"
));

pub const SETTINGS_BLUETOOTH_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/bluetooth.png"
));

pub const SETTINGS_CAST_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/cast.png"
));

pub const SETTINGS_DELETE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/delete.png"
));

pub const SETTINGS_GLOBE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/globe.png"
));

pub const SETTINGS_MESSAGE_CIRCLE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/message-circle.png"
));

pub const SETTINGS_MIC_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/mic.png"
));

pub const SETTINGS_MIC_OFF_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/mic-off.png"
));

pub const SETTINGS_SETTINGS_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/settings.png"
));

pub const SETTINGS_SLASH_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/slash.png"
));

pub const SETTINGS_SPEAKER_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/speaker.png"
));

pub const SETTINGS_SUN_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/sun.png"
));

pub const SETTINGS_TERMINAL_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/terminal.png"
));

pub const SETTINGS_TOGGLE_LEFT_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/toggle-left.png"
));

pub const SETTINGS_TOGGLE_RIGHT_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/toggle-right.png"
));

pub const SETTINGS_TOOL_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/tool.png"
));

pub const SETTINGS_VOLUME_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/volume.png"
));

pub const SETTINGS_VOLUME_1_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/volume-1.png"
));

pub const SETTINGS_VOLUME_2_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/volume-2.png"
));

pub const SETTINGS_VOLUME_X_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/volume-x.png"
));

pub const SETTINGS_WATCH_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/watch.png"
));

pub const SETTINGS_WIFI_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/wifi.png"
));

pub const SETTINGS_WIFI_OFF_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/icons/wifi-off.png"
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

pub const APP3_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/apps/app3.png"
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

pub const CALL_ACCEPT_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/call_accept.png"
));

pub const CALL_DECLINE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/call_decline.png"
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

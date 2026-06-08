// BASE/MACROS
// SIMPLE SHORTHAND HELPERS


//unsafe fn flush_cache() {
//    unsafe extern "C" {
//        fn Cache_Invalidate_DCache_All();
//    }
//    unsafe { Cache_Invalidate_DCache_All() };
//}

// ───────────────────────────────────────────────────────────────────────
// DISPLAY RELATED

// DIRTY!
// CALL WHEN A VISIBLE VALUE CHANGES AND A DISPLAY REDRAW IS NEEDED.
// USAGE: `dirty!();`
#[macro_export]
macro_rules! dirty {
    () => {
        crate::state::DISPLAY_DIRTY.store(true, core::sync::atomic::Ordering::Release);
    };
}

// IS_DIRTY!
// CHECK IF A DISPLAY REDRAW IS NEEDED - AND RESET THE FLAG
// RETURNS `true` IF REDRAW WAS REQUESTED SINCE LAST CHECK.
// USAGE: `if is_dirty!() { … }`
#[macro_export]
macro_rules! is_dirty {
    () => {
        crate::state::DISPLAY_DIRTY.swap(false, core::sync::atomic::Ordering::Acquire)
    };
}

#[macro_export]
macro_rules! dirty_loop_on {
    () => {
        crate::state::DISPLAY_LOOP_DIRTY.store(true, core::sync::atomic::Ordering::Relaxed);
        defmt::info!("DIRTY LOOPING!");
    };
}

#[macro_export]
macro_rules! dirty_loop_off {
    () => {
        defmt::info!("NOO MORE DIRTY LOOPIN'");
        crate::state::DISPLAY_LOOP_DIRTY.store(false, core::sync::atomic::Ordering::Relaxed);
    };
}


// ───────────────────────────────────────────────────────────────────────
// DELAY RELATED

// WAIT_MS (BLOCKING)
// USAGE:
// wait_ms!(100);
#[macro_export]
macro_rules! wait_ms {
    ($ms:expr) => {
        embassy_time::block_for(embassy_time::Duration::from_millis($ms))
    };
}

// WAIT_S (BLOCKING)
// USAGE:
// wait_s!(10);
#[macro_export]
macro_rules! wait_s {
    ($s:expr) => {
        embassy_time::block_for(embassy_time::Duration::from_secs($s))
    };
}

// DELAY_MS
// USAGE:
// delay_ms!(100);
#[macro_export]
macro_rules! delay_ms {
    ($ms:expr) => {
        embassy_time::Timer::after(embassy_time::Duration::from_millis($ms)).await
    };
}

// DELAY_S
// USAGE:
// delay_s!(10);
#[macro_export]
macro_rules! delay_s {
    ($s:expr) => {
        embassy_time::Timer::after(embassy_time::Duration::from_secs($s)).await
    };
}


// ───────────────────────────────────────────────────────────────────────
// ATOMIC VARIABLES RELATED

// INIT_BOOL
// USAGE:
// init_bool!(MIC_MUTED, false);
#[macro_export]
macro_rules! init_bool {
    ($name:ident, $val:expr) => {
        pub static $name: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new($val);
    };
}

// TOGGLE
// USAGE:
// toggle!(STATE);
#[macro_export]
macro_rules! toggle {
    ($var:expr) => {{
        let prev = $var.fetch_xor(true, ::core::sync::atomic::Ordering::Relaxed);
        let new = !prev;
        defmt::debug!("toggled {} to {}", stringify!($var), new);
        new
    }};
}

// SWAP!
// AUTOMATICALLY TOGGLE AN AtomicBool
// RETURNS THE VALUE **BEFORE** THE SWAP.
// USAGE:
//   let was_on = swap!(POWER_STATE);
#[macro_export]
macro_rules! swap {
    ($var:expr) => {{
        $var.fetch_xor(true, ::core::sync::atomic::Ordering::Relaxed)
    }};
}


// INIT_u8
// USAGE:
// init_u8!(MIC_VOLUME, 72);
#[macro_export]
macro_rules! init_u8 {
    ($name:ident, $val:expr) => {
        pub static $name: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new($val);
    };
}

// INIT_u16
// USAGE:
// init_u16!(MIC_VOLUME, 72);
#[macro_export]
macro_rules! init_u16 {
    ($name:ident, $val:expr) => {
        pub static $name: core::sync::atomic::AtomicU16 = core::sync::atomic::AtomicU16::new($val);
    };
}


// INIT_U32
// USAGE:
// init_u32!(BATTERY_VOLTAGE, 0);
#[macro_export]
macro_rules! init_u32 {
    ($name:ident, $val:expr) => {
        pub static $name: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new($val);
    };
}


// INIT_I8
// USAGE:
// init_i8!(SOME_SIGNED_VALUE, -10);
#[macro_export]
macro_rules! init_i8 {
    ($name:ident, $val:expr) => {
        pub static $name: core::sync::atomic::AtomicI8 = core::sync::atomic::AtomicI8::new($val);
    };
}

// INIT_I32
// USAGE:
// init_i32!(RSSI, 0);
#[macro_export]
macro_rules! init_i32 {
    ($name:ident, $val:expr) => {
        pub static $name: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new($val);
    };
}

// STORE ATOMIC VARIABLES
// USAGE:
// store!(PRESENCE, current);
// store!(TEMPERATURE, temp as u8);
#[macro_export]
macro_rules! store {
    ($var:expr, $value:expr) => {
        $var.store($value, core::sync::atomic::Ordering::Relaxed)
    };
}

// LOAD ATOMIC VARIABLES
// USAGE:
// info!("{}", load!(TEMPERATURE));
#[macro_export]
macro_rules! load {
    ($var:expr) => {
        $var.load(core::sync::atomic::Ordering::Relaxed)
    };
}


// ───────────────────────────────────────────────────────────────────────
// TASK SPAWNER
// USAGE: 
// spawn!(spawner, task_name());
#[macro_export]
macro_rules! spawn {
    ($spawner:expr, $task:expr) => {{
        match $task {
            Ok(token) => $spawner.spawn(token),
            Err(e) => ::defmt::error!("Failed to spawn task: {:?}", e),
        }
    }};
}


// ───────────────────────────────────────────────────────────────────────
// MK_STATIC
#[macro_export]
macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write($val);
        x
    }};
}


// STATIC_MUTEX
#[macro_export]
macro_rules! static_mutex {
    ($mutex_type:ty, $value:expr) => {{
        let value = $value;
        let mutex = Box::leak(Box::new(<$mutex_type>::new(value)));
        mutex
    }};
}


// ───────────────────────────────────────────────────────────────────────
// ENV_DEF
#[macro_export]
macro_rules! env_def {
    ($name:expr, $default:expr) => {
        match option_env!($name) {
            Some(val) => val,
            None => $default,
        }
    };
}


#[macro_export]
macro_rules! gpio_input {
    ($pin:expr, $pull:expr) => {{
        use esp_hal::gpio::{Input, InputConfig, Pull};
        Input::new($pin, InputConfig::default().with_pull($pull))
    }};
}


#[macro_export]
macro_rules! gpio_output {
    ($pin:expr, $initial_level:expr) => {{
        use esp_hal::gpio::{Output, OutputConfig, Level};
        Output::new($pin, $initial_level, OutputConfig::default())
    }};
}


//macro_rules! display_brightness {
//    ($channel:expr, $percent:expr) => {{
//        let percent = $percent.clamp(0, 100);
//        $channel.set_duty_percent(percent).unwrap();
//    }};
//}


// ───────────────────────────────────────────────────────────────────────
// SCAN I2C BUS
//defmt::info!("Scanning I2C bus on GPIO15(SDA)/GPIO14(SCL)");
//for addr in 0x08..=0x7F {
//    let result = critical_section::with(|cs| {
//        let mut i2c = i2c_a_mutex.borrow(cs).borrow_mut();
//        i2c.write(addr, &[])
//    });
//    if result.is_ok() {
//        defmt::info!("Found device at address 0x{:02X}", addr);
//    }
//}
//defmt::info!("Scan complete");


